use alloc::boxed::Box;

use crate::{
    mm::translated_refmut,
    task::{
        current_process, current_user_token, suspend_current_and_run_next, SignalFlags, SIG_SETMASK,
    },
    timer::TimeSpec,
};

use super::signal::sys_sigprocmask;


///  A scheduling  scheme  whereby  the  local  process  periodically  checks  until  the  pre-specified events (for example, read, write) have occurred.
/// The PollFd struct in 32-bit style.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PollFd {
    /// File descriptor
    fd: u32,
    /// Requested events
    events: PollEvent,
    /// Returned events
    revents: PollEvent,
}

bitflags! {
    /// Event types that can be polled for.
    ///
    /// These bits may be set in `events`(see `ppoll()`) to indicate the interesting event types;
    ///
    /// they will appear in `revents` to indicate the status of the file descriptor.
    struct PollEvent:u16 {
    /// There is data to read.
    const POLLIN = 0x001;
    /// There is urgent data to read.
    const POLLPRI = 0x002;
    /// Writing now will not block.
    const POLLOUT = 0x004;

    /* Event types always implicitly polled for.
    These bits need not be set in `events',
    but they will appear in `revents' to indicate the status of the file descriptor.*/

    /// Implicitly polled for only.
    /// Error condition.
    const POLLERR = 0x008;
    /// Implicitly polled for only.
    /// Hung up.
    const POLLHUP = 0x010;
    /// Implicitly polled for only.
    /// Invalid polling request.
    const POLLNVAL = 0x020;
    }
}

/// `ppoll(&fds, nfds, tmo_p, &sigmask);`
/// is equal to
/// `{
///     pthread_sigmask(SIG_SETMASK, &sigmask, &origmask);
///     ready = poll(&fds, nfds, timeout);
///     pthread_sigmask(SIG_SETMASK, &origmask, NULL);
/// }`
///
/// Timeout is not yet supported.
pub fn sys_ppoll(
    fds: *mut PollFd,
    nfds: usize,
    tmo_p: *const TimeSpec,
    sigmask: *const SignalFlags,
) -> isize {
    let token = current_user_token();
    // log!("[sys_ppoll] nfds = {}", nfds);
    // oldsig in kernel space
    let oldsig = Box::new(SignalFlags::empty());
    let raw_ptr = Box::into_raw(oldsig);
    if !sigmask.is_null() {
        sys_sigprocmask(SIG_SETMASK, sigmask as *mut usize, raw_ptr as *mut usize, true);
    }
    if tmo_p as usize != 0 {
        println!("[sys_ppoll] Time limited maybe is needed!")
    }
    let mut done = 0;
    loop {
        let process = current_process();
        let inner = process.inner_exclusive_access();
        let fd_table = inner.fd_table.lock();
        for i in 0..nfds {
            let poll_fd = translated_refmut(token, unsafe { fds.add(i) });
            let fd = poll_fd.fd as usize;
            match fd_table.get_ref(fd) {
                Ok(file_descriptor) => {
                    let mut trigger = 0;
                    if file_descriptor.file.hang_up() {
                        poll_fd.revents |= PollEvent::POLLHUP;
                        trigger = 1;
                    }
                    if poll_fd.events.contains(PollEvent::POLLIN) && file_descriptor.file.r_ready()
                    {
                        poll_fd.revents |= PollEvent::POLLIN;
                        trigger = 1;
                    }
                    if poll_fd.events.contains(PollEvent::POLLOUT) && file_descriptor.file.w_ready()
                    {
                        poll_fd.revents |= PollEvent::POLLOUT;
                        trigger = 1;
                    }
                    done += trigger;
                }
                Err(_) => continue,
            }
        }
        if done > 0 {
            break;
        }
        drop(fd_table);
        drop(inner);
        drop(process);
        suspend_current_and_run_next();
    }

    if !sigmask.is_null() {
        sys_sigprocmask(SIG_SETMASK, raw_ptr as *mut usize, 0 as *mut usize, true);
    }
    unsafe {
        let _ = Box::from_raw(raw_ptr);
    }
    done
}

