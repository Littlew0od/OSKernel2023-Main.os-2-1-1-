use core::mem::size_of;
use core::slice::from_raw_parts;

use crate::mm::{translated_byte_buffer, UserBuffer};
use crate::task::current_user_token;

pub fn sys_uname(buf: *mut u8) -> isize {
    let token = current_user_token();
    let mut user_buf = UserBuffer::new(translated_byte_buffer(
        token,
        buf,
        core::mem::size_of::<utsname>(),
    ));
    let write_size = user_buf.write(utsname::new().as_bytes());
    match write_size {
        0 => -1,
        _ => 0,
    }
}

struct utsname {
    sysName: [u8; 65],
    nodeName: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainName: [u8; 65],
}

impl utsname {
    pub fn new() -> Self {
        Self {
            sysName: utsname::str2array("Linux"),
            nodeName: utsname::str2array("DESKTOP"),
            release: utsname::str2array("5.10.0-7-riscv64"),
            version: utsname::str2array("#1 SMP Debian 5.10.40-1 "),
            machine: utsname::str2array("riscv"),
            domainName: utsname::str2array(""),
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