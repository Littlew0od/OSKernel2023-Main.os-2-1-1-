mod config;
pub mod errno;
mod fs;
mod ppoll;
mod process;
mod signal;
mod sync;
mod syslog;
mod system;
mod thread;

use crate::{
    task::{SigInfo, SignalAction, SignalFlags, Rusage},
    timer::{TimeSpec, Times},
};
use config::*;
use fs::*;
use ppoll::*;
pub use process::sys_getpid;
use process::*;
use signal::*;
use sync::*;
use system::*;
use thread::*;

pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    // println!(
    //     "[kernel] syscall start, syscall_name: {}, syscall_id: {}",
    //     syscall_name(syscall_id),
    //     syscall_id,
    // );
    let ret = match syscall_id {
        SYSCALL_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_DUP3 => sys_dup3(args[0], args[1], args[2] as u32),
        SYSCALL_FCNTL64 => sys_fcntl(args[0], args[1] as u32, args[2]),
        SYSCALL_IOCTL => sys_ioctl(args[0], args[1], args[2]),
        SYSCALL_MKDIRAT => sys_mkdirat(args[0], args[1] as *const u8, args[2] as u32),
        SYSCALL_UNLINKAT => sys_unlinkat(args[0], args[1] as *const u8, args[2] as u32),
        SYSCALL_UMOUNT2 => sys_umount2(args[0] as *const u8, args[1] as u32),
        SYSCALL_MOUNT => sys_mount(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
            args[4] as *const u8,
        ),
        SYSCALL_FACCESSAT => sys_faccessat2(args[0], args[1] as *const u8, args[2] as u32, 0u32),
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
        SYSCALL_GETDENTS64 => sys_getdents64(args[0], args[1] as *mut u8, args[2]),
        SYSCALL_LSEEK => sys_lseek(args[0], args[1] as isize, args[2] as u32),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_READV => sys_readv(args[0], args[1], args[2]),
        SYSCALL_WRITEV => sys_writev(args[0], args[1], args[2]),
        SYSCALL_STATFS =>sys_statfs(args[0] as *const u8, args[1] as *const u8),
        SYSCALL_PREAD => sys_pread(args[0], args[1], args[2], args[3]),
        SYSCALL_SENDFILE => sys_sendfile(args[0], args[1], args[2] as *mut usize, args[3]),
        SYSCALL_PPOLL => sys_ppoll(
            args[0] as *mut PollFd,
            args[1],
            args[2] as *const TimeSpec,
            args[3] as *const SignalFlags,
        ),
        SYSCALL_READLINKAT => {
            sys_readlinkat(args[0], args[1] as *const u8, args[2] as *mut u8, args[3])
        }
        SYSCALL_FSTATAT => sys_fstatat(
            args[0],
            args[1] as *const u8,
            args[2] as *mut u8,
            args[3] as u32,
        ),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut u8),
        SYSCALL_UTIMENSAT => sys_utimensat(
            args[0],
            args[1] as *const u8,
            args[2] as *mut [TimeSpec; 2],
            args[3] as u32,
        ),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_EXIT_GROUP => sys_exit(args[0] as i32),
        SYSCALL_SET_TID_ADDRESS => sys_set_tid_address(args[0]),
        SYSCALL_FUTEX => sys_futex(
            args[0] as *mut u32,
            args[1],
            args[2] as u32,
            args[3] as *const TimeSpec,
            args[4] as *const u32,
            args[5] as u32,
        ),
        SYSCALL_SLEEP => sys_sleep(args[0] as *const u64, args[1] as *mut u64),
        SYSCALL_SYSLOG => sys_syslog(args[0], args[1] as *mut u8, args[2]),
        SYSCALL_CLOCK_GETTIME => sys_clock_gettime(args[0], args[1] as *mut TimeSpec),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0], args[1]),
        SYSCALL_TKILL => sys_tkill(args[0], args[1]),
        SYSCALL_SIGACTION => sys_sigaction(
            args[0],
            args[1] as *const SignalAction,
            args[2] as *mut SignalAction,
        ),
        SYSCALL_SIGPROMASK => {
            sys_sigprocmask(args[0], args[1] as *mut usize, args[2] as *mut usize, false)
        }
        SYSCALL_SIGTIMEDWAIT => sys_sigtimedwait(
            args[0] as *mut usize,
            args[1] as *mut SigInfo,
            args[2] as *const TimeSpec,
            args[3],
        ),
        SYSCALL_SIGRETURN => sys_sigreturn(),
        // SYSCALL_TIMES => sys_get_process_time(args[0] as *mut u64),
        SYSCALL_TIMES => sys_times(args[0] as *mut Times),
        SYSCALL_UNAME => sys_uname(args[0] as *mut u8),
        SYSCALL_GETRUSAGE => sys_getrusage(args[0] as isize, args[1] as *mut Rusage),
        SYSCALL_GET_TIME_DAY => sys_get_time_day(args[0] as *mut crate::timer::TimeVal),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_GETPPID => sys_getppid(),
        SYSCALL_GETUID => sys_getuid(),
        SYSCALL_GETEUID => sys_geteuid(),
        SYSCALL_GETEGID => sys_getegid(),
        SYSCALL_GETTID => sys_gettid(),
        SYSCALL_SYSINFO => sys_sysinfo(args[0] as *mut u8),
        SYSCALL_BRK => sys_brk(args[0]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_CLONE => sys_clone(
            args[0],
            args[1],
            args[2] as *mut usize,
            args[3],
            args[4] as *mut usize,
        ),
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
        SYSCALL_WAIT4 => sys_wait4(
            args[0] as isize,
            args[1] as *mut i32,
            args[2] as u32,
            args[3],
        ),
        SYSCALL_PRLIMIT => sys_prlimit(
            args[0],
            args[1] as u32,
            args[2] as *const RLimit,
            args[3] as *mut RLimit,
        ),
        SYSCALL_RENAMEAT2 => sys_renameat2(
            args[0],
            args[1] as *const u8,
            args[2],
            args[3] as *const u8,
            args[4] as u32,
        ),
        SYSCALL_WAITTID => sys_waittid(args[0]) as isize,
        SYSCALL_SHUTDOWN => sys_shutdown(args[0] != 0),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };
    // tip!(
    //     "[syscall] pid: {}, syscall_name: {}, syscall_id: {}, returned {:#x}",
    //     sys_getpid(),
    //     syscall_name(syscall_id),
    //     syscall_id,
    //     ret
    // );

    // if [
    //     SYSCALL_SIGACTION,
    //     SYSCALL_CLOCK_GETTIME,
    //     SYSCALL_SIGACTION,
    //     SYSCALL_SIGPROMASK,
    // ]
    // .contains(&syscall_id)
    // {
    //     tip!(
    //         "[syscall] pid: {}, syscall_name: {}, syscall_id: {}, returned {:#x}",
    //         sys_getpid(),
    //         syscall_name(syscall_id),
    //         syscall_id,
    //         ret
    //     );
    // }
    ret
}
