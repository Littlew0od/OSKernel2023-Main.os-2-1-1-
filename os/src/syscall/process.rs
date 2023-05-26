use crate::fs::{OpenFlags};//open_file
use crate::fs::FileDescriptor;
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::task::{
    current_process, current_task, current_user_token, exit_current_and_run_next, pid2process,
    suspend_current_and_run_next, SignalFlags,
};
use crate::timer::{get_time_ms, get_time_us, get_time};
use crate::sbi::shutdown;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::{debug, error, info, trace, warn};

pub fn sys_shutdown(failure:bool) -> !{
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
pub fn sys_get_process_time(times: *mut u64) -> isize{
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
        *translated_refmut(token, time_return) = get_time() as u64;
        *translated_refmut(token, unsafe { time_return.add(1) }) = 0;
    }
    0
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().process.upgrade().unwrap().getpid() as isize
}

pub fn sys_getppid() -> isize {
    current_task()
        .unwrap()
        .process
        .upgrade()
        .unwrap()
        .inner_exclusive_access()
        .parent
        .unwrap()
        .upgrade()
        .unwrap()
        .getpid() as isize
}

pub fn sys_fork(
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

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
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
    let proscee = current_process();
    let working_inode = proscee.inner_exclusive_access().work_path.working_inode;
    match working_inode.open(&path, OpenFlags::O_RDONLY, false) {
        Ok(file) => {
            // if file.get_size() < 4 {
            //     return ENOEXEC;
            // }
            // let mut magic_number = Box::<[u8; 4]>::new([0; 4]);
            // this operation may be expensive... I'm not sure
            file.read(Some(&mut 0usize), magic_number.as_mut_slice());
            let elf = match magic_number.as_slice() {
                b"\x7fELF" => file,
                b"#!" => {
                    let shell_file = working_inode
                        .open(DEFAULT_SHELL, OpenFlags::O_RDONLY, false)
                        .unwrap();
                    argv_vec.insert(0, DEFAULT_SHELL.to_string());
                    shell_file
                }
                _ => return ENOEXEC,
            };

            let task = current_task().unwrap();
            show_frame_consumption! {
                "load_elf";
                if let Err(errno) = task.load_elf(elf, &argv_vec, &envp_vec) {
                    return errno;
                };
            }
            // should return 0 in success
            SUCCESS
        }
        Err(errno) => -1,
    }
    // if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
    //     let all_data = app_inode.read_all();
    //     let process = current_process();
    //     let argc = args_vec.len();
    //     process.exec(all_data.as_slice(), args_vec);
    //     // return argc because cx.x[10] will be covered with it later
    //     argc as isize
    // } else {
    //     -1
    // }
}

///fake
pub fn sys_brk(addr: usize) -> isize {
    0
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
