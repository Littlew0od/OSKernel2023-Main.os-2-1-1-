use crate::config::PAGE_SIZE;
use crate::fs::FileDescriptor;
use crate::fs::OpenFlags; //open_file
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::sbi::shutdown;
use crate::task::{
    current_process, current_task, current_user_token, exit_current_and_run_next, pid2process,
    suspend_current_and_run_next, SignalFlags,
};
use crate::timer::{get_time_ns, get_time_sec, get_time_us};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::{debug, error, info, trace, warn};

pub fn sys_shutdown(failure: bool) -> ! {
    shutdown(failure);
}

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
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
    // current_task()
    //     .unwrap()
    //     .process
    //     .upgrade()
    //     .unwrap()
    //     .inner_exclusive_access()
    //     .parent
    //     .as_ref()
    //     .unwrap()
    //     .upgrade()
    //     .unwrap()
    //     .getpid() as isize
    current_process()
        .inner_exclusive_access()
        .parent
        .as_ref()
        .unwrap()
        .upgrade()
        .unwrap()
        .getpid() as isize
}

pub fn sys_clone(
    flags: u32,
    stack: *const u8,
    ptid: *const u32,
    tls: *const usize,
    ctid: *const u32,
) -> isize {
    let current_process = current_process();
    let new_process = current_process.fork();
    let new_pid = new_process.getpid();
    // modify trap context of new_task, because it returns immediately after switching
    let new_process_inner = new_process.inner_exclusive_access();
    let task = new_process_inner.tasks[0].as_ref().unwrap();
    let trap_cx = task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
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
        let align_addr = ((addr) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        let align_end = ((inner.heap_end.0) + PAGE_SIZE) & (!(PAGE_SIZE - 1));
        if align_end > addr {
            inner.heap_end = addr.into();
            align_addr as isize
        } else {
            let heap_end = inner.heap_end;
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
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
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
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
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
    prot: usize,
    flags: usize,
    fd: isize,
    offset: usize,
) -> isize {
    0
}
