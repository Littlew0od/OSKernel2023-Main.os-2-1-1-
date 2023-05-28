// use crate::fs::poll::{ppoll, pselect, FdSet, PollFd};
use crate::fs::*;
use crate::mm::{
    translated_byte_buffer, translated_refmut, translated_str, MapPermission, UserBuffer, VirtAddr,
};
// translated_byte_buffer_append_to_existing_vec,copy_from_user, try_get_from_user,
//copy_from_user_array,copy_to_user, copy_to_user_array, copy_to_user_string,
use crate::task::{current_process, current_user_token};
// use crate::timer::TimeSpec;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem::size_of;
use log::{debug, error, info, trace, warn};
use num_enum::FromPrimitive;

use super::errno::*;

pub const AT_FDCWD: usize = 100usize.wrapping_neg();

/// # Warning
/// `fs` & `files` is locked in this function
// fn __openat(dirfd: usize, path: &str) -> Result<FileDescriptor, isize> {
//     let task = current_task().unwrap();
//     let file_descriptor = match dirfd {
//         AT_FDCWD => task.fs.lock().working_inode.as_ref().clone(),
//         fd => {
//             let fd_table = task.files.lock();
//             match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return Err(errno),
//             }
//         }
//     };
//     file_descriptor.open(path, OpenFlags::O_RDONLY, false)
// }

pub fn sys_getpwd(buf: *mut u8, size: usize) -> isize {
    let process = current_process();
    let token = current_user_token();
    if size == 0  {//&& buf != 0
        // The size argument is zero and buf is not a NULL pointer.
        return EINVAL;
    }
    let working_dir = process.inner_exclusive_access().work_path.working_inode.get_cwd().unwrap();
    if working_dir.len() >= size {
        // The size argument is less than the length of the absolute pathname of the working directory,
        // including the terminating null byte.
        return ERANGE;
    }
    let mut userbuf = UserBuffer::new(translated_byte_buffer(token, buf, size));
    let ret = userbuf.write(working_dir.as_bytes());
    if ret == 0 {
        0
    } else {
        buf as isize
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };
    if !file_descriptor.readable() {
        return EBADF;
    }
    file_descriptor.read_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    ) as isize
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };
    if !file_descriptor.writable() {
        return EBADF;
    }
    file_descriptor.write_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    ) as isize
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IOVec {
    iov_base: *const u8, /* Starting address */
    iov_len: usize,      /* Number of bytes to transfer */
}

pub fn sys_dup(oldfd: usize) -> isize{
    1
}

bitflags! {
    pub struct FstatatFlags: u32 {
        const AT_EMPTY_PATH = 0x1000;
        const AT_NO_AUTOMOUNT = 0x800;
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}

pub fn sys_openat(dirfd: usize, path: *const u8, flags: u32, mode: u32) -> isize {
    let process = current_process();
    let token = current_user_token();
    let path = translated_str(token, path);
    let flags = match OpenFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            warn!("[sys_openat] unknown flags");
            return EINVAL;
        }
    };
    let mode = StatMode::from_bits(mode);
    info!(
        "[sys_openat] dirfd: {}, path: {}, flags: {:?}, mode: {:?}",
        dirfd as isize, path, flags, mode
    );
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let file_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => {
            match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            }
        }
    };

    let new_file_descriptor = match file_descriptor.open(&path, flags, false) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };

    let new_fd = match fd_table.insert(new_file_descriptor) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    new_fd as isize
}

// pub fn sys_renameat2(
//     olddirfd: usize,
//     oldpath: *const u8,
//     newdirfd: usize,
//     newpath: *const u8,
//     flags: u32,
// ) -> isize {
//     let task = current_task().unwrap();
//     let token = task.get_user_token();
//     let oldpath = match translated_str(token, oldpath) {
//         Ok(path) => path,
//         Err(errno) => return errno,
//     };
//     let newpath = match translated_str(token, newpath) {
//         Ok(path) => path,
//         Err(errno) => return errno,
//     };
//     info!(
//         "[sys_renameat2] olddirfd: {}, oldpath: {}, newdirfd: {}, newpath: {}, flags: {}",
//         olddirfd as isize, oldpath, newdirfd as isize, newpath, flags
//     );

//     let old_file_descriptor = match olddirfd {
//         AT_FDCWD => task.fs.lock().working_inode.as_ref().clone(),
//         fd => {
//             let fd_table = task.files.lock();
//             match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return errno,
//             }
//         }
//     };
//     let new_file_descriptor = match newdirfd {
//         AT_FDCWD => task.fs.lock().working_inode.as_ref().clone(),
//         fd => {
//             let fd_table = task.files.lock();
//             match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return errno,
//             }
//         }
//     };

//     match FileDescriptor::rename(
//         &old_file_descriptor,
//         &oldpath,
//         &new_file_descriptor,
//         &newpath,
//     ) {
//         Ok(_) => SUCCESS,
//         Err(errno) => errno,
//     }
// }

// pub fn sys_ioctl(fd: usize, cmd: u32, arg: usize) -> isize {
//     let task = current_task().unwrap();
//     let fd_table = task.files.lock();
//     let file_descriptor = match fd_table.get_ref(fd) {
//         Ok(file_descriptor) => file_descriptor,
//         Err(errno) => return errno,
//     };
//     file_descriptor.ioctl(cmd, arg)
// }

// pub fn sys_ppoll(fds: usize, nfds: usize, tmo_p: usize, sigmask: usize) -> isize {
//     ppoll(
//         fds as *mut PollFd,
//         nfds,
//         tmo_p as *const TimeSpec,
//         sigmask as *const crate::task::Signals,
//     )
// }

// pub fn sys_mkdirat(dirfd: usize, path: *const u8, mode: u32) -> isize {
//     let task = current_task().unwrap();
//     let token = task.get_user_token();
//     let path = match translated_str(token, path) {
//         Ok(path) => path,
//         Err(errno) => return errno,
//     };
//     info!(
//         "[sys_mkdirat] dirfd: {}, path: {}, mode: {:?}",
//         dirfd as isize,
//         path,
//         StatMode::from_bits(mode)
//     );
//     let file_descriptor = match dirfd {
//         AT_FDCWD => task.fs.lock().working_inode.as_ref().clone(),
//         fd => {
//             let fd_table = task.files.lock();
//             match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return errno,
//             }
//         }
//     };
//     match file_descriptor.mkdir(&path) {
//         Ok(_) => SUCCESS,
//         Err(errno) => errno,
//     }
// }

bitflags! {
    pub struct UnlinkatFlags: u32 {
        const AT_REMOVEDIR = 0x200;
    }
}

/// # Warning
/// Currently we have no hard-link so this syscall will remove file directly.
// pub fn sys_unlinkat(dirfd: usize, path: *const u8, flags: u32) -> isize {
//     let task = current_task().unwrap();
//     let token = task.get_user_token();
//     let path = match translated_str(token, path) {
//         Ok(path) => path,
//         Err(errno) => return errno,
//     };
//     let flags = match UnlinkatFlags::from_bits(flags) {
//         Some(flags) => flags,
//         None => {
//             warn!("[sys_unlinkat] unknown flags");
//             return EINVAL;
//         }
//     };
//     info!(
//         "[sys_unlinkat] dirfd: {}, path: {}, flags: {:?}",
//         dirfd as isize, path, flags
//     );

//     let file_descriptor = match dirfd {
//         AT_FDCWD => task.fs.lock().working_inode.as_ref().clone(),
//         fd => {
//             let fd_table = task.files.lock();
//             match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return errno,
//             }
//         }
//     };
//     match file_descriptor.delete(&path, flags.contains(UnlinkatFlags::AT_REMOVEDIR)) {
//         Ok(_) => SUCCESS,
//         Err(errno) => errno,
//     }
// }

bitflags! {
    pub struct UmountFlags: u32 {
        const MNT_FORCE           =   1;
        const MNT_DETACH          =   2;
        const MNT_EXPIRE          =   4;
        const UMOUNT_NOFOLLOW     =   8;
    }
}

// pub fn sys_umount2(target: *const u8, flags: u32) -> isize {
//     if target.is_null() {
//         return EINVAL;
//     }
//     let token = current_user_token();
//     let target = match translated_str(token, target) {
//         Ok(target) => target,
//         Err(errno) => return errno,
//     };
//     let flags = match UmountFlags::from_bits(flags) {
//         Some(flags) => flags,
//         None => return EINVAL,
//     };
//     info!("[sys_umount2] target: {}, flags: {:?}", target, flags);
//     warn!("[sys_umount2] fake implementation!");
//     SUCCESS
// }

bitflags! {
    pub struct MountFlags: usize {
        const MS_RDONLY         =   1;
        const MS_NOSUID         =   2;
        const MS_NODEV          =   4;
        const MS_NOEXEC         =   8;
        const MS_SYNCHRONOUS    =   16;
        const MS_REMOUNT        =   32;
        const MS_MANDLOCK       =   64;
        const MS_DIRSYNC        =   128;
        const MS_NOATIME        =   1024;
        const MS_NODIRATIME     =   2048;
        const MS_BIND           =   4096;
        const MS_MOVE           =   8192;
        const MS_REC            =   16384;
        const MS_SILENT         =   32768;
        const MS_POSIXACL       =   (1<<16);
        const MS_UNBINDABLE     =   (1<<17);
        const MS_PRIVATE        =   (1<<18);
        const MS_SLAVE          =   (1<<19);
        const MS_SHARED         =   (1<<20);
        const MS_RELATIME       =   (1<<21);
        const MS_KERNMOUNT      =   (1<<22);
        const MS_I_VERSION      =   (1<<23);
        const MS_STRICTATIME    =   (1<<24);
        const MS_LAZYTIME       =   (1<<25);
        const MS_NOREMOTELOCK   =   (1<<27);
        const MS_NOSEC          =   (1<<28);
        const MS_BORN           =   (1<<29);
        const MS_ACTIVE         =   (1<<30);
        const MS_NOUSER         =   (1<<31);
    }
}

// pub fn sys_mount(
//     source: *const u8,
//     target: *const u8,
//     filesystemtype: *const u8,
//     mountflags: usize,
//     data: *const u8,
// ) -> isize {
//     if source.is_null() || target.is_null() || filesystemtype.is_null() {
//         return EINVAL;
//     }
//     let token = current_user_token();
//     let source = match translated_str(token, source) {
//         Ok(source) => source,
//         Err(errno) => return errno,
//     };
//     let target = match translated_str(token, target) {
//         Ok(target) => target,
//         Err(errno) => return errno,
//     };
//     let filesystemtype = match translated_str(token, filesystemtype) {
//         Ok(filesystemtype) => filesystemtype,
//         Err(errno) => return errno,
//     };
//     // infallible
//     let mountflags = MountFlags::from_bits(mountflags).unwrap();
//     info!(
//         "[sys_mount] source: {}, target: {}, filesystemtype: {}, mountflags: {:?}, data: {:?}",
//         source, target, filesystemtype, mountflags, data
//     );
//     warn!("[sys_mount] fake implementation!");
//     SUCCESS
// }

bitflags! {
    pub struct UtimensatFlags: u32 {
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}

// pub fn sys_utimensat(
//     dirfd: usize,
//     pathname: *const u8,
//     times: *const [TimeSpec; 2],
//     flags: u32,
// ) -> isize {
//     const UTIME_NOW: usize = 0x3fffffff;
//     const UTIME_OMIT: usize = 0x3ffffffe;

//     let token = current_user_token();
//     let path = if !pathname.is_null() {
//         match translated_str(token, pathname) {
//             Ok(path) => path,
//             Err(errno) => return errno,
//         }
//     } else {
//         String::new()
//     };
//     let flags = match UtimensatFlags::from_bits(flags) {
//         Some(flags) => flags,
//         None => {
//             warn!("[sys_utimensat] unknown flags");
//             return EINVAL;
//         }
//     };

//     info!(
//         "[sys_utimensat] dirfd: {}, path: {}, times: {:?}, flags: {:?}",
//         dirfd as isize, path, times, flags
//     );

//     let inode = match __openat(dirfd, &path) {
//         Ok(inode) => inode,
//         Err(errno) => return errno,
//     };

//     let now = TimeSpec::now();
//     let timespec = &mut [now; 2];
//     let mut atime = Some(now.tv_sec);
//     let mut mtime = Some(now.tv_sec);
//     if !times.is_null() {
//         copy_from_user(token, times, timespec);
//         match timespec[0].tv_nsec {
//             UTIME_NOW => (),
//             UTIME_OMIT => atime = None,
//             _ => atime = Some(timespec[0].tv_sec),
//         }
//         match timespec[1].tv_nsec {
//             UTIME_NOW => (),
//             UTIME_OMIT => mtime = None,
//             _ => mtime = Some(timespec[1].tv_sec),
//         }
//     }

//     inode.set_timestamp(None, atime, mtime);
//     SUCCESS
// }

#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u32)]
pub enum Fcntl_Command {
    DUPFD = 0,
    GETFD = 1,
    SETFD = 2,
    GETFL = 3,
    SETFL = 4,
    GETLK = 5,
    SETLK = 6,
    SETLKW = 7,
    SETOWN = 8,
    GETOWN = 9,
    SETSIG = 10,
    GETSIG = 11,
    SETOWN_EX = 15,
    GETOWN_EX = 16,
    GETOWNER_UIDS = 17,
    OFD_GETLK = 36,
    OFD_SETLK = 37,
    OFD_SETLKW = 38,
    SETLEASE = 1024,
    GETLEASE = 1025,
    NOTIFY = 1026,
    CANCELLK = 1029,
    DUPFD_CLOEXEC = 1030,
    SETPIPE_SZ = 1031,
    GETPIPE_SZ = 1032,
    ADD_SEALS = 1033,
    GET_SEALS = 1034,
    GET_RW_HINT = 1035,
    SET_RW_HINT = 1036,
    GET_FILE_RW_HINT = 1037,
    SET_FILE_RW_HINT = 1038,
    #[num_enum(default)]
    ILLEAGAL,
}

// pub fn sys_fcntl(fd: usize, cmd: u32, arg: usize) -> isize {
//     const FD_CLOEXEC: usize = 1;

//     let task = current_task().unwrap();
//     let mut fd_table = task.files.lock();

//     info!(
//         "[sys_fcntl] fd: {}, cmd: {:?}, arg: {:X}",
//         fd,
//         Fcntl_Command::from_primitive(cmd),
//         arg
//     );

//     match Fcntl_Command::from_primitive(cmd) {
//         Fcntl_Command::DUPFD => {
//             let new_file_descriptor = match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return errno,
//             };
//             match fd_table.try_insert_at(new_file_descriptor, arg) {
//                 Ok(fd) => fd as isize,
//                 Err(errno) => errno,
//             }
//         }
//         Fcntl_Command::DUPFD_CLOEXEC => {
//             let mut new_file_descriptor = match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor.clone(),
//                 Err(errno) => return errno,
//             };
//             new_file_descriptor.set_cloexec(true);
//             match fd_table.try_insert_at(new_file_descriptor, arg) {
//                 Ok(fd) => fd as isize,
//                 Err(errno) => errno,
//             }
//         }
//         Fcntl_Command::GETFD => {
//             let file_descriptor = match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor,
//                 Err(errno) => return errno,
//             };
//             file_descriptor.get_cloexec() as isize
//         }
//         Fcntl_Command::SETFD => {
//             let file_descriptor = match fd_table.get_refmut(fd) {
//                 Ok(file_descriptor) => file_descriptor,
//                 Err(errno) => return errno,
//             };
//             file_descriptor.set_cloexec((arg & FD_CLOEXEC) != 0);
//             if (arg & !FD_CLOEXEC) != 0 {
//                 warn!("[fcntl] Unsupported flag exists: {:X}", arg);
//             }
//             SUCCESS
//         }
//         Fcntl_Command::GETFL => {
//             let file_descriptor = match fd_table.get_ref(fd) {
//                 Ok(file_descriptor) => file_descriptor,
//                 Err(errno) => return errno,
//             };
//             // Access control is not fully implemented
//             let mut res = OpenFlags::O_RDWR.bits() as isize;
//             if file_descriptor.get_nonblock() {
//                 res |= OpenFlags::O_NONBLOCK.bits() as isize;
//             }
//             res
//         }
//         command => {
//             warn!("[fcntl] Unsupported command: {:?}", command);
//             SUCCESS
//         } // WARNING!!!
//     }
// }

// pub fn sys_pselect(
//     nfds: usize,
//     read_fds: *mut FdSet,
//     write_fds: *mut FdSet,
//     exception_fds: *mut FdSet,
//     timeout: *mut TimeSpec,
//     sigmask: *const crate::task::signal::Signals,
// ) -> isize {
//     if (nfds as isize) < 0 {
//         return EINVAL;
//     }
//     let token = current_user_token();
//     let mut kread_fds = match try_get_from_user(token, read_fds) {
//         Ok(fds) => fds,
//         Err(errno) => return errno,
//     };
//     let mut kwrite_fds = match try_get_from_user(token, write_fds) {
//         Ok(fds) => fds,
//         Err(errno) => return errno,
//     };
//     let mut kexception_fds = match try_get_from_user(token, exception_fds) {
//         Ok(fds) => fds,
//         Err(errno) => return errno,
//     };
//     let ktimeout = match try_get_from_user(token, timeout) {
//         Ok(timeout) => timeout,
//         Err(errno) => return errno,
//     };
//     let ret = pselect(
//         nfds,
//         &mut kread_fds,
//         &mut kwrite_fds,
//         &mut kexception_fds,
//         &ktimeout,
//         sigmask,
//     );
//     if let Some(kread_fds) = &kread_fds {
//         trace!("[pselect] read_fds: {:?}", kread_fds);
//         copy_to_user(token, kread_fds, read_fds);
//     }
//     if let Some(kwrite_fds) = &kwrite_fds {
//         trace!("[pselect] write_fds: {:?}", kwrite_fds);
//         copy_to_user(token, kwrite_fds, write_fds);
//     }
//     if let Some(kexception_fds) = &kexception_fds {
//         trace!("[pselect] exception_fds: {:?}", kexception_fds);
//         copy_to_user(token, kexception_fds, exception_fds);
//     }
//     ret
// }

/// umask() sets the calling process's file mode creation mask (umask) to
/// mask & 0777 (i.e., only the file permission bits of mask are used),
/// and returns the previous value of the mask.
/// # WARNING
/// In current implementation, umask is always 0. This syscall won't do anything.
pub fn sys_umask(mask: u32) -> isize {
    info!("[sys_umask] mask: {:o}", mask);
    warn!(
        "[sys_umask] In current implementation, umask is always 0. This syscall won't do anything."
    );
    0
}

bitflags! {
    pub struct FaccessatMode: u32 {
        const F_OK = 0;
        const R_OK = 4;
        const W_OK = 2;
        const X_OK = 1;
    }
    pub struct FaccessatFlags: u32 {
        const AT_SYMLINK_NOFOLLOW = 0x100;
        const AT_EACCESS = 0x200;
    }
}

// pub fn sys_faccessat2(dirfd: usize, pathname: *const u8, mode: u32, flags: u32) -> isize {
//     let token = current_user_token();
//     let pathname = match Ok(translated_str(token, pathname)) {
//         Ok(path) => path,
//         Err(errno) => return errno,
//     };
//     let mode = match FaccessatMode::from_bits(mode) {
//         Some(mode) => mode,
//         None => {
//             warn!("[sys_faccessat2] unknown mode");
//             return EINVAL;
//         }
//     };
//     let flags = match FaccessatFlags::from_bits(flags) {
//         Some(flags) => flags,
//         None => {
//             warn!("[sys_faccessat2] unknown flags");
//             return EINVAL;
//         }
//     };

//     info!(
//         "[sys_faccessat2] dirfd: {}, pathname: {}, mode: {:?}, flags: {:?}",
//         dirfd as isize, pathname, mode, flags
//     );

//     // Do not check user's authority, because user group is not implemented yet.
//     // All existing files can be accessed.
//     match __openat(dirfd, pathname.as_str()) {
//         Ok(_) => SUCCESS,
//         Err(errno) => errno,
//     }
// }

bitflags! {
    pub struct MsyncFlags: u32 {
        const MS_ASYNC      =   1;
        const MS_INVALIDATE =   2;
        const MS_SYNC       =   4;
    }
}

// pub fn sys_msync(addr: usize, length: usize, flags: u32) -> isize {
//     if !VirtAddr::from(addr).aligned() {
//         return EINVAL;
//     }
//     let flags = match MsyncFlags::from_bits(flags) {
//         Some(flags) => flags,
//         None => return EINVAL,
//     };
//     let task = current_task().unwrap();
//     if !task
//         .vm
//         .lock()
//         .contains_valid_buffer(addr, length, MapPermission::empty())
//     {
//         return ENOMEM;
//     }
//     info!(
//         "[sys_msync] addr: {:X}, length: {:X}, flags: {:?}",
//         addr, flags, flags
//     );
//     SUCCESS
// }

// pub fn sys_ftruncate(fd: usize, length: isize) -> isize {
//     let task = current_task().unwrap();
//     let fd_table = task.files.lock();
//     let file_descriptor = match fd_table.get_ref(fd) {
//         Ok(file_descriptor) => file_descriptor,
//         Err(errno) => return errno,
//     };
//     match file_descriptor.truncate_size(length) {
//         Ok(()) => SUCCESS,
//         Err(errno) => errno,
//     }
// }
