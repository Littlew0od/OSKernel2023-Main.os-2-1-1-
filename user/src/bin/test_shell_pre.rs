#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::string::String;
use user_lib::{exec, fork, shutdown, waitpid};

#[no_mangle]
fn main() -> i32 {
    let tasks = [
        // "close\0",
        // "execve\0",  
        // "fstat\0",     
        // "getpid\0",        
        // "mkdir_\0",  
        // "umount\0",
        // "chdir\0",
        // "dup\0",
        // "getcwd\0",    
        // "getppid\0",        
        // "open\0",    
        // "read\0",   
        // "uname\0",     
        // "getdents\0",  
        // "gettimeofday\0",  
        // "mount\0",   
        // "openat\0",  
        // "sleep\0",  
        // "times\0",      
        // "unlink\0",  
        // "write\0",  
        // "mmap\0",
        // "brk\0",
        "busybox\0",
        // "dup2\0",
        // "wait\0",  

        // "waitpid\0",
        // "clone\0",     
        // "fork\0",  
        // "exit\0",  
           
        // "yield\0",  
        // "pipe\0",  
    ];
    // If you want to test locally, add the following commented out paths
    let mut path = String::from("/"); //  bin/riscv-syscalls-testing/
    let arr: [*const u8; 4] = [
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
    ];
    let mut exit_code: i32 = 0;
    for path_name in tasks {
        let pid = fork();
        // The program is guaranteed not to execute in parallel
        if pid == 0 {
            path.push_str(path_name);
            println!("[test_shell] path = {}", path);
            exec(path.as_str(), &arr[..]);
        } else {
            waitpid(pid as usize, &mut exit_code);
        }
    }
    // shutdown after test
    shutdown(false);
}