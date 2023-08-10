#![allow(unused)]
mod address;
mod config;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

use address::VPNRange;
pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use config::*;
use core::arch::asm;
pub use frame_allocator::{frame_alloc, frame_alloc_arc, frame_dealloc, FrameTracker};
pub use heap_allocator::get_rest;
pub use memory_set::{
    kernel_token, remap_test, AuxHeader, MapPermission, MapType, MemorySet, KERNEL_SPACE,
    MPROCTECTPROT,
};
use page_table::PTEFlags;

pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

#[inline(always)]
pub fn tlb_invalidate() {
    unsafe {
        asm!("sfence.vma");
    }
}
