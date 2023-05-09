#![no_std]
#![no_main]

use user_lib::{shutdown};

#[no_mangle]
pub fn main() -> !{
    shutdown(false)
}