#[allow(unused)]

pub const USER_STACK_SIZE: usize = 4096 * 5;
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

#[macro_export]
macro_rules! const_def {
    ($ident:ident, $val:expr) => {
        pub const $ident: usize = $val;
    };
}

//aux typ
const_def!(AT_NULL, 0);
const_def!(AT_IGNORE, 1);
const_def!(AT_EXECCFD, 2);
const_def!(AT_PHDR, 3);
const_def!(AT_PHENT, 4);
const_def!(AT_PHNUM, 5);
const_def!(AT_PAGESIZE, 6);
const_def!(AT_BASE, 7);
const_def!(AT_FLAGS, 8);
const_def!(AT_ENTRY, 9);
const_def!(AT_NOELF, 10);
const_def!(AT_UID, 11);
const_def!(AT_EUID, 12);
const_def!(AT_GID, 13);
const_def!(AT_EGID, 14);
const_def!(AT_PLATFORM, 15);
const_def!(AT_HWCAP, 16);
const_def!(AT_CLKTCK, 17);
const_def!(AT_SECURE, 23);
const_def!(AT_BASE_PLATFORM, 24);
const_def!(AT_RANDOM, 25);
const_def!(AT_HWCAP2, 26);
const_def!(AT_EXECFN, 31);
const_def!(AT_SYSINFO, 32);
const_def!(AT_SYSINFO_EHDR, 33);
