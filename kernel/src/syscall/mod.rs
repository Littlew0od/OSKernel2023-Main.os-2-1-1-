mod config;
pub mod errno;
mod fs;
mod ppoll;
mod process;
mod sync;
mod system;
mod thread;

use crate::{
    task::{SignalAction, SignalFlags},
    timer::TimeSpec,
};
use config::*;
use fs::*;
use ppoll::*;
pub use process::sys_getpid;
use process::*;
use sync::*;
use system::*;
use thread::*;

pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    println!(
        "[kernel] syscall start, syscall_name: {}, syscall_id: {}",
        syscall_name(syscall_id),
        syscall_id,
    );
    let ret = match syscall_id {
        SYSCALL_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_DUP3 => sys_dup3(args[0], args[1], args[2] as u32),
        SYSCALL_FCNTL64 => sys_fcntl(args[0], args[1] as u32, args[2]),
        SYSCALL_MKDIRAT => sys_mkdirat(args[0], args[1] as *const u8, args[2] as u32),
        SYSCALL_UNLINKAT => sys_unlinkat(args[0], args[1] as *const u8, args[2] as u32),
        SYSCALL_LINKAT => 0,
        SYSCALL_UMOUNT2 => sys_umount2(args[0] as *const u8, args[1] as u32),
        SYSCALL_MOUNT => sys_mount(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
            args[4] as *const u8,
        ),
        SYSCALL_CHDIR => sys_chdir(args[0] as *const u8),
        SYSCALL_OPEN => sys_openat(AT_FDCWD, args[0] as *const u8, args[1] as u32, 0o777u32),
        SYSCALL_OPENAT => sys_openat(
            args[0],
            args[1] as *const u8,
            args[2] as u32,
            args[3] as u32,
        ),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut u32),
        SYSCALL_GENDENTS64 => sys_getdents64(args[0], args[1] as *mut u8, args[2]),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_SENDFILE => sys_sendfile(args[0], args[1], args[2] as *mut usize, args[3]),
        SYSCALL_PPOLL => sys_ppoll(
            args[0] as *mut PollFd,
            args[1],
            args[2] as *const TimeSpec,
            args[3] as *const SignalFlags,
        ),
        SYSCALL_FSTATAT => sys_fstatat(
            args[0],
            args[1] as *const u8,
            args[2] as *mut u8,
            args[3] as u32,
        ),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut u8),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_EXIT_GROUP => sys_exit(args[0] as i32),
        SYSCALL_SET_TID_ADDRESS => sys_set_tid_address(args[0]),
        SYSCALL_SLEEP => sys_sleep(args[0] as *const u64, args[1] as *mut u64),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0], args[1] as u32),
        SYSCALL_SIGACTION => sys_sigaction(
            args[0],
            args[1] as *const SignalAction,
            args[2] as *mut SignalAction,
        ),
        SYSCALL_SIGPROMASK => {
            sys_sigprocmask(args[0], args[1] as *mut u32, args[2] as *mut u32, false)
        }
        SYSCALL_SIGRETURN => sys_sigreturn(),
        SYSCALL_TIMES => sys_get_process_time(args[0] as *mut u64),
        SYSCALL_UNAME => sys_uname(args[0] as *mut u8),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut u64),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_GETPPID => sys_getppid(),
        SYSCALL_GETUID => sys_getuid(),
        SYSCALL_GETEUID => sys_geteuid(),
        SYSCALL_GETTID => sys_gettid(),
        SYSCALL_BRK => sys_brk(args[0]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_CLONE => sys_fork(args[0], args[1], args[2], args[3], args[4]),
        SYSCALL_EXECVE => sys_execve(
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const usize,
        ),
        SYSCALL_MMAP => sys_mmap(
            args[0],
            args[1],
            args[2] as u32,
            args[3] as u32,
            args[4],
            args[5],
        ),
        SYSCALL_MPROTECT => sys_mprotect(args[0], args[1], args[2]),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_THREAD_CREATE => sys_thread_create(args[0], args[1]),
        SYSCALL_WAITTID => sys_waittid(args[0]) as isize,
        SYSCALL_MUTEX_CREATE => sys_mutex_create(args[0] == 1),
        SYSCALL_MUTEX_LOCK => sys_mutex_lock(args[0]),
        SYSCALL_MUTEX_UNLOCK => sys_mutex_unlock(args[0]),
        SYSCALL_SEMAPHORE_CREATE => sys_semaphore_create(args[0]),
        SYSCALL_SEMAPHORE_UP => sys_semaphore_up(args[0]),
        SYSCALL_SEMAPHORE_DOWN => sys_semaphore_down(args[0]),
        SYSCALL_CONDVAR_CREATE => sys_condvar_create(),
        SYSCALL_CONDVAR_SIGNAL => sys_condvar_signal(args[0]),
        SYSCALL_CONDVAR_WAIT => sys_condvar_wait(args[0], args[1]),
        SYSCALL_SHUTDOWN => sys_shutdown(args[0] != 0),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };
    tip!(
        "[syscall] pid: {}, syscall_name: {}, syscall_id: {}, returned {:#x}",
        sys_getpid(),
        syscall_name(syscall_id),
        syscall_id,
        ret
    );
    ret
}
