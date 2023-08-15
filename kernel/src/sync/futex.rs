#![allow(unused)]

use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use crate::{
    mm::{translated_ref, translated_refmut},
    syscall::errno::{EAGAIN, EINVAL, SUCCESS},
    task::{
        block_current_and_run_next, current_process, current_task, current_user_token,
        unblock_task, TaskControlBlock,
    },
    timer::TimeSpec,
};

use super::UPSafeCell;

pub const FUTEX_WAIT: usize = 0;
pub const FUTEX_WAKE: usize = 1;
pub const FUTEX_FD: usize = 2;
pub const FUTEX_REQUEUE: usize = 3;
pub const FUTEX_CMP_REQUEUE: usize = 4;
pub const FUTEX_WAKE_OP: usize = 5;
pub const FUTEX_LOCK_PI: usize = 6;
pub const FUTEX_UNLOCK_PI: usize = 7;
pub const FUTEX_TRYLOCK_PI: usize = 8;
pub const FUTEX_WAIT_BITSET: usize = 9;
pub const FUTEX_WAKE_BITSET: usize = 10;
pub const FUTEX_WAIT_REQUEUE_PI: usize = 11;
pub const FUTEX_CMP_REQUEUE_PI: usize = 12;

pub const FUTEX_PRIVATE_FLAG: usize = 128;
pub const FUTEX_CLOCK_REALTIME: usize = 256;
pub const FUTEX_CMD_MASK: usize = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

pub struct Futex {
    pub inner: BTreeMap<usize, FutexInner>,
}

pub struct FutexInner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl FutexInner {
    pub fn new() -> Self {
        Self {
            wait_queue: VecDeque::new(),
        }
    }
}

impl Futex {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }
}

pub fn futex_wait(uaddr: *mut u32, timeout: *const TimeSpec, val: u32) -> isize {
    let token = current_user_token();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let timeout = if timeout.is_null() {
        None
    } else {
        Some(*translated_ref(token, timeout) + TimeSpec::now())
    };

    // ? &
    // let futex = inner.futex;

    let futex_word = *translated_ref(token, uaddr);
    
    if futex_word != val {
        tip!(
            "[futex_wait] Futex and val do not match. futex_word = {:#x}, val = {}",
            futex_word,
            val
        );
        return EAGAIN;
    } else {
        let thread = current_task().unwrap();

        if let Some(inner) = process_inner.futex.inner.get_mut(&(uaddr as usize)) {
            inner.wait_queue.push_front(thread.clone());
        } else {
            let mut deuqe = VecDeque::new();
            deuqe.push_front(thread.clone());
            let futex_inner = FutexInner { wait_queue: deuqe };
            process_inner.futex.inner.insert(uaddr as usize, futex_inner);
        }
        drop(process_inner);
        drop(process);
        drop(thread);
        println!("[futex_wait] block_current_and_run_next");
        block_current_and_run_next();
    }
    SUCCESS
}
pub fn futex_signal(uaddr: *mut u32, val: u32) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    for cnt in 0..val {
        if let Some(inner) = inner.futex.inner.get_mut(&(uaddr as usize)) {
            if let Some(thread) = inner.wait_queue.pop_front() {
                unblock_task(thread);
            } else {
                return (cnt + 1) as isize;
            }
        } else {
            return EINVAL;
        }
    }
    val as isize
}
