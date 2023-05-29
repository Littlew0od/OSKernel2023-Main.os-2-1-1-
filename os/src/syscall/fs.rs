#![allow(unused)]
// use crate::fs::poll::{ppoll, pselect, FdSet, PollFd};
use crate::fs::*;
use crate::mm::{
    translated_byte_buffer, translated_refmut, translated_str, MapPermission, UserBuffer, VirtAddr,
};
use crate::syscall::process;
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

pub fn sys_dup(oldfd: usize) -> isize{
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let old_file_descriptor = match fd_table.get_ref(oldfd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    let newfd = match fd_table.insert(old_file_descriptor) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    info!("[sys_dup] oldfd: {}, newfd: {}", oldfd, newfd);
    newfd as isize
}

pub fn sys_dup3(oldfd: usize, newfd: usize, flags: u32) -> isize {
    info!(
        "[sys_dup3] oldfd: {}, newfd: {}, flags: {:?}",
        oldfd,
        newfd,
        OpenFlags::from_bits(flags)
    );
    if oldfd == newfd {
        return EINVAL;
    }
    let is_cloexec = match OpenFlags::from_bits(flags) {
        Some(OpenFlags::O_CLOEXEC) => true,
        // `O_RDONLY == 0`, so it means *NO* cloexec in fact
        Some(OpenFlags::O_RDONLY) => false,
        // flags contain an invalid value
        Some(flags) => {
            warn!("[sys_dup3] invalid flags: {:?}", flags);
            return EINVAL;
        }
        None => {
            warn!("[sys_dup3] unknown flags");
            return EINVAL;
        }
    };
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();

    let mut file_descriptor = match fd_table.get_ref(oldfd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    file_descriptor.set_cloexec(is_cloexec);
    match fd_table.insert_at(file_descriptor, newfd) {
        Ok(fd) => fd as isize,
        Err(errno) => errno,
    }
}

pub fn sys_mkdirat(dirfd: usize, path: *const u8, mode: u32) -> isize {
    // let task = current_task().unwrap();
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let path = translated_str(token, path);
    info!(
        "[sys_mkdirat] dirfd: {}, path: {}, mode: {:?}",
        dirfd as isize,
        path,
        StatMode::from_bits(mode)
    );
    let file_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => {
            let fd_table = inner.fd_table.lock();
            match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            }
        }
    };
    match file_descriptor.mkdir(&path) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
    }
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

pub fn sys_close(fd: usize) -> isize {
    info!("[sys_close] fd: {}", fd);
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    match fd_table.remove(fd) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
    }
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let token = current_user_token();
    let (pipe_read, pipe_write) = make_pipe();
    
    let read_fd = match fd_table.insert(FileDescriptor::new(
        false,
        false,
        pipe_read,
    )) {
        Ok(fd) => fd, 
        Err(errno) => return errno,
    };
    let write_fd = match fd_table.insert(FileDescriptor::new(
        false,
        false,
        pipe_write,
    )) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    SUCCESS
}

pub fn sys_unlinkat(dirfd: usize, path: *const u8, flags: u32) -> isize{
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let path = translated_str(token, path);
    let flags = match UnlinkatFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            warn!("[sys_unlinkat] unknown flags");
            return EINVAL;
        }
    };
    info!(
        "[sys_unlinkat] dirfd: {}, path: {}, flags: {:?}",
        dirfd as isize, path, flags
    );

    let file_descriptor = match dirfd {
        // AT_FDCWD => task.fs.lock().working_inode.as_ref().clone(),
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => {
            match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            }
        }
    };
    match file_descriptor.delete(&path, flags.contains(UnlinkatFlags::AT_REMOVEDIR)) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
    }
}


pub fn sys_umount2(target: *const u8, flags: u32) -> isize {
    if target.is_null() {
        return EINVAL;
    }
    let token = current_user_token();
    let target = translated_str(token, target);
    let flags = match UmountFlags::from_bits(flags) {
        Some(flags) => flags,
        None => return EINVAL,
    };
    info!("[sys_umount2] target: {}, flags: {:?}", target, flags);
    warn!("[sys_umount2] fake implementation!");
    SUCCESS
}

pub fn sys_mount(
    source: *const u8,
    target: *const u8,
    filesystemtype: *const u8,
    mountflags: usize,
    data: *const u8,
) -> isize {
    if source.is_null() || target.is_null() || filesystemtype.is_null() {
        return EINVAL;
    }
    let token = current_user_token();
    let source = translated_str(token, source);
    let target = translated_str(token, target);
    let filesystemtype = translated_str(token, filesystemtype);
    // infallible
    let mountflags = MountFlags::from_bits(mountflags).unwrap();
    info!(
        "[sys_mount] source: {}, target: {}, filesystemtype: {}, mountflags: {:?}, data: {:?}",
        source, target, filesystemtype, mountflags, data
    );
    warn!("[sys_mount] fake implementation!");
    SUCCESS
}


pub fn sys_getdents64(fd: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let file_descriptor = match fd {
        AT_FDCWD => inner.work_path.working_inode.as_ref().clone(),
        fd => {
            match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            }
        }
    };
    let dirent_vec = match file_descriptor.get_dirent(len) {
        Ok(vec) => vec,
        Err(errno) => return errno,
    };
    // copy_to_user_array(
    //     token,
    //     dirent_vec.as_ptr(),
    //     dirp as *mut Dirent,
    //     dirent_vec.len(),
    // );
    let mut user_buf = UserBuffer::new(translated_byte_buffer(token, buf, len));
    user_buf.write(dirent_vec[0].as_bytes());
    info!("[sys_getdents64] fd: {}, count: {}", fd, len);
    (dirent_vec.len() * size_of::<Dirent>()) as isize
}

bitflags! {
    pub struct UnlinkatFlags: u32 {
        const AT_REMOVEDIR = 0x200;
    }
}
bitflags! {
    pub struct UmountFlags: u32 {
        const MNT_FORCE           =   1;
        const MNT_DETACH          =   2;
        const MNT_EXPIRE          =   4;
        const UMOUNT_NOFOLLOW     =   8;
    }
}

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

bitflags! {
    pub struct UtimensatFlags: u32 {
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}

// pub fn sys_utimensat(


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

bitflags! {
    pub struct MsyncFlags: u32 {
        const MS_ASYNC      =   1;
        const MS_INVALIDATE =   2;
        const MS_SYNC       =   4;
    }
}