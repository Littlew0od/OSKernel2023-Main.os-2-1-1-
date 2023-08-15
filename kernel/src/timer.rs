#![allow(unused)]
use core::cmp::Ordering;
use core::ops::{Add, AddAssign, Sub};

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use crate::sync::UPSafeCell;
use crate::task::{wakeup_task, TaskControlBlock};
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use lazy_static::*;
use riscv::register::time;

// pub const TICKS_PER_SEC: usize = 25;
const TICKS_PER_SEC: usize = 100;

pub const MSEC_PER_SEC: usize = 1000;

pub const USEC_PER_SEC: usize = 1_000_000;
pub const USEC_PER_MSEC: usize = 1_000;

pub const NSEC_PER_SEC: usize = 1_000_000_000;
pub const NSEC_PER_MSEC: usize = 1_000_000;
pub const NSEC_PER_USEC: usize = 1_000;

/*
 * The IDs of the various system clocks (for POSIX.1b interval timers):
 */
pub const CLOCK_REALTIME: usize = 0;
pub const CLOCK_MONOTONIC: usize = 1;
pub const CLOCK_PROCESS_CPUTIME_ID: usize = 2;
pub const CLOCK_THREAD_CPUTIME_ID: usize = 3;
pub const CLOCK_MONOTONIC_RAW: usize = 4;
pub const CLOCK_REALTIME_COARSE: usize = 5;
pub const CLOCK_MONOTONIC_COARSE: usize = 6;
pub const CLOCK_BOOTTIME: usize = 7;
pub const CLOCK_REALTIME_ALARM: usize = 8;
pub const CLOCK_BOOTTIME_ALARM: usize = 9;

/// Return current time measured by ticks, which is NOT divided by frequency.
pub fn get_time() -> usize {
    time::read()
}
/// Return current time measured by seconds.
pub fn get_time_sec() -> usize {
    let i = time::read() / (CLOCK_FREQ);
    //log::info!("[timer.rs] time::read(): {},sec: {}", time::read(), i);
    i
}
/// Return current time measured by ms.
pub fn get_time_ms() -> usize {
    let i = time::read() / (CLOCK_FREQ / MSEC_PER_SEC);
    //log::info!("[timer.rs] time::read(): {},ms: {}", time::read(), i);
    i
}
/// Return current time measured by us.
pub fn get_time_us() -> usize {
    let i = time::read() / (CLOCK_FREQ / USEC_PER_SEC);
    //log::info!("[timer.rs] time::read(): {},us: {}", time::read(), i);
    i
}
/// Return current time measured by nano seconds.
pub fn get_time_ns() -> usize {
    let i = time::read() * NSEC_PER_SEC / (CLOCK_FREQ);
    //log::info!("[timer.rs] time::read(): {},ns: {}", time::read(), i);
    i
}

/// Set next trigger.
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Traditional UNIX timespec structures represent elapsed time, measured by the system clock
/// # *CAUTION*
/// tv_sec & tv_usec should be usize.
pub struct TimeSpec {
    /// The tv_sec member represents the elapsed time, in whole seconds.
    pub tv_sec: usize,
    /// The tv_usec member captures rest of the elapsed time, represented as the number of microseconds.
    pub tv_nsec: usize,
}
impl AddAssign for TimeSpec {
    fn add_assign(&mut self, rhs: Self) {
        self.tv_sec += rhs.tv_sec;
        self.tv_nsec += rhs.tv_nsec;
    }
}
impl Add for TimeSpec {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut sec = self.tv_sec + other.tv_sec;
        let mut nsec = self.tv_nsec + other.tv_nsec;
        sec += nsec / NSEC_PER_SEC;
        nsec %= NSEC_PER_SEC;
        Self {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }
}

impl Sub for TimeSpec {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let self_ns = self.to_ns();
        let other_ns = other.to_ns();
        if self_ns <= other_ns {
            TimeSpec::new()
        } else {
            TimeSpec::from_ns(self_ns - other_ns)
        }
    }
}

impl Ord for TimeSpec {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.tv_sec.cmp(&other.tv_sec) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.tv_nsec.cmp(&other.tv_nsec),
            Ordering::Greater => Ordering::Greater,
        }
    }
}

impl PartialOrd for TimeSpec {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TimeSpec {
    pub fn new() -> Self {
        Self {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }
    pub fn from_tick(tick: usize) -> Self {
        Self {
            tv_sec: tick / CLOCK_FREQ,
            tv_nsec: (tick % CLOCK_FREQ) * NSEC_PER_SEC / CLOCK_FREQ,
        }
    }
    pub fn from_s(s: usize) -> Self {
        Self {
            tv_sec: s,
            tv_nsec: 0,
        }
    }
    pub fn from_ms(ms: usize) -> Self {
        Self {
            tv_sec: ms / MSEC_PER_SEC,
            tv_nsec: (ms % MSEC_PER_SEC) * NSEC_PER_MSEC,
        }
    }
    pub fn from_us(us: usize) -> Self {
        Self {
            tv_sec: us / USEC_PER_SEC,
            tv_nsec: (us % USEC_PER_SEC) * NSEC_PER_USEC,
        }
    }
    pub fn from_ns(ns: usize) -> Self {
        Self {
            tv_sec: ns / NSEC_PER_SEC,
            tv_nsec: ns % NSEC_PER_SEC,
        }
    }
    pub fn to_ns(&self) -> usize {
        self.tv_sec * NSEC_PER_SEC + self.tv_nsec
    }
    pub fn is_zero(&self) -> bool {
        self.tv_sec == 0 && self.tv_nsec == 0
    }
    pub fn now() -> Self {
        TimeSpec::from_tick(get_time())
    }
}

/// Traditional UNIX timeval structures represent elapsed time, measured by the system clock
/// # *CAUTION*
/// tv_sec & tv_usec should be usize.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeVal {
    /// The `tv_sec` member represents the elapsed time, in whole seconds
    pub tv_sec: usize,
    /// The `tv_nsec` member represents the rest of the elapsed time in nanoseconds.
    pub tv_usec: usize,
}

impl TimeVal {
    pub fn new() -> Self {
        Self {
            tv_sec: 0,
            tv_usec: 0,
        }
    }
    pub fn from_tick(tick: usize) -> Self {
        Self {
            tv_sec: tick / CLOCK_FREQ,
            tv_usec: (tick % CLOCK_FREQ) * USEC_PER_SEC / CLOCK_FREQ,
        }
    }
    pub fn to_tick(&self) -> usize {
        self.tv_sec * CLOCK_FREQ + self.tv_usec * CLOCK_FREQ / USEC_PER_SEC
    }
    pub fn from_s(s: usize) -> Self {
        Self {
            tv_sec: s,
            tv_usec: 0,
        }
    }
    pub fn from_ms(ms: usize) -> Self {
        Self {
            tv_sec: ms / MSEC_PER_SEC,
            tv_usec: (ms % MSEC_PER_SEC) * USEC_PER_MSEC,
        }
    }
    pub fn from_us(us: usize) -> Self {
        Self {
            tv_sec: us / USEC_PER_SEC,
            tv_usec: us % USEC_PER_SEC,
        }
    }
    pub fn to_us(&self) -> usize {
        self.tv_sec * USEC_PER_SEC + self.tv_usec
    }
    pub fn is_zero(&self) -> bool {
        self.tv_sec == 0 && self.tv_usec == 0
    }
    pub fn now() -> Self {
        TimeVal::from_tick(get_time())
    }
}

impl Add for TimeVal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut sec = self.tv_sec + other.tv_sec;
        let mut usec = self.tv_usec + other.tv_usec;
        sec += usec / USEC_PER_SEC;
        usec %= USEC_PER_SEC;
        Self {
            tv_sec: sec,
            tv_usec: usec,
        }
    }
}

impl Sub for TimeVal {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let self_us = self.to_us();
        let other_us = other.to_us();
        if self_us <= other_us {
            TimeVal::new()
        } else {
            TimeVal::from_us(self_us - other_us)
        }
    }
}

impl Ord for TimeVal {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.tv_sec.cmp(&other.tv_sec) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.tv_usec.cmp(&other.tv_usec),
            Ordering::Greater => Ordering::Greater,
        }
    }
}

impl PartialOrd for TimeVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct TimeZone {
    pub tz_minuteswest: u32,
    pub tz_dsttime: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ITimerVal {
    pub it_interval: TimeVal,
    pub it_value: TimeVal,
}
impl ITimerVal {
    pub fn new() -> Self {
        Self {
            it_interval: TimeVal::new(),
            it_value: TimeVal::new(),
        }
    }
}

#[derive(Clone, Copy)]
/// Store the current process times used in the `time()`.
pub struct Times {
    /// user time
    pub tms_utime: usize,
    /// system time
    pub tms_stime: usize,
    /// user time of children
    pub tms_cutime: usize,
    /// system time of children
    pub tms_cstime: usize,
}

impl Times {
    pub fn new() -> Self {
        Self {
            tms_utime: 0,
            tms_stime: 0,
            tms_cutime: 0,
            tms_cstime: 0,
        }
    }
    pub fn create(
        tms_utime: usize,
        tms_stime: usize,
        tms_cutime: usize,
        tms_cstime: usize,
    ) -> Self {
        Self {
            tms_utime,
            tms_stime,
            tms_cutime,
            tms_cstime,
        }
    }
}

impl Add for Times {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut tms_utime = self.tms_utime + other.tms_utime;
        let mut tms_stime = self.tms_stime + other.tms_stime;
        let mut tms_cutime = self.tms_cutime + other.tms_cutime;
        let mut tms_cstime = self.tms_cstime + other.tms_cstime;
        Self {
            tms_utime,
            tms_stime,
            tms_cutime,
            tms_cstime,
        }
    }
}

impl Sub for Times {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.tms_utime < other.tms_utime
            || self.tms_stime < other.tms_stime
            || self.tms_cutime < other.tms_cutime
            || self.tms_cstime < other.tms_cstime
        {
            return Times::new();
        }
        let mut tms_utime = self.tms_utime + other.tms_utime;
        let mut tms_stime = self.tms_stime + other.tms_stime;
        let mut tms_cutime = self.tms_cutime + other.tms_cutime;
        let mut tms_cstime = self.tms_cstime + other.tms_cstime;
        Self {
            tms_utime,
            tms_stime,
            tms_cutime,
            tms_cstime,
        }
    }
}

pub enum TimeRange {
    TimeSpec(TimeSpec),
    TimeVal(TimeVal),
}

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub task: Arc<TaskControlBlock>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = -(self.expire_ms as isize);
        let b = -(other.expire_ms as isize);
        Some(a.cmp(&b))
    }
}

impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    static ref TIMERS: UPSafeCell<BinaryHeap<TimerCondVar>> =
        unsafe { UPSafeCell::new(BinaryHeap::<TimerCondVar>::new()) };
}

pub fn add_timer(expire_ms: usize, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    timers.push(TimerCondVar { expire_ms, task });
}

pub fn remove_timer(task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    let mut temp = BinaryHeap::<TimerCondVar>::new();
    for condvar in timers.drain() {
        if Arc::as_ptr(&task) != Arc::as_ptr(&condvar.task) {
            temp.push(condvar);
        }
    }
    timers.clear();
    timers.append(&mut temp);
}

pub fn check_timer() {
    let current_ms = get_time_ms();
    let mut timers = TIMERS.exclusive_access();
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= current_ms {
            wakeup_task(Arc::clone(&timer.task));
            timers.pop();
        } else {
            break;
        }
    }
}
