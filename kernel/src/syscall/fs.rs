#![allow(unused)]
use crate::config::INTERRUPTS_FD;
// use crate::fs::poll::{ppoll, pselect, FdSet, PollFd};
use crate::fs::*;
use crate::mm::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, MapPermission,
    UserBuffer, VirtAddr,
};
use crate::syscall::process;
// translated_byte_buffer_append_to_existing_vec,copy_from_user, try_get_from_user,
//copy_from_user_array,copy_to_user, copy_to_user_array, copy_to_user_string,
use crate::task::{current_process, current_user_token, SignalFlags};
use crate::timer::TimeSpec;
use crate::trap::INTERRUPT;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::write;
use core::mem::size_of;
use core::panic;
use log::{debug, error, info, trace, warn};
use num_enum::FromPrimitive;
use riscv::interrupt;

use super::errno::*;
use super::ppoll::{pselect, FdSet};

pub const AT_FDCWD: usize = 100usize.wrapping_neg();

pub fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    let process = current_process();
    let token = current_user_token();
    if size == 0 {
        //&& buf != 0
        // The size argument is zero and buf is not a NULL pointer.
        return EINVAL;
    }
    let working_dir = process
        .inner_exclusive_access()
        .work_path
        .lock()
        .working_inode
        .get_cwd()
        .unwrap();
    // println!("[sys_getcwd] cwd = {}",working_dir);
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
    // log!("[sys_read] read_fd = {}, count = {:#x}.", fd, len);
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();

    if fd == INTERRUPTS_FD {
        let interrupt_str = INTERRUPT.exclusive_access().get();
        println!("{}", interrupt_str);
        let mut userbuf = UserBuffer::new(translated_byte_buffer(token, buf, len));
        let ret = userbuf.write(interrupt_str.as_bytes());
        return interrupt_str.len() as isize;
    }

    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    if !file_descriptor.readable() {
        return EBADF;
    }
    // release current task TCB manually to avoid multi-borrow
    // yield will happend while pipe reading, which will cause multi-borrow
    drop(fd_table);
    drop(inner);
    drop(process);
    file_descriptor.read_user(
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

pub fn sys_readv(fd: usize, iov: usize, iovcnt: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();

    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    if !file_descriptor.readable() {
        return EBADF;
    }
    let mut read_size = 0;
    for i in 0..iovcnt {
        read_size += file_descriptor.read_user(
            None,
            UserBuffer::new({
                let iov_ref = translated_ref(
                    token,
                    (iov + i * core::mem::size_of::<IOVec>()) as *const IOVec,
                );
                let buf =
                    unsafe { translated_byte_buffer(token, iov_ref.iov_base, iov_ref.iov_len) };
                buf
            }),
        )
    }
    read_size as isize
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    // log!("[sys_write] write_fd = {}, count = {:#x}.", fd, len);
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    if !file_descriptor.writable() {
        return EBADF;
    }
    // release current task TCB manually to avoid multi-borrow
    // yield will happend while pipe reading, which will cause multi-borrow
    drop(fd_table);
    drop(inner);
    drop(process);
    let write_size = file_descriptor.write_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    );
    write_size as isize
}

pub fn sys_writev(fd: usize, iov: usize, iovcnt: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();

    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    if !file_descriptor.writable() {
        return EBADF;
    }
    let mut write_size = 0;
    for i in 0..iovcnt {
        write_size += file_descriptor.write_user(
            None,
            UserBuffer::new({
                let iov_ref = translated_ref(
                    token,
                    (iov + i * core::mem::size_of::<IOVec>()) as *const IOVec,
                );
                let buf =
                    unsafe { translated_byte_buffer(token, iov_ref.iov_base, iov_ref.iov_len) };
                buf
            }),
        );
    }
    write_size as isize
}

pub fn sys_dup(oldfd: usize) -> isize {
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
    newfd as isize
}

pub fn sys_dup3(oldfd: usize, newfd: usize, flags: u32) -> isize {
    // tip!("[sys_dup3] old_fd = {}, new_fd = {}", oldfd, newfd);
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
    file_descriptor.set_cloexec(false); //is_cloexec
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
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
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
    log!(
        "[sys_openat] dirfd: {}, path: {}, flags: {:?}, mode: {:?}",
        dirfd as isize,
        path,
        flags,
        mode
    );
    if path == "/proc/interrupts" && flags.contains(OpenFlags::O_RDONLY) {
        INTERRUPT.exclusive_access().offset = 0;
        return INTERRUPTS_FD as isize;
    }
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();
    let file_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
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

pub fn sys_pipe(pipe: *mut u32) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();

    // return tuples of pipe
    let (pipe_read, pipe_write) = make_pipe();

    // add pipe into file table
    let read_fd = match fd_table.insert(FileDescriptor::new(false, false, pipe_read)) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    let write_fd = match fd_table.insert(FileDescriptor::new(false, false, pipe_write)) {
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    *translated_refmut(token, pipe) = read_fd as u32;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd as u32;
    // tip!("[sys_pipe] read_fd = {}, write_fd = {}", read_fd, write_fd);
    SUCCESS
}

pub fn sys_unlinkat(dirfd: usize, path: *const u8, flags: u32) -> isize {
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
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };
    match file_descriptor.delete(&path, flags.contains(UnlinkatFlags::AT_REMOVEDIR)) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
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
    warn!("[sys_mount] fake implementation!");
    SUCCESS
}

pub fn sys_getdents64(fd: usize, buf: *mut u8, count: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();

    let file_descriptor = match fd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };
    let dirent_vec = match file_descriptor.get_dirent(count) {
        Ok(vec) => vec,
        Err(errno) => return errno,
    };
    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        dirent_vec.len() * size_of::<Dirent>(),
    ));
    let buffer_index = dirent_vec.len().min(count / core::mem::size_of::<Dirent>());
    for index in 0..buffer_index {
        user_buf.write_at(size_of::<Dirent>() * index, dirent_vec[index].as_bytes());
    }

    (dirent_vec.len() * size_of::<Dirent>()) as isize
}

bitflags! {
    pub struct UnlinkatFlags: u32 {
        const AT_REMOVEDIR = 0x200;
    }
}

pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut work_path = inner.work_path.lock();
    let path = translated_str(token, path);
    match work_path.working_inode.cd(&path) {
        Ok(new_working_inode) => {
            work_path.working_inode = new_working_inode;
            SUCCESS
        }
        Err(errno) => errno,
    }
}

pub fn sys_fstat(fd: usize, buf: *mut u8) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();

    let mut fd_table = inner.fd_table.lock();
    let file_descriptor = match fd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };

    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        core::mem::size_of::<Stat>(),
    ));
    user_buf.write(file_descriptor.get_stat().as_bytes());
    SUCCESS
}

bitflags! {
    pub struct FstatatFlags: u32 {
        const AT_EMPTY_PATH = 0x1000;
        const AT_NO_AUTOMOUNT = 0x800;
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}

pub fn sys_fstatat(dirfd: usize, path: *const u8, buf: *mut u8, flags: u32) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    // println!("[sys_fstatat] dirfh = {}, path = {}", dirfd as isize, path);
    let flags = match FstatatFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            warn!("[sys_fstatat] unknown flags");
            return EINVAL;
        }
    };
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();

    let file_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };

    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        core::mem::size_of::<Stat>(),
    ));

    match file_descriptor.open(&path, OpenFlags::O_RDONLY, false) {
        Ok(file_descriptor) => {
            user_buf.write(file_descriptor.get_stat().as_bytes());
            SUCCESS
        }
        Err(errno) => errno,
    }
}

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

pub fn sys_fcntl(fd: usize, cmd: u32, arg: usize) -> isize {
    const FD_CLOEXEC: usize = 1;

    let process = current_process();
    let inner = process.inner_exclusive_access();
    let mut fd_table = inner.fd_table.lock();

    // tip!(
    //     "[sys_fcntl] fd: {}, cmd: {:?}, arg: {:X}",
    //     fd,
    //     Fcntl_Command::from_primitive(cmd),
    //     arg
    // );

    match Fcntl_Command::from_primitive(cmd) {
        Fcntl_Command::DUPFD => {
            let new_file_descriptor = match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            };
            match fd_table.try_insert_at(new_file_descriptor, arg) {
                Ok(fd) => fd as isize,
                Err(errno) => errno,
            }
        }
        Fcntl_Command::DUPFD_CLOEXEC => {
            let mut new_file_descriptor = match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            };
            new_file_descriptor.set_cloexec(true);
            match fd_table.try_insert_at(new_file_descriptor, arg) {
                Ok(fd) => fd as isize,
                Err(errno) => errno,
            }
        }
        Fcntl_Command::GETFD => {
            let file_descriptor = match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor,
                Err(errno) => return errno,
            };
            file_descriptor.get_cloexec() as isize
        }
        Fcntl_Command::SETFD => {
            let file_descriptor = match fd_table.get_refmut(fd) {
                Ok(file_descriptor) => file_descriptor,
                Err(errno) => return errno,
            };
            file_descriptor.set_cloexec((arg & FD_CLOEXEC) != 0);
            if (arg & !FD_CLOEXEC) != 0 {
                warn!("[fcntl] Unsupported flag exists: {:X}", arg);
            }
            SUCCESS
        }
        Fcntl_Command::GETFL => {
            let file_descriptor = match fd_table.get_ref(fd) {
                Ok(file_descriptor) => file_descriptor,
                Err(errno) => return errno,
            };
            // Access control is not fully implemented
            let mut res = OpenFlags::O_RDWR.bits() as isize;
            if file_descriptor.get_nonblock() {
                res |= OpenFlags::O_NONBLOCK.bits() as isize;
            }
            res
        }
        command => {
            warn!("[fcntl] Unsupported command: {:?}", command);
            SUCCESS
        } // WARNING!!!
    }
}

/// If offset is not NULL, then it points to a variable holding the
/// file offset from which sendfile() will start reading data from
/// in_fd.
///
/// When sendfile() returns,
/// this variable will be set to the offset of the byte following
/// the last byte that was read.
///
/// If offset is not NULL, then sendfile() does not modify the file
/// offset of in_fd; otherwise the file offset is adjusted to reflect
/// the number of bytes read from in_fd.
///
/// If offset is NULL, then data will be read from in_fd starting at
/// the file offset, and the file offset will be updated by the call.
pub fn sys_sendfile(out_fd: usize, in_fd: usize, mut offset: *mut usize, count: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let in_file = match fd_table.get_ref(in_fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    let out_file = match fd_table.get_ref(out_fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    // log!("[sys_sendfile] out_fd: {}, in_fd: {}, offset = {}, count = {:#x}", out_fd, in_fd, offset as usize, count);
    if !in_file.readable() || !out_file.writable() {
        return EBADF;
    }

    // turn a pointer in user space into a pointer in kernel space if it is not null
    if offset as usize != 0 {
        offset = translated_refmut(token, offset);
    }

    // a buffer in kernel
    const BUFFER_SIZE: usize = 4096;
    let mut buffer = Vec::<u8>::with_capacity(BUFFER_SIZE);

    let mut left_bytes = count;
    let mut write_size = 0;

    drop(fd_table);
    drop(inner);
    drop(process);
    loop {
        unsafe {
            buffer.set_len(left_bytes.min(BUFFER_SIZE));
        }
        let read_size = in_file.read(unsafe { offset.as_mut() }, buffer.as_mut_slice());
        if read_size == 0 {
            break;
        }
        unsafe {
            buffer.set_len(read_size);
        }
        write_size += out_file.write(None, buffer.as_slice());
        left_bytes -= read_size;
    }
    // tip!("[sys_sendfile] written bytes: {}", write_size);
    write_size as isize
}

pub fn sys_ioctl(fd: usize, cmd: usize, arg: usize) -> isize {
    ENOTTY
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

/// All existing files can be accessed.
pub fn sys_faccessat2(dirfd: usize, pathname: *const u8, mode: u32, flags: u32) -> isize {
    let token = current_user_token();
    let pathname = translated_str(token, pathname);
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();

    let mode = match FaccessatMode::from_bits(mode) {
        Some(mode) => mode,
        None => {
            log!("[sys_faccessat2] unknown mode");
            return EINVAL;
        }
    };
    let flags = match FaccessatFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            log!("[sys_faccessat2] unknown flags");
            return EINVAL;
        }
    };

    let file_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };

    match file_descriptor.open(pathname.as_str(), OpenFlags::O_RDONLY, false) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
    }
}

bitflags! {
    pub struct UtimensatFlags: u32 {
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}

pub fn sys_utimensat(
    dirfd: usize,
    pathname: *const u8,
    times: *mut [TimeSpec; 2],
    flags: u32,
) -> isize {
    const UTIME_NOW: usize = 0x3fffffff;
    const UTIME_OMIT: usize = 0x3ffffffe;

    let token = current_user_token();
    let path = if !pathname.is_null() {
        translated_str(token, pathname)
    } else {
        String::new()
    };
    let flags = match UtimensatFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            warn!("[sys_utimensat] unknown flags");
            return EINVAL;
        }
    };

    // info!(
    //     "[sys_utimensat] dirfd: {}, path: {}, times: {:?}, flags: {:?}",
    //     dirfd as isize, path, times, flags
    // );

    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();

    let dir_descriptor = match dirfd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };
    let file_descriptor = match dir_descriptor.open(path.as_str(), OpenFlags::O_RDONLY, false) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };

    let now = TimeSpec::now();
    let mut atime = Some(now.tv_sec);
    let mut mtime = Some(now.tv_sec);
    if !times.is_null() {
        let timespec = translated_refmut(token, times);
        match timespec[0].tv_nsec {
            UTIME_NOW => (),
            UTIME_OMIT => atime = None,
            _ => atime = Some(timespec[0].tv_sec),
        }
        match timespec[1].tv_nsec {
            UTIME_NOW => (),
            UTIME_OMIT => mtime = None,
            _ => mtime = Some(timespec[1].tv_sec),
        }
    }

    file_descriptor.set_timestamp(None, atime, mtime);
    SUCCESS
}

pub fn sys_lseek(fd: usize, offset: isize, whence: u32) -> isize {
    // whence is not valid
    let whence = match SeekWhence::from_bits(whence) {
        Some(whence) => whence,
        None => {
            warn!("[sys_lseek] unknown flags");
            return EINVAL;
        }
    };
    info!(
        "[sys_lseek] fd: {}, offset: {}, whence: {:?}",
        fd, offset, whence,
    );
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };
    match file_descriptor.lseek(offset, whence) {
        Ok(pos) => pos as isize,
        Err(errno) => errno,
    }
}

pub fn sys_renameat2(
    olddirfd: usize,
    oldpath: *const u8,
    newdirfd: usize,
    newpath: *const u8,
    flags: u32,
) -> isize {
    let token = current_user_token();
    let oldpath = translated_str(token, oldpath);
    let newpath = translated_str(token, newpath);
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();

    info!(
        "[sys_renameat2] olddirfd: {}, oldpath: {}, newdirfd: {}, newpath: {}, flags: {}",
        olddirfd as isize, oldpath, newdirfd as isize, newpath, flags
    );

    let old_file_descriptor = match olddirfd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };
    let new_file_descriptor = match newdirfd {
        AT_FDCWD => inner.work_path.lock().working_inode.as_ref().clone(),
        fd => match fd_table.get_ref(fd) {
            Ok(file_descriptor) => file_descriptor.clone(),
            Err(errno) => return errno,
        },
    };

    match FileDescriptor::rename(
        &old_file_descriptor,
        &oldpath,
        &new_file_descriptor,
        &newpath,
    ) {
        Ok(_) => SUCCESS,
        Err(errno) => errno,
    }
}

pub fn sys_readlinkat(dirfd: usize, pathname: *const u8, buf: *mut u8, bufsiz: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let path = translated_str(token, pathname);
    // log!(
    //     "[sys_readlinkat] dirfd: {}, pathname: {}, buf: {:?}, bufsiz: {}",
    //     dirfd as isize,
    //     path,
    //     buf,
    //     bufsiz,
    // );
    let cwd = process.inner_exclusive_access().self_exe.clone();
    if path.as_str() == "/proc/self/exe" {
        let mut user_buf = UserBuffer::new(translated_byte_buffer(token, buf, bufsiz));
        user_buf.write(cwd.as_bytes()) as isize
    } else {
        log!("[sys_readlinkat] unsupport pathname!");
        return EINVAL;
    }
}

pub fn sys_pread(fd: usize, buf: usize, count: usize, offset: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor,
        Err(errno) => return errno,
    };
    // fd is not open for reading
    if !file_descriptor.readable() {
        return EBADF;
    }
    file_descriptor.read_user(
        Some(offset),
        UserBuffer::new({ translated_byte_buffer(token, buf as *const u8, count) }),
    ) as isize
}

pub fn sys_statfs(_path: *const u8, buf: *const u8) -> isize {
    let token = current_user_token();
    let mut userbuf = UserBuffer::new(translated_byte_buffer(token, buf, size_of::<Statfs>()));
    let statfs = Statfs::new();
    userbuf.write(statfs.as_bytes());
    SUCCESS
}

#[repr(C)]
pub struct Statfs {
    f_type: i64,
    // Type of filesystem
    f_bsize: i64,
    // Optimal transfer block size
    f_blocks: i64,
    // Total data blocks in filesystem
    f_bfree: i64,
    // Free blocks in filesystem
    f_bavail: i64,
    // Free blocks available to unprivileged user
    f_files: i64,
    // Total inodes in filesystem
    f_ffree: i64,
    // Free inodes in filesystem
    f_fsid: i64,
    // Filesystem ID
    f_name_len: i64,
    // Maximum length of filenames
    f_frsize: i64,
    // Fragment size
    f_flags: i64,
    // Mount flags of filesystem
    f_spare: [i64; 4], // Padding bytes
}

impl Statfs {
    pub fn new() -> Self {
        Self {
            f_type: 0x4006,
            f_bsize: 512,
            f_blocks: 1048576,
            f_bfree: 1048576,
            f_bavail: 0,
            f_files: 131072,
            f_ffree: 131072,
            f_fsid: 0,
            f_name_len: 255,
            f_frsize: 0,
            f_flags: 0,
            f_spare: [0; 4],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *mut u8, size) }
    }
}

pub fn sys_pselect(
    nfds: usize,
    read_fds: *mut FdSet,
    write_fds: *mut FdSet,
    exception_fds: *mut FdSet,
    timeout: *mut TimeSpec,
    sigmask: *const SignalFlags,
) -> isize {
    // println!("[sys_pselect] nfds = {:#x}, read_fds = {:#x}, write_fds = {:#x}, exception_fds = {:#x}, timeout = {:#x}, sigmask = {:#x}.", nfds, read_fds as usize,write_fds as usize, exception_fds as usize, timeout as usize, sigmask as usize);
    if (nfds as isize) < 0 {
        return EINVAL;
    }
    let token = current_user_token();
    let mut kread_fds = if read_fds as usize != 0 {
        Some(*translated_ref(token, read_fds))
    } else {
        None
    };
    let mut kwrite_fds = if write_fds as usize != 0 {
        Some(*translated_ref(token, write_fds))
    } else {
        None
    };
    let mut kexception_fds = if exception_fds as usize != 0 {
        Some(*translated_ref(token, exception_fds))
    } else {
        None
    };
    let ktimeout = if timeout as usize != 0 {
        Some(*translated_ref(token, timeout))
    } else {
        None
    };
    let ret = pselect(
        nfds,
        &mut kread_fds,
        &mut kwrite_fds,
        &mut kexception_fds,
        &ktimeout,
        sigmask,
    );
    if let Some(kread_fds) = &kread_fds {
        trace!("[pselect] read_fds: {:?}", kread_fds);
        // copy_to_user(token, kread_fds, read_fds);
        *translated_refmut(token, read_fds) = *kread_fds;
    }
    if let Some(kwrite_fds) = &kwrite_fds {
        trace!("[pselect] write_fds: {:?}", kwrite_fds);
        // copy_to_user(token, kwrite_fds, write_fds);
        *translated_refmut(token, write_fds) = *kwrite_fds;
    }
    if let Some(kexception_fds) = &kexception_fds {
        trace!("[pselect] exception_fds: {:?}", kexception_fds);
        // copy_to_user(token, kexception_fds, exception_fds);
        *translated_refmut(token, exception_fds) = *kexception_fds;
    }
    ret
}
