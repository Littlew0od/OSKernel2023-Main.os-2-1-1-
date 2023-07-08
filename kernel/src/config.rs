#![allow(unused)]

pub const USER_STACK_SIZE: usize = 4096 * 40;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 0x1000; // 20_0000
                                                        // pub const MEMORY_END: usize = 0x80800000;
pub const MEMORY_END: usize = 0x9000_0000;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MMIO};
pub const SYSTEM_FD_LIMIT: usize = 256;

// Define the underlying virtual addresses of mmap and stack

pub const STACK_TOP: usize = 0x1_0000_0000;
pub const MMAP_BASE: usize = 0x4000_0000;

//aux typ
pub const AT_NULL: usize = 0;
pub const AT_IGNORE: usize = 1;
pub const AT_EXECCFD: usize = 2;
pub const AT_PHDR: usize = 3;
pub const AT_PHENT: usize = 4;
pub const AT_PHNUM: usize = 5;
pub const AT_PAGESIZE: usize = 6;
pub const AT_BASE: usize = 7;
pub const AT_FLAGS: usize = 8;
pub const AT_ENTRY: usize = 9;
pub const AT_NOELF: usize = 10;
pub const AT_UID: usize = 11;
pub const AT_EUID: usize = 12;
pub const AT_GID: usize = 13;
pub const AT_EGID: usize = 14;
pub const AT_PLATFORM: usize = 15;
pub const AT_HWCAP: usize = 16;
pub const AT_CLKTCK: usize = 17;
pub const AT_SECURE: usize = 23;
pub const AT_BASE_PLATFORM: usize = 24;
pub const AT_RANDOM: usize = 25;
pub const AT_HWCAP2: usize = 26;
pub const AT_EXECFN: usize = 31;
pub const AT_SYSINFO: usize = 32;
pub const AT_SYSINFO_EHDR: usize = 33;
