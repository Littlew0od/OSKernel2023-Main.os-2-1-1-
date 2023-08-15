#![allow(unused)]
use core::mem::size_of;
use core::slice::from_raw_parts;

use crate::mm::{translated_byte_buffer, UserBuffer};
use crate::syscall::errno::EPERM;
use crate::syscall::syslog::*;
use crate::task::current_user_token;
use crate::timer::get_time_sec;

use super::errno::SUCCESS;

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

    // For easier memory writing
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, size) }
    }
}

pub fn sys_syslog(type_: usize, bufp: *mut u8, len: usize) -> isize {
    const LOG_BUF_LEN: usize = 4096;
    const LOG: &str ="[    0.000000] Linux version 5.15.90.1-microsoft-standard-WSL2 (oe-user@oe-host) (x86_64-msft-linux-gcc (GCC) 9.3.0, GNU ld (GNU Binutils) 2.34.0.20200220) #1 SMP Fri Jan 27 02:56:13 UTC 2023";
    let len = LOG.len().min(len as usize);
    let token = current_user_token();
    match type_ {
        SYSLOG_ACTION_CLOSE | SYSLOG_ACTION_OPEN => SUCCESS,
        SYSLOG_ACTION_READ => {
            let mut user_buf = UserBuffer::new(translated_byte_buffer(token, bufp, len));
            let write_size = user_buf.write(LOG[..len].as_bytes());
            len as isize
        }
        SYSLOG_ACTION_READ_ALL => {
            let mut user_buf = UserBuffer::new(translated_byte_buffer(token, bufp, len));
            let write_size = user_buf.write(LOG[LOG.len() - len..].as_bytes());
            len as isize
        }
        SYSLOG_ACTION_READ_CLEAR => todo!(),
        SYSLOG_ACTION_CLEAR => todo!(),
        SYSLOG_ACTION_CONSOLE_OFF => todo!(),
        SYSLOG_ACTION_CONSOLE_ON => todo!(),
        SYSLOG_ACTION_CONSOLE_LEVEL => todo!(),
        SYSLOG_ACTION_SIZE_UNREAD => todo!(),
        SYSLOG_ACTION_SIZE_BUFFER => LOG_BUF_LEN as isize,
        _ => {
            println!("[sys_syslog] unkonwn type!");
            EPERM
        }
    }
}

#[repr(C)]
pub struct SysInfo {
    uptime: isize,
    loads: [usize; 3],
    total_ram: usize,
    free_ram: usize,
    shared_ram: usize,
    buffer_ram: usize,
    total_swap: usize,
    free_swap: usize,
    procs: u16,
    total_high: usize,
    free_high: usize,
    mem_unit: u32,
    _f: [u8; 20 - 2 * size_of::<usize>() - size_of::<u32>()],
}

impl SysInfo {
    pub fn new() -> Self {
        extern "C" {
            fn BASE_ADDRESS();
        }
        Self {
            uptime: get_time_sec() as isize,
            // Use only current sample (as average) to evaluate
            loads: [1092, 218, 72],
            total_ram: crate::config::MEMORY_END - BASE_ADDRESS as usize,
            free_ram: 0x1000000,
            shared_ram: 1,
            buffer_ram: 1,
            total_swap: 0,
            free_swap: 0,
            procs: 1,
            total_high: 0,
            free_high: 0,
            mem_unit: 1,
            _f: [0; 20 - 2 * size_of::<usize>() - size_of::<u32>()],
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const _ as *const u8, size_of::<SysInfo>()) }
    }
}

pub fn sys_sysinfo(buf: *mut u8) -> isize {
    let token = current_user_token();
    let mut user_buf = UserBuffer::new(translated_byte_buffer(token, buf, size_of::<SysInfo>()));
    user_buf.write(SysInfo::new().as_bytes());
    SUCCESS
}
