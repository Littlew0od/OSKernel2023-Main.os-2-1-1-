#![no_std]
#![no_main]
#![feature(int_roundings)]
#![feature(string_remove_matches)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
extern crate bitflags;

use riscv::register::mstatus::set_fs;
use riscv::register::sstatus::FS;

#[cfg(feature = "board_k210")]
#[path = "boards/k210.rs"]
mod board;
#[cfg(not(any(feature = "board_k210")))]
#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod config;
mod drivers;
mod fs;
mod lang_items;
mod mm;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_initial_apps.S"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

fn enable_float() {
    unsafe {
        set_fs(FS::Clean);
    };
}

#[no_mangle]
pub fn rust_main() -> ! {
    enable_float();
    clear_bss();
    println!("[kernel] Hello, world!");
    mm::init();
    mm::remap_test();
    trap::init();
    println!("[kernel] Finish trap init! ");
    trap::enable_timer_interrupt();
    println!("[kernel] Finish enable timer interrupt! ");
    // Avoid cluttered output, we can disable timer interrupt
    timer::set_next_trigger();
    println!("[kernel] Finish set trigger! ");
    fs::directory_tree::init_fs();
    println!("[kernel] Finish init fs! ");
    // fs::list_apps();
    // we embeded initproc process and shell process into kernel
    // we should load them into file system first
    task::load_initialproc();
    task::add_initproc();
    println!("[kernel] Finish add initproc! ");
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
