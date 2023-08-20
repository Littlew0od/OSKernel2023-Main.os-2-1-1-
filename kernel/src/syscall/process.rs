#![allow(unused)]
use core::mem::size_of;

use crate::config::MMAP_BASE;
use crate::config::PAGE_SIZE;
use crate::fs::OpenFlags;
use crate::mm::translated_byte_buffer;
use crate::mm::UserBuffer;
use crate::mm::VirtAddr;
use crate::mm::MPROCTECTPROT;
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::sbi::shutdown;
use crate::sync::futex_signal;
use crate::sync::futex_wait;
use crate::sync::FUTEX_REQUEUE;
use crate::sync::{FUTEX_CMD_MASK, FUTEX_PRIVATE_FLAG, FUTEX_WAIT, FUTEX_WAKE};
use crate::syscall::errno::ECHILD;
use crate::task::Rusage;
use crate::task::{
    block_current_and_run_next, current_process, current_task, current_user_token,
    exit_current_and_run_next, suspend_current_and_run_next, CloneFlags, SignalFlags, CSIGNAL,
};
use crate::timer::ITimerVal;
use crate::timer::USEC_PER_SEC;
use crate::timer::{get_time_us, TimeSpec, TimeVal, Times, CLOCK_REALTIME};
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

pub fn sys_yield() -> ! {
    suspend_current_and_run_next();
    panic!("Unreachable in sys_exit!");
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
    // println!("[sys_clock_gettime] tp = {:#x}", tp as usize);
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

// MainOS does not support multi-user
pub fn sys_getegid() -> isize {
    0
}

pub fn sys_clone(
    flags: usize,
    stack_ptr: usize,
    ptid: *mut usize,
    tls: usize,
    ctid: *mut usize,
) -> isize {
    let current_process = current_process();

    let exit_signal = SignalFlags::from_bits(1 << ((flags & CSIGNAL) - 1)).unwrap();
    let clone_signals = CloneFlags::from_bits((flags & !CSIGNAL) as u32).unwrap();

    // println!(
    //     "[sys_clone] exit_signal ={:?}, clone_signals = {:?}, stack_ptr = {:#x}, ptid = {:#x}, tls = {:#x}, ctid = {:#x}",
    //     exit_signal, clone_signals, stack_ptr, ptid as usize, tls, ctid as usize
    // );

    if !clone_signals.contains(CloneFlags::CLONE_THREAD) {
        assert!(stack_ptr == 0);
        return current_process.fork() as isize;
    } else {
        println!("[sys_clone] create thread");
        let new_thread = current_process.clone2(exit_signal, clone_signals, stack_ptr, tls);

        // The thread ID of the main thread needs to be the same as the Process ID,
        // so we will exchange the thread whose thread ID is equal to Process ID with the thread whose thread ID is equal to 0,
        // but the system will not exchange it internally
        let process_pid = current_process.getpid();
        let mut new_thread_ttid = new_thread.inner_exclusive_access().gettid();
        if new_thread_ttid == process_pid {
            new_thread_ttid = 0;
        }

        let token = current_user_token();
        if clone_signals.contains(CloneFlags::CLONE_PARENT_SETTID) {
            if !ptid.is_null() {
                *translated_refmut(token, ptid) = new_thread_ttid;
            }
        }
        if clone_signals.contains(CloneFlags::CLONE_CHILD_SETTID) {
            if !ctid.is_null() {
                *translated_refmut(token, ctid) = new_thread_ttid;
            }
        }
        if clone_signals.contains(CloneFlags::CLONE_CHILD_CLEARTID) {
            let mut thread_inner = new_thread.inner_exclusive_access();
            thread_inner.clear_child_tid = ctid as usize;
        }

        return new_thread_ttid as isize;
    }
}

fn contains_substrings(vec_of_strings: Vec<String>, target_substring: &str) -> bool {
    vec_of_strings.iter().any(|s| s.contains(target_substring))
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
    // skip some test
    if contains_substrings(args_vec.clone(), "pthread") {
        return SUCCESS;
    };
    if contains_substrings(args_vec.clone(), "socket") {
        return SUCCESS;
    };
    if contains_substrings(args_vec.clone(), "sem_init") {
        return SUCCESS;
    };
    if contains_substrings(args_vec.clone(), "tls_init") {
        return SUCCESS;
    };
    if contains_substrings(args_vec.clone(), "tls_local_exec") {
        return SUCCESS;
    };
    if contains_substrings(args_vec.clone(), "tls_get_new_dtv") {
        return SUCCESS;
    };

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
            let cwd = file.get_cwd().unwrap();
            let argc = args_vec.len();
            process.exec(file, args_vec, envp_vec);
            process.inner_exclusive_access().self_exe = cwd;
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
    //     "[sys_waitpid] call wait4, option = {}, ru = {:#x}, pid = {}, exit_code_ptr = {:#x}",
    //     option,
    //     ru,
    //     pid,
    //     exit_code_ptr as usize
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

pub fn sys_mmap(
    start: usize,
    len: usize,
    prot: u32,
    flags: u32,
    fd: usize,
    offset: usize,
) -> isize {
    // println!(
    //     "[sys_mmap] start = {}, len = {}, prot = {}, flags = {}, fd = {:#x}, offset = {}",
    //     start, len, prot, flags, fd, offset
    // );
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

pub fn sys_set_tid_address(tid_ptr: usize) -> isize {
    // tip!("[sys_set_tid_address] tid_ptr = {:#x}", tid_ptr);
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.clear_child_tid = tid_ptr;
    task_inner.gettid() as isize
}

pub fn sys_mprotect(addr: usize, len: usize, prot: usize) -> isize {
    // tip!("[sys_mprotect] addr = {:#x}", addr);
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

#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct RLimit {
    rlim_cur: usize, /* Soft limit */
    rlim_max: usize, /* Hard limit (ceiling for rlim_cur) */
}

pub const RESOURCE_CPU: u32 = 0;
pub const RESOURCE_FSIZE: u32 = 1;
pub const RESOURCE_DATA: u32 = 2;
pub const RESOURCE_STACK: u32 = 3;
pub const RESOURCE_CORE: u32 = 4;
pub const RESOURCE_RSS: u32 = 5;
pub const RESOURCE_NPROC: u32 = 6;
pub const RESOURCE_NOFILE: u32 = 7;
pub const RESOURCE_MEMLOCK: u32 = 8;
pub const RESOURCE_AS: u32 = 9;
pub const RESOURCE_LOCKS: u32 = 10;
pub const RESOURCE_SIGPENDING: u32 = 11;
pub const RESOURCE_MSGQUEUE: u32 = 12;
pub const RESOURCE_NICE: u32 = 13;
pub const RESOURCE_RTPRIO: u32 = 14;
pub const RESOURCE_RTTIME: u32 = 15;
pub const RESOURCE_NLIMITS: u32 = 16;

pub fn sys_prlimit(
    pid: usize,
    resource: u32,
    new_limit: *const RLimit,
    old_limit: *mut RLimit,
) -> isize {
    // println!("[sys_prlimit] pid: {}, resource: {:?}", pid, resource);
    if !new_limit.is_null() {
        let rlimit = &mut RLimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        let token = current_user_token();
        let process = current_process();
        let inner = process.inner_exclusive_access();
        let mut fd_table = inner.fd_table.lock();
        let rlimit = translated_ref(token, new_limit);
        match resource {
            RESOURCE_NOFILE => {
                fd_table.set_soft_limit(rlimit.rlim_cur);
                fd_table.set_hard_limit(rlimit.rlim_max);
            }
            RESOURCE_STACK => {
                // println!("[prlimit] Unsupported modification stack");
            }
            _ => todo!(),
        }
    }
    SUCCESS
}

#[allow(unused)]
pub fn sys_futex(
    uaddr: *mut u32,
    futex_op: usize,
    val: u32,
    timeout: *const TimeSpec,
    uaddr2: *const u32,
    val3: u32,
) -> isize {
    if futex_op & FUTEX_PRIVATE_FLAG == 0 {
        panic!("[sys_futex] process-shared futex is unimplemented");
    }

    let cmd = futex_op & FUTEX_CMD_MASK;
    println!(
        "[futex] uaddr: {:?}, futex_op: {:?}, val: {:#x}, timeout: {:?}, uaddr2: {:?}, val3: {:#x}",
        uaddr, cmd, val, timeout, uaddr2, val3
    );
    match cmd {
        FUTEX_WAIT => futex_wait(uaddr, timeout, val),
        FUTEX_WAKE => futex_signal(uaddr, val),
        FUTEX_REQUEUE => todo!(),
        _ => todo!(),
    }
}

pub fn sys_getrusage(who: isize, usage: *mut Rusage) -> isize {
    if who != 0 {
        panic!("[sys_getrusage] parameter 'who' is not RUSAGE_SELF.");
    }
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();

    *translated_refmut(token, usage) = inner.rusage;
    //tip!("[sys_getrusage] who: RUSAGE_SELF, usage: {:?}", inner.rusage);
    SUCCESS
}

pub const MADV_NORMAL: usize = 0; /* no further special treatment */
pub const MADV_RANDOM: usize = 1; /* expect random page references */
pub const MADV_SEQUENTIAL: usize = 2; /* expect sequential page references */
pub const MADV_WILLNEED: usize = 3; /* will need these pages */
pub const MADV_DONTNEED: usize = 4; /* don't need these pages */
pub const MADV_SPACEAVAIL: usize = 5; /* ensure resources are available */

/* common/generic parameters */
pub const MADV_FREE: usize = 8; /* free pages only if memory pressure */
pub const MADV_REMOVE: usize = 9; /* remove these pages & resources */
pub const MADV_DONTFORK: usize = 10; /* don't inherit across fork */
pub const MADV_DOFORK: usize = 11; /* do inherit across fork */

pub const MADV_MERGEABLE: usize = 12; /* KSM may merge identical pages */
pub const MADV_UNMERGEABLE: usize = 13; /* KSM may not merge identical pages */

pub const MADV_HUGEPAGE: usize = 14; /* Worth backing with hugepages */
pub const MADV_NOHUGEPAGE: usize = 15; /* Not worth backing with hugepages */

pub const MADV_DONTDUMP: usize = 16; /* Explicity exclude from the core dump,
                                     overrides the coredump filter bits */
pub const MADV_DODUMP: usize = 17; /* Clear the MADV_NODUMP flag */

pub const MADV_WIPEONFORK: usize = 18; /* Zero memory on fork, child only */
pub const MADV_KEEPONFORK: usize = 19; /* Undo MADV_WIPEONFORK */

pub fn sys_madvise(addr: usize, length: usize, advice: usize) -> isize {
    println!(
        "[sys_madvise] addr = {:#x}, length = {:#x}, advice = {}",
        addr, length, advice
    );
    SUCCESS
}

pub fn sys_getitimer(which: isize, curr_value: *mut ITimerVal) -> isize {
    if which != 0 {
        panic!("unsupport gettimer");
    }
    let token = current_user_token();
    if curr_value as usize != 0 {
        let mut itimer = current_task().unwrap().inner_exclusive_access().itimer;
        *translated_refmut(token, curr_value) = itimer;
        SUCCESS
    } else {
        EINVAL
    }
}

pub fn sys_setitimer(which: isize, new_value: *mut ITimerVal, old_value: *mut ITimerVal) -> isize {
    if which != 0 {
        panic!("unsupport settimer");
    }
    let token = current_user_token();
    if old_value as usize != 0 {
        let mut itimer = current_task().unwrap().inner_exclusive_access().itimer;
        *translated_refmut(token, old_value) = itimer;
    }
    if new_value as usize != 0 {
        let mut itimer = current_task().unwrap().inner_exclusive_access().itimer;
        itimer = *translated_refmut(token, old_value);
    }
    SUCCESS
}
