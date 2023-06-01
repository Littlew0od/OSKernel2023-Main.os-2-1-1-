use crate::fs::directory_tree::DirectoryTreeNode;
use crate::fs::{File, Stat, StatMode, DiskInodeType};
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use crate::syscall::errno::{ENOTDIR, ESPIPE};
use alloc::{sync::{Arc, Weak}, vec::Vec};
use spin::Mutex;

use crate::task::suspend_current_and_run_next;

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
    pub fn read_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }
    pub fn write_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

const RING_BUFFER_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }
    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }
    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        c
    }
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

/// Return (read_end, write_end)
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(unsafe { UPSafeCell::new(PipeRingBuffer::new()) });
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);
    (read_end, write_end)
}

impl File for Pipe {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        assert!(self.readable());
        let want_to_read = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_read = 0usize;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                return already_read;
                // if ring_buffer.all_write_ends_closed() {
                //     return already_read;
                // }
                // drop(ring_buffer);
                // suspend_current_and_run_next();
                // continue;
            }
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();
                    }
                    already_read += 1;
                    if already_read == want_to_read {
                        return want_to_read;
                    }
                } else {
                    return already_read;
                }
            }
        }
    }
    fn write_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        assert!(self.writable());
        let want_to_write = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_write = 0usize;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let loop_write = ring_buffer.available_write();
            if loop_write == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            // write at most loop_write bytes
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    already_write += 1;
                    if already_write == want_to_write {
                        return want_to_write;
                    }
                } else {
                    return already_write;
                }
            }
        }
    }

    fn deep_clone(&self) -> Arc<dyn File> {
        todo!()
    }

    fn read(&self, offset: Option<&mut usize>, buf: &mut [u8]) -> usize {
        todo!()
    }

    fn write(&self, offset: Option<&mut usize>, buf: &[u8]) -> usize {
       todo!()
    }

    fn r_ready(&self) -> bool {
        let ring_buffer = self.buffer.exclusive_access();
        ring_buffer.status != RingBufferStatus::Empty
    }

    fn w_ready(&self) -> bool {
        let ring_buffer = self.buffer.exclusive_access();
        ring_buffer.status != RingBufferStatus::Full
    }

    fn read_all(&self) -> Vec<u8> {
        Vec::new()
    }

    fn get_size(&self) -> usize {
        RING_BUFFER_SIZE
    }

    fn get_stat(&self) -> Stat {
        Stat::new(
            crate::makedev!(8, 0),
            1,
            StatMode::S_IFIFO.bits() | 0o666,
            1,
            0,
            0,
            0,
            0,
            0,
        )
    }

    fn get_file_type(&self) -> DiskInodeType {
        DiskInodeType::File
    }

    fn info_dirtree_node(&self, dirnode_ptr: Weak<crate::fs::directory_tree::DirectoryTreeNode>) {
        todo!()
    }

    fn get_dirtree_node(&self) -> Option<Arc<DirectoryTreeNode>> {
        todo!()
    }

    fn open(&self, flags: crate::fs::layout::OpenFlags, special_use: bool) -> Arc<dyn File> {
        todo!()
    }

    fn open_subfile(
        &self,
    ) -> Result<alloc::vec::Vec<(alloc::string::String, alloc::sync::Arc<dyn File>)>, isize> {
        Err(ENOTDIR)
    }

    fn create(&self, name: &str, file_type: DiskInodeType) -> Result<Arc<dyn File>, isize> {
        todo!()
    }

    fn link_child(&self, name: &str, child: &Self) -> Result<(), isize>
    where
        Self: Sized,
    {
        todo!()
    }

    fn unlink(&self, delete: bool) -> Result<(), isize> {
        todo!()
    }

    fn get_dirent(&self, count: usize) -> alloc::vec::Vec<crate::fs::layout::Dirent> {
        todo!()
    }

    fn lseek(&self, offset: isize, whence: crate::fs::SeekWhence) -> Result<usize, isize> {
        Err(ESPIPE)
    }

    fn modify_size(&self, diff: isize) -> Result<(), isize> {
        todo!()
    }

    fn truncate_size(&self, new_size: usize) -> Result<(), isize> {
        todo!()
    }

    fn set_timestamp(&self, ctime: Option<usize>, atime: Option<usize>, mtime: Option<usize>) {
        todo!()
    }

    fn get_single_cache(&self, offset: usize) -> Result<Arc<Mutex<crate::fs::PageCache>>, ()> {
        todo!()
    }

    fn get_all_caches(&self) -> Result<alloc::vec::Vec<Arc<Mutex<crate::fs::PageCache>>>, ()> {
        todo!()
    }

    fn oom(&self) -> usize {
        0
    }

    fn hang_up(&self) -> bool {
        todo!()
    }

    fn fcntl(&self, cmd: u32, arg: u32) -> isize {
        todo!()
    }
}

// use crate::config::PAGE_SIZE;
// use crate::fs::directory_tree::DirectoryTreeNode;
// use crate::fs::layout::Stat;
// use crate::fs::DiskInodeType;
// use crate::fs::StatMode;
// use crate::syscall::errno::*;
// use crate::task::{current_task, suspend_current_and_run_next};
// use crate::{fs::file_trait::File, mm::UserBuffer};
// use alloc::boxed::Box;
// use alloc::sync::{Arc, Weak};
// use alloc::vec::Vec;
// use core::ptr::copy_nonoverlapping;
// use num_enum::FromPrimitive;
// use spin::Mutex;

// pub struct Pipe {
//     readable: bool,
//     writable: bool,
//     buffer: Arc<Mutex<PipeRingBuffer>>,
// }

// impl Pipe {
//     pub fn read_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
//         Self {
//             readable: true,
//             writable: false,
//             buffer,
//         }
//     }
//     pub fn write_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
//         Self {
//             readable: false,
//             writable: true,
//             buffer,
//         }
//     }
// }

// #[cfg(feature = "board_fu740")]
// const RING_DEFAULT_BUFFER_SIZE: usize = 4096 * 16;
// #[cfg(not(feature = "board_fu740"))]
// const RING_DEFAULT_BUFFER_SIZE: usize = 256;

// #[derive(Copy, Clone, PartialEq, Debug)]
// enum RingBufferStatus {
//     FULL,
//     EMPTY,
//     NORMAL,
// }

// pub struct PipeRingBuffer {
//     arr: Box<[u8; RING_DEFAULT_BUFFER_SIZE]>,
//     head: usize,
//     tail: usize,
//     status: RingBufferStatus,
//     write_end: Option<Weak<Pipe>>,
//     read_end: Option<Weak<Pipe>>,
// }

// impl PipeRingBuffer {
//     fn new() -> Self {
//         // let mut vec = Vec::<u8>::with_capacity(RING_DEFAULT_BUFFER_SIZE);
//         // unsafe {
//         //     vec.set_len(RING_DEFAULT_BUFFER_SIZE);
//         // }
//         Self {
//             arr: Box::new([0u8; RING_DEFAULT_BUFFER_SIZE]),
//             head: 0,
//             tail: 0,
//             status: RingBufferStatus::EMPTY,
//             write_end: None,
//             read_end: None,
//         }
//     }
//     fn get_used_size(&self) -> usize {
//         if self.status == RingBufferStatus::FULL {
//             self.arr.len()
//         } else if self.status == RingBufferStatus::EMPTY {
//             0
//         } else {
//             assert!(self.head != self.tail);
//             if self.head < self.tail {
//                 self.tail - self.head
//             } else {
//                 self.tail + self.arr.len() - self.head
//             }
//         }
//     }
//     #[inline]
//     fn buffer_read(&mut self, buf: &mut [u8]) -> usize {
//         // get range
//         let begin = self.head;
//         let end = if self.tail <= self.head {
//             RING_DEFAULT_BUFFER_SIZE
//         } else {
//             self.tail
//         };
//         // copy
//         let read_bytes = buf.len().min(end - begin);
//         unsafe {
//             copy_nonoverlapping(self.arr.as_ptr().add(begin), buf.as_mut_ptr(), read_bytes);
//         };
//         // update head
//         self.head = if begin + read_bytes == RING_DEFAULT_BUFFER_SIZE {
//             0
//         } else {
//             begin + read_bytes
//         };
//         read_bytes
//     }
//     #[inline]
//     fn buffer_write(&mut self, buf: &[u8]) -> usize {
//         // get range
//         let begin = self.tail;
//         let end = if self.tail < self.head {
//             self.head
//         } else {
//             RING_DEFAULT_BUFFER_SIZE
//         };
//         // write
//         let write_bytes = buf.len().min(end - begin);
//         unsafe {
//             copy_nonoverlapping(buf.as_ptr(), self.arr.as_mut_ptr().add(begin), write_bytes);
//         };
//         // update tail
//         self.tail = if begin + write_bytes == RING_DEFAULT_BUFFER_SIZE {
//             0
//         } else {
//             begin + write_bytes
//         };
//         write_bytes
//     }
//     fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
//         self.write_end = Some(Arc::downgrade(write_end));
//     }
//     fn set_read_end(&mut self, read_end: &Arc<Pipe>) {
//         self.read_end = Some(Arc::downgrade(read_end));
//     }
//     fn all_write_ends_closed(&self) -> bool {
//         self.write_end.as_ref().unwrap().upgrade().is_none()
//     }
//     fn all_read_ends_closed(&self) -> bool {
//         self.read_end.as_ref().unwrap().upgrade().is_none()
//     }
// }

// /// Return (read_end, write_end)
// pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
//     let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
//     // buffer仅剩两个强引用，这样读写端关闭后就会被释放
//     let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
//     let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
//     buffer.lock().set_write_end(&write_end);
//     buffer.lock().set_read_end(&read_end);
//     (read_end, write_end)
// }

// #[allow(unused)]
// impl File for Pipe {
//     fn deep_clone(&self) -> Arc<dyn File> {
//         todo!()
//     }

//     fn readable(&self) -> bool {
//         self.readable
//     }

//     fn writable(&self) -> bool {
//         self.writable
//     }

//     fn read(&self, offset: Option<&mut usize>, buf: &mut [u8]) -> usize {
//         if offset.is_some() {
//             todo!()
//         }
//         let mut read_size = 0usize;
//         loop {
//             let mut ring = self.buffer.lock();
//             if ring.status == RingBufferStatus::EMPTY {
//                 if ring.all_write_ends_closed() {
//                     return read_size;
//                 }
//                 drop(ring);
//                 suspend_current_and_run_next();
//                 continue;
//             }
//             // We guarantee that this operation will read at least one byte
//             let mut buf_start = 0;
//             while buf_start < buf.len() {
//                 let read_bytes = ring.buffer_read(&mut buf[buf_start..]);
//                 buf_start += read_bytes;
//                 read_size += read_bytes;
//                 if ring.head == ring.tail {
//                     ring.status = RingBufferStatus::EMPTY;
//                     read_size += buf_start;
//                     return read_size;
//                 }
//             }
//             read_size += buf_start;

//             ring.status = RingBufferStatus::NORMAL;
//             return read_size;
//         }
//     }

//     fn write(&self, offset: Option<&mut usize>, buf: &[u8]) -> usize {
//         if offset.is_some() {
//             todo!()
//         }
//         let mut write_size = 0usize;
//         loop {
//             // let task = current_task().unwrap();
//             // let inner = task.acquire_inner_lock();
//             // if !inner.sigpending.difference(inner.sigmask).is_empty() {
//             //     return ERESTART as usize;
//             // }
//             // drop(inner);
//             // drop(task);
//             let mut ring = self.buffer.lock();
//             if ring.status == RingBufferStatus::FULL {
//                 if ring.all_read_ends_closed() {
//                     return write_size;
//                 }
//                 drop(ring);
//                 suspend_current_and_run_next();
//                 continue;
//             }
//             // We guarantee that this operation will write at least one byte
//             // So we modify status first
//             let mut buf_start = 0;
//             while buf_start < buf.len() {
//                 let write_bytes = ring.buffer_write(&buf[buf_start..]);
//                 buf_start += write_bytes;
//                 write_size += write_bytes;
//                 if ring.head == ring.tail {
//                     ring.status = RingBufferStatus::FULL;
//                     write_size += buf_start;
//                     return write_size;
//                 }
//             }
//             write_size += buf_start;

//             ring.status = RingBufferStatus::NORMAL;
//             return write_size;
//         }
//     }

//     fn r_ready(&self) -> bool {
//         let ring_buffer = self.buffer.lock();
//         ring_buffer.status != RingBufferStatus::EMPTY
//     }

//     fn w_ready(&self) -> bool {
//         let ring_buffer = self.buffer.lock();
//         ring_buffer.status != RingBufferStatus::FULL
//     }

//     fn read_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
//         if offset.is_some() {
//             return ESPIPE as usize;
//         }
//         let mut read_size = 0usize;
//         loop {
//             // let task = current_task().unwrap();
//             // let inner = task.acquire_inner_lock();
//             // if !inner.sigpending.difference(inner.sigmask).is_empty() {
//             //     return ERESTART as usize;
//             // }
//             // drop(inner);
//             // drop(task);
//             let mut ring = self.buffer.lock();
//             if ring.status == RingBufferStatus::EMPTY {
//                 if ring.all_write_ends_closed() {
//                     return read_size;
//                 }
//                 drop(ring);
//                 suspend_current_and_run_next();
//                 continue;
//             }
//             // We guarantee that this operation will read at least one byte
//             // So we modify status first
//             for buf in buf.buffers {
//                 let mut buf_start = 0;
//                 while buf_start < buf.len() {
//                     let read_bytes = ring.buffer_read(&mut buf[buf_start..]);
//                     buf_start += read_bytes;
//                     if ring.head == ring.tail {
//                         ring.status = RingBufferStatus::EMPTY;
//                         read_size += buf_start;
//                         return read_size;
//                     }
//                 }
//                 read_size += buf_start;
//             }
//             ring.status = RingBufferStatus::NORMAL;
//             return read_size;
//         }
//     }
//     fn read_all(&self) -> Vec<u8> {
//         Vec::new()
//     }

//     fn write_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
//         if offset.is_some() {
//             return ESPIPE as usize;
//         }
//         let mut write_size = 0usize;
//         loop {
//             // let task = current_task().unwrap();
//             // let inner = task.acquire_inner_lock();
//             // if !inner.sigpending.difference(inner.sigmask).is_empty() {
//             //     return ERESTART as usize;
//             // }
//             // drop(inner);
//             // drop(task);
//             let mut ring = self.buffer.lock();
//             if ring.status == RingBufferStatus::FULL {
//                 if ring.all_read_ends_closed() {
//                     return write_size;
//                 }
//                 drop(ring);
//                 suspend_current_and_run_next();
//                 continue;
//             }
//             // We guarantee that this operation will write at least one byte
//             // So we modify status first
//             for buf in buf.buffers {
//                 let mut buf_start = 0;
//                 while buf_start < buf.len() {
//                     let write_bytes = ring.buffer_write(&buf[buf_start..]);
//                     buf_start += write_bytes;
//                     if ring.head == ring.tail {
//                         ring.status = RingBufferStatus::FULL;
//                         write_size += buf_start;
//                         return write_size;
//                     }
//                 }
//                 write_size += buf_start;
//             }
//             ring.status = RingBufferStatus::NORMAL;
//             return write_size;
//         }
//     }

//     fn get_size(&self) -> usize {
//         todo!()
//     }

//     fn get_stat(&self) -> Stat {
//         Stat::new(
//             crate::makedev!(8, 0),
//             1,
//             StatMode::S_IFIFO.bits() | 0o666,
//             1,
//             0,
//             0,
//             0,
//             0,
//             0,
//         )
//     }

//     fn get_file_type(&self) -> DiskInodeType {
//         DiskInodeType::File
//     }

//     fn info_dirtree_node(&self, dirnode_ptr: Weak<crate::fs::directory_tree::DirectoryTreeNode>) {
//         todo!()
//     }

//     fn get_dirtree_node(&self) -> Option<Arc<DirectoryTreeNode>> {
//         todo!()
//     }

//     fn open(&self, flags: crate::fs::layout::OpenFlags, special_use: bool) -> Arc<dyn File> {
//         todo!()
//     }

//     fn open_subfile(
//         &self,
//     ) -> Result<alloc::vec::Vec<(alloc::string::String, alloc::sync::Arc<dyn File>)>, isize> {
//         Err(ENOTDIR)
//     }

//     fn create(&self, name: &str, file_type: DiskInodeType) -> Result<Arc<dyn File>, isize> {
//         todo!()
//     }

//     fn link_child(&self, name: &str, child: &Self) -> Result<(), isize>
//     where
//         Self: Sized,
//     {
//         todo!()
//     }

//     fn unlink(&self, delete: bool) -> Result<(), isize> {
//         todo!()
//     }

//     fn get_dirent(&self, count: usize) -> alloc::vec::Vec<crate::fs::layout::Dirent> {
//         todo!()
//     }

//     fn lseek(&self, offset: isize, whence: crate::fs::SeekWhence) -> Result<usize, isize> {
//         Err(ESPIPE)
//     }

//     fn modify_size(&self, diff: isize) -> Result<(), isize> {
//         todo!()
//     }

//     fn truncate_size(&self, new_size: usize) -> Result<(), isize> {
//         todo!()
//     }

//     fn set_timestamp(&self, ctime: Option<usize>, atime: Option<usize>, mtime: Option<usize>) {
//         todo!()
//     }

//     fn get_single_cache(&self, offset: usize) -> Result<Arc<Mutex<crate::fs::PageCache>>, ()> {
//         todo!()
//     }

//     fn get_all_caches(&self) -> Result<alloc::vec::Vec<Arc<Mutex<crate::fs::PageCache>>>, ()> {
//         todo!()
//     }

//     fn oom(&self) -> usize {
//         0
//     }

//     fn hang_up(&self) -> bool {
//         // The peer has closed its end.
//         // Or maybe you should only check whether both ends have been closed by the peer.
//         if self.readable {
//             self.buffer.lock().all_write_ends_closed()
//         } else {
//             //writable
//             self.buffer.lock().all_read_ends_closed()
//         }
//     }

//     fn fcntl(&self, cmd: u32, arg: u32) -> isize {
//         todo!()
//     }
// }
