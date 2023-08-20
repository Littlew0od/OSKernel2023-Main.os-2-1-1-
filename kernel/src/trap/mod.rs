mod context;

use crate::config::{TRAMPOLINE, MAX_TRAP_ID};
use crate::sync::UPSafeCell;
use crate::syscall::{sys_getpid, syscall};
use crate::task::{
    check_signals_of_current_process, current_add_signal, current_trap_cx, current_trap_cx_user_va,
    current_user_token, exit_current_and_run_next, handle_signals, suspend_current_and_run_next,
    SignalFlags,
};
use crate::timer::{check_timer, set_next_trigger};
use core::arch::{asm, global_asm};
use alloc::string::{String, ToString};
use lazy_static::*;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

lazy_static! {
    pub static ref INTERRUPT: UPSafeCell<InterruptNum> =
        unsafe { UPSafeCell::new(InterruptNum::new()) };
}

pub struct InterruptNum {
    pub inner: [i32; MAX_TRAP_ID],
    pub offset: usize,
}

impl InterruptNum {
    fn new() -> Self {
        Self {
            inner: [0; MAX_TRAP_ID],
            offset: 0,
        }
    }
    fn add(&mut self, trap_id: usize) {
        self.inner[trap_id] += 1;
    }
    pub fn get(&mut self) -> String{
        if self.offset == 0 {
            self.offset = MAX_TRAP_ID;
            let mut str = String::from("");
            for index in 0..MAX_TRAP_ID{
                let mut substr = index.to_string();
                substr.push_str(": ");
                substr.push_str(&self.inner[index].to_string());
                str.push_str(&substr);
                str.push('\n');
            }
            return str;
        }else {
            return String::new();
        }
    }
}

pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            INTERRUPT
                .exclusive_access()
                .add(Exception::UserEnvCall as usize);
            // jump to next instruction anyway
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            // get system call return value
            let result = syscall(
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]],
            );
            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            INTERRUPT
                .exclusive_access()
                .add(Exception::StorePageFault as usize);
            log!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it, pid = {}.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
                sys_getpid(),
            );
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            INTERRUPT
                .exclusive_access()
                .add(Exception::IllegalInstruction as usize);
            log!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it, pid = {}.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
                sys_getpid(),
            );
            current_add_signal(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            INTERRUPT
                .exclusive_access()
                .add(Interrupt::SupervisorTimer as usize);
            set_next_trigger();
            check_timer();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    // handle signals (handle the sent signal)
    handle_signals();

    // check signals
    if let Some((errno, msg)) = check_signals_of_current_process() {
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }
    // if let Some((errno, msg)) = check_signals_of_current_thread() {
    //     println!("[kernel] {}", msg);
    //     exit_current_and_run_next(errno);
    // }
    trap_return();
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_user_va = current_trap_cx_user_va();
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_user_va,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    use riscv::register::sepc;
    println!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}

pub use context::TrapContext;
