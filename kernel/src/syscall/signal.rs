use crate::{
    mm::{translated_ref, translated_refmut},
    syscall::{errno::EAGAIN, process},
    task::{
        current_process, current_task, current_user_token, pid2process,
        suspend_current_and_run_next, SigInfo, SignalAction, SignalFlags, MAX_SIG, SIG_BLOCK,
        SIG_SETMASK, SIG_UNBLOCK,
    },
    timer::TimeSpec,
};

use super::errno::{EPERM, SUCCESS};

pub fn sys_sigprocmask(
    how: usize,
    set: *mut usize,
    old_set: *mut usize,
    kernel_space: bool,
) -> isize {
    let token = current_user_token();
    let process = current_process();
    let mut inner = process.inner_exclusive_access();

    let mut mask = inner.signal_mask;

    if kernel_space {
        if old_set as usize != 0 {
            unsafe {
                *old_set = mask.bits();
            }
        }
    } else {
        if old_set as usize != 0 {
            *translated_refmut(token, old_set) = mask.bits();
        }
    }

    if set as usize != 0 {
        let set = *translated_ref(token, set);
        // tip!("[sys_sigprocmask] set = {:#b}, how = {}", set, how);
        let set_flags = SignalFlags::from_bits(set).unwrap();
        // if set_flags.contains(SignalFlags::SIGILL) {
        //     log!("[sys_sigprocmask] SignalFlags::SIGILL");
        // }
        match how {
            // SIG_BLOCK The set of blocked signals is the union of the current set and the set argument.
            SIG_BLOCK => mask |= set_flags,
            // SIG_UNBLOCK The signals in set are removed from the current set of blocked signals.
            SIG_UNBLOCK => mask &= !set_flags,
            // SIG_SETMASK The set of blocked signals is set to the argument set.
            SIG_SETMASK => mask = set_flags,
            _ => return EPERM,
        }
        inner.signal_mask = mask;
    }
    SUCCESS
}

pub fn sys_sigreturn() -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.handling_sig = -1;
    // restore the trap context
    let trap_ctx = inner.get_trap_cx();
    *trap_ctx = inner.trap_ctx_backup.unwrap();
    SUCCESS
}

fn check_sigaction_error(signal: SignalFlags) -> bool {
    if signal == SignalFlags::SIGKILL || signal == SignalFlags::SIGSTOP {
        true
    } else {
        false
    }
}

pub fn sys_sigaction(
    signum: usize,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    // tip!(
    //     "[sys_sigaction] signum = {:#x}, action = {:X}, old_action = {:X}",
    //     signum,
    //     action as usize,
    //     old_action as usize
    // );
    let token = current_user_token();
    let process = current_process();
    let mut inner_process = process.inner_exclusive_access();
    if signum > MAX_SIG {
        log!("[sys_sigaction] error signum");
        return EPERM;
    }
    if old_action as usize != 0 {
        *translated_refmut(token, old_action) = inner_process.signal_actions.table[signum].clone();
    }
    if let Some(flag) = SignalFlags::from_bits(1 << (signum - 1)) {
        if check_sigaction_error(flag) {
            log!("[sys_sigaction] check_sigaction_error");
            return EPERM;
        }
        let old_kernel_action = inner_process.signal_actions.table[signum];
        if old_action as usize != 0 {
            if old_kernel_action.mask != SignalFlags::from_bits(40).unwrap() {
                *translated_refmut(token, old_action) = old_kernel_action;
            } else {
                let mut ref_old_action = *translated_refmut(token, old_action);
                ref_old_action.sa_handler = old_kernel_action.sa_handler;
            }
        }
        if action as usize != 0 {
            let ref_action = translated_ref(token, action);
            inner_process.signal_actions.table[signum as usize] = *ref_action;
        }
        return SUCCESS;
    } else {
        println!("Undefined SignalFlags");
        return EPERM;
    }
}

// The timedwiat used in the libtest is different from the linux manual page
pub fn sys_sigtimedwait(
    uthese: *mut usize,
    info: *mut SigInfo,
    uts: *const TimeSpec,
    // I find sigsetsize in Linux 5.2 source code, but I dont know how to use it.
    sigsetsize: usize,
) -> isize {
    let token = current_user_token();
    if uthese as usize == 0 || uts as usize == 0 {
        println!("[sys_sigtimedwait] Null pointer.");
        return EPERM;
    }

    let timeout = *translated_ref(token, uts);
    let limit_time = TimeSpec::now() + timeout;

    let set = *translated_ref(token, uthese);
    let set_flags = SignalFlags::from_bits(set).unwrap();

    // log!(
    //     "[sys_sigtimedwait] uthese = {:?}, uts = {:?}, set = {}.",
    //     set_flags,
    //     uts,
    //     set
    // );

    loop {
        let process = current_process();
        let signals_pending = process.inner_exclusive_access().signals_pending;
        // Every matched signals will return. This method is wrong.
        let match_signals = set_flags & signals_pending;
        if !match_signals.is_empty() {
            let first_signals = match_signals.bits().trailing_zeros();
            if info as usize != 0 {
                let siginfo = SigInfo::new(first_signals as usize, 0, 0);
                *translated_refmut(token, info) = siginfo;
            }
            return SUCCESS;
        }
        if limit_time < TimeSpec::now() {
            println!("[sys_sigtimedwait] Timeout.");
            return EAGAIN;
        }
        drop(process);
        drop(signals_pending);
        suspend_current_and_run_next();
    }
}

pub fn sys_kill(pid: usize, signum: usize) -> isize {
    tip!("[sys_kill] Add siganl = {:?}.", signum);
    if let Some(process) = pid2process(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << (signum - 1)) {
            process.inner_exclusive_access().signals_pending |= flag;
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_tkill(tid: usize, signum: usize) -> isize {
    let signal = SignalFlags::from_bits(1 << (signum - 1)).unwrap();
    println!(
        "[sys_tkill] tid = {}, signum = {}, signal = {:?}",
        tid, signum, signal
    );
    SUCCESS
}
