#![allow(unused)]

pub const USER_STACK_SIZE: usize = 4096 * 10;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 0xc0; // 20_0000
                                                        // pub const MEMORY_END: usize = 0x80800000;
// pub const MEMORY_END: usize = 0x8800_0000;
// pub const MEMORY_END: usize = 0x809e_0000;
pub const MEMORY_END: usize = 0x8080_0000;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const SIGNAL_TRAMPOLINE: usize = TRAMPOLINE - PAGE_SIZE;
pub const TRAP_CONTEXT_BASE: usize = SIGNAL_TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MMIO};
pub const SYSTEM_FD_LIMIT: usize = 256;

// Define the underlying virtual addresses of mmap and stack

pub const STACK_TOP: usize = 0x1_0000_0000;
pub const MMAP_BASE: usize = 0x2000_0000;
pub const DYN_BASE: usize = 0x6000_0000;

pub const DISK_IMAGE_BASE: usize = 0x10_0000;

