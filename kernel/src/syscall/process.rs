use crate::config::PAGE_SIZE;
use crate::fs::OpenFlags;
use crate::mm::VirtAddr;
use crate::mm::MPROCTECTPROT;
//open_file
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::sbi::shutdown;
use crate::syscall::errno::ECHILD;
use crate::task::block_current_and_run_next;
use crate::task::current_task;
use crate::task::{
    current_process, current_user_token, exit_current_and_run_next, pid2process,
    suspend_current_and_run_next, SignalAction, SignalFlags, MAX_SIG, SIG_BLOCK, SIG_SETMASK,
    SIG_UNBLOCK,
};
use crate::timer::get_time_us;
use crate::timer::TimeSpec;
use crate::timer::TimeVal;
use crate::timer::Times;
use crate::timer::CLOCK_REALTIME;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use super::errno::{EINVAL, EPERM, SUCCESS};

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

/// False implementation, but the required struct is ready.
pub fn sys_times(buf: *mut Times) -> isize {
    log!("[sys_times] The return value is not exact!");
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut times = inner.tms;
    let usec = get_time_us();
    times.tms_stime = usec;
    times.tms_utime = usec;
    times.tms_cstime = usec;
    times.tms_cutime = usec;
    *translated_refmut(token, buf) = times;
    SUCCESS
}

pub fn sys_get_time_day(tr: *mut TimeVal) -> isize {
    let token = current_user_token();
    if tr as usize != 0 {
        let timeval = TimeVal::now();
        *translated_refmut(token, tr) = timeval;
    }
    SUCCESS
}

pub fn sys_clock_gettime(clk_id: usize, tp: *mut TimeSpec) -> isize {
    if clk_id == CLOCK_REALTIME {
        if !tp.is_null() {
            let token = current_user_token();
            let timespec = TimeSpec::now();
            *translated_refmut(token, tp) = timespec;
        }
    } else {
        log!("[sys_clock_gettime] Unsupport clock type!");
    }
    SUCCESS
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

// MainOS does not support multi-user
pub fn sys_getuid() -> isize {
    0
}

// MainOS does not support multi-user
pub fn sys_geteuid() -> isize {
    0
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

pub fn sys_execve(path: *const u8, mut args: *const usize, mut envp: *const usize) -> isize {
    let token = current_user_token();
    let mut path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    let mut envp_vec: Vec<String> = Vec::new();
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
    if envp as usize != 0 {
        loop {
            let env_str_ptr = *translated_ref(token, envp);
            if env_str_ptr == 0 {
                break;
            }
            envp_vec.push(translated_str(token, env_str_ptr as *const u8));
            unsafe {
                envp = envp.add(1);
            }
        }
    }
    if path.ends_with(".sh") {
        args_vec.insert(0, String::from("sh"));
        args_vec.insert(0, String::from("/busybox"));
        path = String::from("./busybox");
    }
    // log!(
    //     "[exec] path: {} argv: {:?} /* {} vars */, envp: {:?} /* {} vars */",
    //     path,
    //     args_vec,
    //     args_vec.len(),
    //     envp_vec,
    //     envp_vec.len()
    // );
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
            process.exec(all_data.as_slice(), args_vec, envp_vec);
            // return argc because cx.x[10] will be covered with it later
            argc as isize
        }
        Err(errno) => errno,
    }
}

pub fn sys_brk(addr: usize) -> isize {
    // println!("[sys_brk] addr = {:#x}", addr);
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if addr == 0 {
        inner.heap_end.0 as isize
    } else if addr < inner.heap_base.0 {
        EINVAL
    } else {
        // We need to calculate to determine if we need a new page table
        // current end page address
        let align_addr = ((addr) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        // the end of 'addr' value
        let align_end = ((inner.heap_end.0) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        if align_end >= addr {
            inner.heap_end = addr.into();
            align_addr as isize
        } else {
            let heap_end = inner.heap_end;
            // map heap
            inner.memory_set.map_heap(heap_end, align_addr.into());
            inner.heap_end = align_addr.into();
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
pub fn sys_wait4(pid: isize, exit_code_ptr: *mut i32, option: u32, ru: usize) -> isize {
    // log!(
    //     "[sys_waitpid] call wait4, option = {}, ru = {:#x}, pid = {}",
    //     option,
    //     ru,
    //     pid,
    // );
    let option = WaitOption::from_bits(option).unwrap();
    loop {
        // tip!("[sys_waitpid] wait pid = {}", pid);
        let process = current_process();
        // find a child process

        let mut inner = process.inner_exclusive_access();
        if !inner
            .children
            .iter()
            .any(|p| pid == -1 || pid as usize == p.getpid())
        {
            return ECHILD;
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
            if option.contains(WaitOption::WNOHANG) {
                return SUCCESS;
            } else {
                // suspend_current_and_run_next();
                block_current_and_run_next();
                // log!("[sys_wait4] --resumed--");
            }
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

pub fn sys_sigprocmask(how: usize, set: *mut u32, old_set: *mut u32, kernelSpace: bool) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let mut mask = inner.signal_mask;

    let token = current_user_token();

    if kernelSpace {
        if old_set as usize != 0 {
            unsafe {
                *old_set = mask.bits();
            }
        }
    } else {
        if old_set as usize != 0 {
            *translated_refmut(token, old_set) = mask.bits();
        }
    }

    if set as usize != 0 {
        let set = *translated_ref(token, set);
        let set_flags = SignalFlags::from_bits(set).unwrap();
        // if set_flags.contains(SignalFlags::SIGILL) {
        //     log!("[sys_sigprocmask] SignalFlags::SIGILL");
        // }
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

fn check_sigaction_error(signal: SignalFlags) -> bool {
    if signal == SignalFlags::SIGKILL || signal == SignalFlags::SIGSTOP {
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
    // tip!(
    //     "[sys_sigaction] signum = {:#x}, action = {:X}, old_action = {:X}",
    //     signum,
    //     action as usize,
    //     old_action as usize
    // );
    let token = current_user_token();
    let process = current_process();
    let mut inner_process = process.inner_exclusive_access();
    if signum > MAX_SIG {
        log!("[sys_sigaction] error signum");
        return EPERM;
    }
    if old_action as usize != 0 {
        *translated_refmut(token, old_action) = inner_process.signal_actions.table[signum].clone();
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag) {
            log!("[sys_sigaction] check_sigaction_error");
            return EPERM;
        }
        let old_kernel_action = inner_process.signal_actions.table[signum];
        if old_action as usize != 0 {
            if old_kernel_action.mask != SignalFlags::from_bits(40).unwrap() {
                *translated_refmut(token, old_action) = old_kernel_action;
            } else {
                let mut ref_old_action = *translated_refmut(token, old_action);
                ref_old_action.sa_handler = old_kernel_action.sa_handler;
            }
        }
        if action as usize != 0 {
            let ref_action = translated_ref(token, action);
            inner_process.signal_actions.table[signum as usize] = *ref_action;
        }
        return SUCCESS;
    } else {
        println!("Undefined SignalFlags");
        return EPERM;
    }
}

pub fn sys_set_tid_address(tid_ptr: usize) -> isize {
    // tip!("[sys_set_tid_address] tid_ptr = {:#x}", tid_ptr);
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.clear_child_tid = tid_ptr;
    task_inner.gettid() as isize
}

pub fn sys_mprotect(addr: usize, len: usize, prot: usize) -> isize {
    tip!("[sys_mprotect] addr = {:#x}", addr); 
    if addr == 0 || addr % PAGE_SIZE != 0 {
        EINVAL
    } else {
        current_process()
            .inner_exclusive_access()
            .memory_set
            .mprotect(
                VirtAddr(addr),
                VirtAddr(addr + len),
                MPROCTECTPROT::from_bits(prot as u32).unwrap().into(),
            )
    }
}
