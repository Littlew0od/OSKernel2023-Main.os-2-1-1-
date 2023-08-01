use core::fmt::{self, Debug, Formatter};

use crate::timer::TimeVal;

#[allow(unused)]
#[derive(Clone, Copy)]
pub struct Rusage {
    pub ru_utime: TimeVal, /* user CPU time used */
    pub ru_stime: TimeVal, /* system CPU time used */
    ru_maxrss: isize,      // NOT IMPLEMENTED /* maximum resident set size */
    ru_ixrss: isize,       // NOT IMPLEMENTED /* integral shared memory size */
    ru_idrss: isize,       // NOT IMPLEMENTED /* integral unshared data size */
    ru_isrss: isize,       // NOT IMPLEMENTED /* integral unshared stack size */
    ru_minflt: isize,      // NOT IMPLEMENTED /* page reclaims (soft page faults) */
    ru_majflt: isize,      // NOT IMPLEMENTED /* page faults (hard page faults) */
    ru_nswap: isize,       // NOT IMPLEMENTED /* swaps */
    ru_inblock: isize,     // NOT IMPLEMENTED /* block input operations */
    ru_oublock: isize,     // NOT IMPLEMENTED /* block output operations */
    ru_msgsnd: isize,      // NOT IMPLEMENTED /* IPC messages sent */
    ru_msgrcv: isize,      // NOT IMPLEMENTED /* IPC messages received */
    ru_nsignals: isize,    // NOT IMPLEMENTED /* signals received */
    ru_nvcsw: isize,       // NOT IMPLEMENTED /* voluntary context switches */
    ru_nivcsw: isize,      // NOT IMPLEMENTED /* involuntary context switches */
}

impl Rusage {
    pub fn new() -> Self {
        Self {
            ru_utime: TimeVal::new(),
            ru_stime: TimeVal::new(),
            ru_maxrss: 0,
            ru_ixrss: 0,
            ru_idrss: 0,
            ru_isrss: 0,
            ru_minflt: 0,
            ru_majflt: 0,
            ru_nswap: 0,
            ru_inblock: 0,
            ru_oublock: 0,
            ru_msgsnd: 0,
            ru_msgrcv: 0,
            ru_nsignals: 0,
            ru_nvcsw: 0,
            ru_nivcsw: 0,
        }
    }
}

impl Debug for Rusage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "(ru_utime:{:?}, ru_stime:{:?})",
            self.ru_utime, self.ru_stime
        ))
    }
}