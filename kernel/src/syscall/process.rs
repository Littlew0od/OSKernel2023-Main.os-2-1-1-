use crate::config::MMAP_BASE;
use crate::config::PAGE_SIZE;
use crate::fs::OpenFlags;
//open_file
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::sbi::shutdown;
use crate::task::current_task;
use crate::task::{
    current_process, current_user_token, exit_current_and_run_next, pid2process,
    suspend_current_and_run_next, SignalAction, SignalFlags, MAX_SIG, SIG_BLOCK, SIG_SETMASK,
    SIG_UNBLOCK,
};
use crate::timer::{get_time_ns, get_time_sec, get_time_us};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::task;
use alloc::vec::Vec;

use super::errno::EPERM;
use super::errno::SUCCESS;

pub fn sys_shutdown(failure: bool) -> ! {
    shutdown(failure);
}

pub fn sys_exit(exit_code: i32) -> ! {
    // the lower 8 bits of return value is for return in function
    // the lower 9-16 bits is for the return value in the system
    exit_current_and_run_next((exit_code & 0xff) << 8);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

///fake
pub fn sys_get_process_time(times: *mut u64) -> isize {
    let token = current_user_token();
    let usec = get_time_us() as u64;

    *translated_refmut(token, times) = usec;
    *translated_refmut(token, unsafe { times.add(1) }) = usec;
    *translated_refmut(token, unsafe { times.add(2) }) = usec;
    *translated_refmut(token, unsafe { times.add(3) }) = usec;

    usec as isize
}

pub fn sys_get_time(time_return: *mut u64) -> isize {
    let token = current_user_token();
    if time_return as usize != 0 {
        *translated_refmut(token, time_return) = get_time_sec() as u64;
        *translated_refmut(token, unsafe { time_return.add(1) }) = get_time_ns() as u64;
    }
    0
}

pub fn sys_getpid() -> isize {
    current_process().getpid() as isize
}

pub fn sys_getppid() -> isize {
    current_process()
        .inner_exclusive_access()
        .parent
        .as_ref()
        .unwrap()
        .upgrade()
        .unwrap()
        .getpid() as isize
}

pub fn sys_fork(flags: usize, stack_ptr: usize, ptid: usize, tls: usize, ctid: usize) -> isize {
    let current_process = current_process();
    let new_process = current_process.fork();
    let new_pid = new_process.getpid();
    // modify trap context of new_task, because it returns immediately after switching
    let new_process_inner = new_process.inner_exclusive_access();
    let task = new_process_inner.tasks[0].as_ref().unwrap();
    let trap_cx = task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    if stack_ptr != 0 {
        trap_cx.x[2] = stack_ptr;
    }
    trap_cx.x[10] = 0;
    new_pid as isize
}

pub fn sys_execve(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    if args as usize != 0 {
        loop {
            let arg_str_ptr = *translated_ref(token, args);
            if arg_str_ptr == 0 {
                break;
            }
            args_vec.push(translated_str(token, arg_str_ptr as *const u8));
            unsafe {
                args = args.add(1);
            }
        }
    }
    let process = current_process();
    let working_inode = process
        .inner_exclusive_access()
        .work_path
        .lock()
        .working_inode
        .clone();
    match working_inode.open(&path, OpenFlags::O_RDONLY, false) {
        Ok(file) => {
            let all_data = file.read_all();
            let argc = args_vec.len();
            process.exec(all_data.as_slice(), args_vec);
            // return argc because cx.x[10] will be covered with it later
            argc as isize
        }
        Err(errno) => errno,
    }
}

pub fn sys_brk(addr: usize) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if addr == 0 {
        inner.heap_end.0 as isize
    } else if addr < inner.heap_base.0 {
        -1
    } else {
        // We need to calculate to determine if we need a new page table
        // current end page address
        let align_addr = ((addr) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        // the end of 'addr' value
        let align_end = ((inner.heap_end.0) + PAGE_SIZE) & (!(PAGE_SIZE - 1));
        if align_end > addr {
            inner.heap_end = addr.into();
            align_addr as isize
        } else {
            let heap_end = inner.heap_end;
            // map heap
            inner.memory_set.map_heap(heap_end, align_addr.into());
            inner.heap_end = align_end.into();
            addr as isize
        }
    }
}

bitflags! {
    struct WaitOption: u32 {
        const WNOHANG    = 1;
        const WSTOPPED   = 2;
        const WEXITED    = 4;
        const WCONTINUED = 8;
        const WNOWAIT    = 0x1000000;
    }
}
/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
/// We use loop to ensure that the corresponding process has ended
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    loop {
        let process = current_process();
        // find a child process

        let mut inner = process.inner_exclusive_access();
        if !inner
            .children
            .iter()
            .any(|p| pid == -1 || pid as usize == p.getpid())
        {
            return -1;
            // ---- release current PCB
        }
        let pair = inner.children.iter().enumerate().find(|(_, p)| {
            // ++++ temporarily access child PCB exclusively
            p.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == p.getpid())
            // ++++ release child PCB
        });
        if let Some((idx, _)) = pair {
            let child = inner.children.remove(idx);
            // confirm that child will be deallocated after being removed from children list
            assert_eq!(Arc::strong_count(&child), 1);
            let found_pid = child.getpid();
            // ++++ temporarily access child PCB exclusively
            let exit_code = child.inner_exclusive_access().exit_code;
            // ++++ release child PCB
            if !exit_code_ptr.is_null() {
                *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
            }
            return found_pid as isize;
        } else {
            // drop ProcessControlBlock and ProcessControlBlock to avoid mulit-use
            drop(inner);
            drop(process);
            suspend_current_and_run_next();
        }
    }
    // ---- release current PCB automatically
}

pub fn sys_kill(pid: usize, signal: u32) -> isize {
    if let Some(process) = pid2process(pid) {
        if let Some(flag) = SignalFlags::from_bits(signal) {
            process.inner_exclusive_access().signals |= flag;
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_mmap(
    start: usize,
    len: usize,
    prot: u32,
    flags: u32,
    fd: usize,
    offset: usize,
) -> isize {
    if start as isize == -1 || len == 0 {
        return EPERM;
    }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    inner.mmap(start, len, prot, flags, fd, offset)
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    current_process()
        .inner_exclusive_access()
        .munmap(start, len)
}

pub fn sys_sigprocmask(how: usize, set: *mut u32, old_set: *mut u32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let mut mask = inner.signal_mask;

    let token = current_user_token();

    if old_set as usize != 0 {
        *translated_refmut(token, old_set) = mask.bits();
    }
    if set as usize != 0 {
        let set = *translated_ref(token, set);
        let set_flags = SignalFlags::from_bits(set).unwrap();
        match how {
            // SIG_BLOCK The set of blocked signals is the union of the current set and the set argument.
            SIG_BLOCK => mask |= set_flags,
            // SIG_UNBLOCK The signals in set are removed from the current set of blocked signals.
            SIG_UNBLOCK => mask &= !set_flags,
            // SIG_SETMASK The set of blocked signals is set to the argument set.
            SIG_SETMASK => mask = set_flags,
            _ => return EPERM,
        }
        inner.signal_mask = mask;
    }
    SUCCESS
}

pub fn sys_sigreturn() -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.handling_sig = -1;
    // restore the trap context
    let trap_ctx = inner.get_trap_cx();
    *trap_ctx = inner.trap_ctx_backup.unwrap();
    SUCCESS
}

fn check_sigaction_error(signal: SignalFlags, action: usize) -> bool {
    if action == 0 || signal == SignalFlags::SIGKILL || signal == SignalFlags::SIGSTOP {
        true
    } else {
        false
    }
}

pub fn sys_sigaction(
    signum: usize,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    let token = current_user_token();
    let process = current_process();
    let mut inner_process = process.inner_exclusive_access();
    if signum > MAX_SIG {
        return EPERM;
    }
    if old_action as usize != 0 {
        *translated_refmut(token, old_action) = inner_process.signal_actions.table[signum].clone();
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag, action as usize) {
            return EPERM;
        }
        let old_kernel_action = inner_process.signal_actions.table[signum];
        if old_kernel_action.mask != SignalFlags::from_bits(40).unwrap() {
            *translated_refmut(token, old_action) = old_kernel_action;
        } else {
            let mut ref_old_action = *translated_refmut(token, old_action);
            ref_old_action.sa_handler = old_kernel_action.sa_handler;
        }
        let ref_action = translated_ref(token, action);
        inner_process.signal_actions.table[signum as usize] = *ref_action;
        return SUCCESS;
    } else {
        println!("Undefined SignalFlags");
        return EPERM;
    }
}

pub fn sys_set_tid_address(tid_ptr: usize) -> isize {
    let mut task_inner = current_task().unwrap().inner_exclusive_access();
    task_inner.clear_child_tid = tid_ptr;
    task_inner.gettid() as isize
}
