use super::{PhysAddr, PhysPageNum};
use crate::sync::UPSafeCell;
use crate::{config::MEMORY_END, fs};
use alloc::{sync::Arc, vec::Vec};
use core::fmt::{self, Debug, Formatter};
use core::result;
use lazy_static::*;

pub struct FrameTracker {
    pub ppn: PhysPageNum,
    pub hold: bool,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn, hold: true }
    }
    pub fn cover(ppn: PhysPageNum) -> Self {
        Self { ppn, hold: false }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        if self.hold {
            frame_dealloc(self.ppn);
        }
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        println!("last {} Physical Frames, memory = {:#X}000", self.end - self.current, self.end);
    }
    pub fn unallocated_frames(&self) -> usize {
        self.recycled.len() + self.end - self.current
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.current == self.end {
            None
        } else {
            self.current += 1;
            Some((self.current - 1).into())
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // validity check
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

pub fn frame_reserve(num: usize) {
    let remain = FRAME_ALLOCATOR.exclusive_access().unallocated_frames();
    if remain < num {
        panic!("[frame_reserve] failed");
        // oom_handler(num - remain).unwrap()
    }
}

#[cfg(feature = "board_qemu")]
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

#[cfg(feature = "board_k210")]
pub fn frame_alloc() -> Option<FrameTracker> {
    let result = FRAME_ALLOCATOR.exclusive_access().alloc();
    if let Some(frame) = result {
        return Some(FrameTracker::new(frame));
    }
    drop(result);
    oom_handler(1).unwrap();
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

pub fn frame_alloc_arc() -> Option<Arc<FrameTracker>> {
    let result = FRAME_ALLOCATOR.exclusive_access().alloc();
    if let Some(frame) = result {
        return Some(Arc::new(FrameTracker::new(frame)));
    }
    drop(result);
    oom_handler(1).unwrap();
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
        .map(Arc::new)
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}

pub fn oom_handler(req: usize) -> Result<(), ()> {
    // clean fs
    // println!("[oom_handler] start");
    // show_unallocated_frames();
    let mut released = 0;
    released += fs::directory_tree::oom();
    // show_unallocated_frames();
    if released >= req {
        return Ok(());
    }
    println!("[oom_handler] fail");
    Err(())
}

pub fn show_unallocated_frames() {
    println!(
        "unallocated frames = {}",
        FRAME_ALLOCATOR.exclusive_access().unallocated_frames()
    );
}
