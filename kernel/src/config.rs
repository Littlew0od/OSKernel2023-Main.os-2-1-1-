#![allow(unused)]

pub const USER_STACK_SIZE: usize = 4096 * 20;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
#[cfg(feature = "board_qemu")]
pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 0x500;
#[cfg(feature = "board_k210")]
pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 0xe0;
// pub const MEMORY_END: usize = 0x8800_0000;

#[cfg(feature = "board_qemu")]
pub const MEMORY_END: usize = 0x8800_0000;
// pub const MEMORY_END: usize = 0x8800_0000;
#[cfg(feature = "board_k210")]
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
pub const SECOND_MMAP_BASE: usize = 0x4000_0000;
pub const DYN_BASE: usize = 0x6000_0000;
pub const INTERRUPTS_FD: usize = 0x520;
pub const MAX_TRAP_ID: usize = 0xB;

pub const DISK_IMAGE_BASE: usize = 0x10_0000;
