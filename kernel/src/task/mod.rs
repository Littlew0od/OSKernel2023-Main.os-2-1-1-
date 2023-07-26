#![allow(unused)]
mod action;
mod context;
mod id;
mod manager;
mod process;
mod processor;
mod signal;
mod switch;
mod futex;
#[allow(clippy::module_inception)]
mod task;

use self::id::TaskUserRes;
use self::manager::{block_task, unblock_task};
use crate::fs::{OpenFlags, ROOT_FD}; // open_file,
use crate::sbi::shutdown;
use crate::timer::remove_timer;
use alloc::{sync::Arc, vec::Vec};
use lazy_static::*;
use manager::fetch_task;
use process::ProcessControlBlock;
use switch::__switch;

pub use action::{SignalAction, SignalActions};
pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle, IDLE_PID};
pub use manager::{add_task, pid2process, remove_from_pid2process, remove_task, wakeup_task};
pub use process::{CloneFlags, Flags, CSIGNAL};
pub use processor::{
    current_kstack_top, current_process, current_task, current_trap_cx, current_trap_cx_user_va,
    current_user_token, run_tasks, schedule, take_current_task,
};
pub use signal::{SigInfo, SignalFlags, MAX_SIG, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK};
pub use task::{TaskControlBlock, TaskStatus};
pub use futex::*;

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current TCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

pub fn block_current_and_run_next() {
    // pop task so that we can push it in block queue
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Blocked;
    block_task(task.clone());

    drop(task_inner);
    schedule(task_cx_ptr);
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let process = task.process.upgrade().unwrap();
    // send exit signals.
    // The exit_signal is never empty
    let exit_signal = process.inner_exclusive_access().exit_signal;
    if !exit_signal.is_empty() {
        let mut process_inner = process.inner_exclusive_access();

        let parent_process = process_inner.parent.as_ref().unwrap().upgrade().unwrap();
        let mut parent_process_inner = parent_process.inner_exclusive_access();

        parent_process_inner.signals_pending |= exit_signal;
        let parent_task = parent_process_inner.get_task(0);
        if parent_task.inner_exclusive_access().task_status == TaskStatus::Blocked {
            unblock_task(parent_task.clone());
        }
    } else {
        log!("[exit_current_and_run_next] Empty exit_signal!");
    }

    let tid = task_inner.res.as_ref().unwrap().tid;
    // record exit code
    task_inner.exit_code = Some(exit_code);
    task_inner.res = None;
    // here we do not remove the thread since we are still using the kstack
    // it will be deallocated when sys_waittid is called
    drop(task_inner);
    drop(task);
    // however, if this is the main thread of current process
    // the process should terminate at once
    if tid == 0 {
        let pid = process.getpid();
        if pid == IDLE_PID {
            println!(
                "[kernel] Idle process exit with exit_code {} ...",
                exit_code
            );
            if exit_code != 0 {
                //crate::sbi::shutdown(255); //255 == -1 for err hint
                shutdown(true);
            } else {
                //crate::sbi::shutdown(0); //0 for success hint
                shutdown(false);
            }
        }
        remove_from_pid2process(pid);
        let mut process_inner = process.inner_exclusive_access();
        // mark this process as a zombie process
        process_inner.is_zombie = true;
        // record exit code of main process
        process_inner.exit_code = exit_code;

        {
            // move all child processes under init process
            let mut initproc_inner = INITPROC.inner_exclusive_access();
            for child in process_inner.children.iter() {
                child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
                initproc_inner.children.push(child.clone());
            }
        }

        // deallocate user res (including tid/trap_cx/ustack) of all threads
        // it has to be done before we dealloc the whole memory_set
        // otherwise they will be deallocated twice
        let mut recycle_res = Vec::<TaskUserRes>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            // if other tasks are Ready in TaskManager or waiting for a timer to be
            // expired, we should remove them.
            //
            // Mention that we do not need to consider Mutex/Semaphore since they
            // are limited in a single process. Therefore, the blocked tasks are
            // removed when the PCB is deallocated.
            remove_inactive_task(Arc::clone(&task));
            let mut task_inner = task.inner_exclusive_access();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }
        // dealloc_tid and dealloc_user_res require access to PCB inner, so we
        // need to collect those user res first, then release process_inner
        // for now to avoid deadlock/double borrow problem.
        drop(process_inner);
        recycle_res.clear();

        let mut process_inner = process.inner_exclusive_access();
        process_inner.children.clear();
        // deallocate other data in user space i.e. program code/data section
        process_inner.memory_set.recycle_data_pages();
        // drop file descriptors
        process_inner.fd_table.lock().clear_inner();
        // remove all tasks
        process_inner.tasks.clear();
    }
    drop(process);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = {
        let initproc_fd = ROOT_FD.open("/initproc", OpenFlags::O_RDONLY, true).unwrap();
        // let initproc_fd = ROOT_FD.open("/initprocfortest", OpenFlags::O_RDONLY, true).unwrap();
        let v = initproc_fd.read_all();
        ProcessControlBlock::new(v.as_slice())
    };
}

pub fn load_initialproc() {
    // These global variables are defined in link_initial_apps.S
    extern "C" {
        fn app_0_start();
        fn app_0_end();
        fn app_1_start();
        fn app_1_end();
    }
    // let initprocfortest = ROOT_FD.open("initprocfortest", OpenFlags::O_CREAT, false).unwrap();
    // initprocfortest.write(None, unsafe {
    //     core::slice::from_raw_parts(
    //         app_0_start as *const u8,
    //         app_0_end as usize - app_0_start as usize,
    //     )
    // });
    let initproc = ROOT_FD.open("initproc", OpenFlags::O_CREAT, false).unwrap();
    initproc.write(None, unsafe {
        core::slice::from_raw_parts(
            app_0_start as *const u8,
            app_0_end as usize - app_0_start as usize,
        )
    });
    let test_shell = ROOT_FD
        .open("test_shell", OpenFlags::O_CREAT, false)
        .unwrap();
    test_shell.write(None, unsafe {
        core::slice::from_raw_parts(
            app_1_start as *const u8,
            app_1_end as usize - app_1_start as usize,
        )
    });
}

pub fn add_initproc() {
    let _initproc = INITPROC.clone();
}

pub fn check_signals_of_current() -> Option<(i32, &'static str)> {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    process_inner.signals_pending.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.signals_pending |= signal;
}

pub fn remove_inactive_task(task: Arc<TaskControlBlock>) {
    remove_task(Arc::clone(&task));
    remove_timer(Arc::clone(&task));
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    // tip!("[call_kernel_signal_handler]");
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            task_inner.frozen = true;
            process_inner.signals_pending ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if process_inner.signals_pending.contains(SignalFlags::SIGCONT) {
                process_inner.signals_pending ^= SignalFlags::SIGCONT;
                task_inner.frozen = false;
            }
        }
        _ => {
            task_inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    // tip!("[call_user_signal_handler]");
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let handler = process_inner.signal_actions.table[sig].sa_handler;
    if handler != 0 {
        // change current mask
        process_inner.signal_mask = process_inner.signal_actions.table[sig].mask;
        // handle flag
        task_inner.handling_sig = sig as isize;
        process_inner.signals_pending ^= signal;

        // backup trapframe
        let mut trap_ctx = task_inner.get_trap_cx();
        task_inner.trap_ctx_backup = Some(*trap_ctx);

        // modify trapframe
        trap_ctx.sepc = handler;

        // put args (a0)
        trap_ctx.x[10] = sig;
    }
}
fn check_pending_signals() {
    for sig in 0..(MAX_SIG + 1) {
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if process_inner.signals_pending.contains(signal)
            && (!process_inner.signal_mask.contains(signal))
        {
            if task_inner.handling_sig == -1 {
                drop(task_inner);
                drop(task);
                drop(process_inner);
                drop(process);
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    return;
                }
            } else {
                if !process_inner.signal_actions.table[task_inner.handling_sig as usize]
                    .mask
                    .contains(signal)
                {
                    drop(task_inner);
                    drop(task);
                    drop(process_inner);
                    drop(process);
                    if signal == SignalFlags::SIGKILL
                        || signal == SignalFlags::SIGSTOP
                        || signal == SignalFlags::SIGCONT
                    {
                        // signal is a kernel signal
                        call_kernel_signal_handler(signal);
                    } else {
                        // signal is a user signal
                        call_user_signal_handler(sig, signal);
                        return;
                    }
                }
            }
        }
    }
}

pub fn handle_signals() {
    check_pending_signals();
    loop {
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        let frozen_flag = task_inner.frozen;
        let killed_flag = task_inner.killed;
        drop(task_inner);
        drop(task);
        if (!frozen_flag) || killed_flag {
            break;
        }
        check_pending_signals();
        suspend_current_and_run_next()
    }
}
