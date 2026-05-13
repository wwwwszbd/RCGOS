//! 陷入（trap）处理。
//!
//! 本模块设置 `stvec` 指向统一入口 `__alltraps`（见 `trap.S`），并在 Rust 侧分发处理：
//! - 系统调用：进入 `syscall()`。
//! - 时钟中断：触发抢占与任务切换。
mod context;

use crate::config::TRAMPOLINE;
use crate::syscall::syscall;
use crate::task::{
    SignalFlags, check_signals_error_of_current, current_add_signal, current_trap_cx,
    current_trap_cx_user_va,
    current_user_token, exit_current_and_run_next, handle_signals, suspend_current_and_run_next,
};
use crate::timer::{check_timer, set_next_trigger};
use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// 初始化陷入入口（设置 `stvec`）。
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as *const () as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

/// 开启时钟中断。
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[unsafe(no_mangle)]
/// 处理来自用户态的异常/中断/系统调用。
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            // 系统调用返回值
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]);
            // sys_exec 可能切换地址空间并重建 TrapContext，因此需要重新获取引用
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            current_add_signal(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
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
    handle_signals();

    if let Some((errno, msg)) = check_signals_error_of_current() {
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }
    trap_return();
}

#[unsafe(no_mangle)]
/// 返回用户态：跳转到 TRAMPOLINE 中的 `__restore`，并设置必要寄存器。
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = current_trap_cx_user_va();
    let user_satp = current_user_token();
    unsafe extern "C" {
        unsafe fn __alltraps();
        unsafe fn __restore();
    }
    let restore_va = __restore as *const () as usize - __alltraps as *const () as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // 跳转到 TRAMPOLINE 中映射后的 __restore
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = TrapContext 的虚拟地址
            in("a1") user_satp,        // a1 = 用户页表 token
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
/// 内核态陷入处理：当前未实现。
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

pub use context::TrapContext;
