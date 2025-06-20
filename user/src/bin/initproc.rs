#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    // we shouldn't use it to run test apps, initproc is just initproc
    println!("[initproc] starting");
    if fork() == 0 {
        exec("/test_shell\0", &[core::ptr::null::<u8>()]); // user_shell
    } else {
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            /*
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid,
                exit_code,
            );
            */
        }
    }
    0
}
