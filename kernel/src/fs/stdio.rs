#![allow(unused)]
use super::{DiskInodeType, File};
use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;
use crate::{mm::UserBuffer, syscall::errno::ENOTDIR};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

#[derive(Copy, Clone)]
pub struct Stdin;

#[derive(Copy, Clone)]
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    // fn read(&self, offset: Option<&mut usize>, mut user_buf: UserBuffer) -> usize {
    //     assert_eq!(user_buf.len(), 1);
    //     // busy loop
    //     let mut c: usize;
    //     loop {
    //         c = console_getchar();
    //         if c == 0 {
    //             suspend_current_and_run_next();
    //             continue;
    //         } else {
    //             break;
    //         }
    //     }
    //     let ch = c as u8;
    //     unsafe {
    //         user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
    //     }
    //     1
    // }
    // fn write(&self, _user_buf: UserBuffer) -> usize {
    //     panic!("Cannot write to stdin!");
    // }

    fn deep_clone(&self) -> Arc<dyn File> {
        todo!()
    }

    // fn readable(&self) -> bool {
    //     todo!()
    // }

    // fn writable(&self) -> bool {
    //     todo!()
    // }

    fn read(&self, offset: Option<&mut usize>, buf: &mut [u8]) -> usize {
        todo!()
    }

    fn read_all(&self) -> Vec<u8> {
        Vec::new()
    }

    fn write(&self, offset: Option<&mut usize>, buf: &[u8]) -> usize {
        todo!()
    }

    fn r_ready(&self) -> bool {
        todo!()
    }

    fn w_ready(&self) -> bool {
        todo!()
    }

    fn read_user(&self, offset: Option<usize>, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        let ch = c as u8;
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }

    fn write_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }

    fn get_size(&self) -> usize {
        todo!()
    }

    fn get_stat(&self) -> crate::fs::Stat {
        todo!()
    }

    fn get_file_type(&self) -> DiskInodeType {
        todo!()
    }

    fn info_dirtree_node(&self, dirnode_ptr: Weak<crate::fs::directory_tree::DirectoryTreeNode>) {
        todo!()
    }

    fn get_dirtree_node(&self) -> Option<Arc<crate::fs::directory_tree::DirectoryTreeNode>> {
        todo!()
    }

    fn open(&self, flags: crate::fs::OpenFlags, special_use: bool) -> Arc<dyn File> {
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

    fn get_dirent(&self, count: usize) -> alloc::vec::Vec<crate::fs::Dirent> {
        todo!()
    }

    fn lseek(&self, offset: isize, whence: crate::fs::SeekWhence) -> Result<usize, isize> {
        todo!()
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

    fn get_single_cache(
        &self,
        offset: usize,
    ) -> Result<Arc<spin::Mutex<crate::fs::PageCache>>, ()> {
        todo!()
    }

    fn get_all_caches(
        &self,
    ) -> Result<alloc::vec::Vec<Arc<spin::Mutex<crate::fs::PageCache>>>, ()> {
        todo!()
    }

    fn oom(&self) -> usize {
        todo!()
    }

    fn hang_up(&self) -> bool {
        todo!()
    }

    fn fcntl(&self, cmd: u32, arg: u32) -> isize {
        todo!()
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    // fn read(&self, _user_buf: UserBuffer) -> usize {
    //     panic!("Cannot read from stdout!");
    // }
    // fn write(&self, user_buf: UserBuffer) -> usize {
    //     for buffer in user_buf.buffers.iter() {
    //         print!("{}", core::str::from_utf8(*buffer).unwrap());
    //     }
    //     user_buf.len()
    // }
    fn deep_clone(&self) -> Arc<dyn File> {
        todo!()
    }

    // fn readable(&self) -> bool {
    //     todo!()
    // }

    // fn writable(&self) -> bool {
    //     todo!()
    // }

    fn read(&self, offset: Option<&mut usize>, buf: &mut [u8]) -> usize {
        todo!()
    }

    fn read_all(&self) -> Vec<u8> {
        Vec::new()
    }

    fn write(&self, offset: Option<&mut usize>, buf: &[u8]) -> usize {
        todo!()
    }

    fn r_ready(&self) -> bool {
        todo!()
    }

    fn w_ready(&self) -> bool {
        todo!()
    }

    fn read_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }

    fn write_user(&self, offset: Option<usize>, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }

    fn get_size(&self) -> usize {
        todo!()
    }

    fn get_stat(&self) -> crate::fs::Stat {
        todo!()
    }

    fn get_file_type(&self) -> DiskInodeType {
        todo!()
    }

    fn info_dirtree_node(&self, dirnode_ptr: Weak<crate::fs::directory_tree::DirectoryTreeNode>) {
        todo!()
    }

    fn get_dirtree_node(&self) -> Option<Arc<crate::fs::directory_tree::DirectoryTreeNode>> {
        todo!()
    }

    fn open(&self, flags: crate::fs::OpenFlags, special_use: bool) -> Arc<dyn File> {
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

    fn get_dirent(&self, count: usize) -> alloc::vec::Vec<crate::fs::Dirent> {
        todo!()
    }

    fn lseek(&self, offset: isize, whence: crate::fs::SeekWhence) -> Result<usize, isize> {
        todo!()
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

    fn get_single_cache(
        &self,
        offset: usize,
    ) -> Result<Arc<spin::Mutex<crate::fs::PageCache>>, ()> {
        todo!()
    }

    fn get_all_caches(
        &self,
    ) -> Result<alloc::vec::Vec<Arc<spin::Mutex<crate::fs::PageCache>>>, ()> {
        todo!()
    }

    fn oom(&self) -> usize {
        todo!()
    }

    fn hang_up(&self) -> bool {
        todo!()
    }

    fn fcntl(&self, cmd: u32, arg: u32) -> isize {
        todo!()
    }
}
