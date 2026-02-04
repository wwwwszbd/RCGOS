mod context;

// use crate::loader::run_next_app;
use crate::syscall::syscall;
use crate::task::{suspend_current_and_run_next, exit_current_and_run_next};
use crate::timer::get_time_us;
use crate::timer::set_next_trigger;
use crate::trap::scause::Interrupt;
use crate::task::SWITCH_TIME_START;
use crate::task::SWITCH_TIME_COUNT;
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

pub fn init() {
    unsafe extern "C" { 
      fn __alltraps(); 
    }
    unsafe {
        stvec::write(__alltraps as *const () as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn switch_cost(cx: &mut TrapContext) -> &mut TrapContext {
    unsafe {
        SWITCH_TIME_COUNT += get_time_us() - SWITCH_TIME_START;
    }
    cx
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    crate::task::user_time_end();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            // run_next_app();
            panic!("[kernel] Cannot continue!");
            // exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            // run_next_app();
            panic!("[kernel] Cannot continue!");
            // exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    crate::task::user_time_start();
    cx
}

pub use context::TrapContext;