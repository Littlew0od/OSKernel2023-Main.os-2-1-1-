use crate::config::CLOCK_FREQ;
use crate::mm::{translated_ref, translated_refmut};
use crate::task::{current_user_token, suspend_current_and_run_next};
use crate::timer::{get_time, NSEC_PER_SEC};

pub fn sys_sleep(time_req: *const u64, time_remain: *mut u64) -> isize {
    #[inline]
    fn is_end(end_time: usize) -> bool {
        let current_time = get_time();
        current_time >= end_time
    }
    let token = current_user_token();
    let sec = *translated_ref(token, time_req);
    let nano_sec = *translated_ref(token, unsafe { time_req.add(1) });
    let end_time =
        get_time() + sec as usize * CLOCK_FREQ + nano_sec as usize * CLOCK_FREQ / NSEC_PER_SEC;

    loop {
        if is_end(end_time) {
            break;
        } else {
            suspend_current_and_run_next()
        }
    }
    if time_remain as usize != 0 {
        *translated_refmut(token, time_remain) = 0;
        *translated_refmut(token, unsafe { time_remain.add(1) }) = 0;
    }
    0
}
