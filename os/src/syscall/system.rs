use core::mem::size_of;
use core::slice::from_raw_parts;

use crate::mm::{translated_byte_buffer, UserBuffer};
use crate::task::current_user_token;

pub fn sys_uname(buf: *mut u8) -> isize {
    let token = current_user_token();
    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        core::mem::size_of::<Utsname>(),
    ));
    let write_size = user_buf.write(Utsname::new().as_bytes());
    match write_size {
        0 => -1,
        _ => 0,
    }
}

struct Utsname {
    sysname: [u8; 65],
    nodename: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainname: [u8; 65],
}

impl Utsname {
    pub fn new() -> Self {
        Self {
            sysname: Utsname::str2array("Linux"),
            nodename: Utsname::str2array("DESKTOP"),
            release: Utsname::str2array("5.10.0-7-riscv64"),
            version: Utsname::str2array("#1 SMP Debian 5.10.40-1 "),
            machine: Utsname::str2array("riscv"),
            domainname: Utsname::str2array(""),
        }
    }
    
    fn str2array(str: &str) -> [u8; 65] {
        let bytes = str.as_bytes();
        let len = bytes.len();
        let mut ret = [0u8; 65];
        let copy_part = &mut ret[..len];
        copy_part.copy_from_slice(bytes);
        ret
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, size) }
    }
}