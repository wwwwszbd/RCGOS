#![no_std]
#![no_main]

#[macro_use]
mod console;
mod sbi;
mod lang_items;
mod logging;
mod sync;
mod config;
mod stack_trace;
pub mod loader;
pub mod syscall;
pub mod trap;

use core::arch::global_asm;
use log::*;
//use sbi::*;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

fn clear_bss() {
    unsafe extern "C" {
        fn sbss();  // .bss段起始地址（来自链接脚本）
        fn ebss();  // .bss段结束地址
    }
    
    // 遍历sbss到ebss，逐字节清零
    (sbss as *const () as usize..ebss as *const () as usize).for_each(|addr| {
        unsafe { (addr as *mut u8).write_volatile(0) }
    });
}

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    unsafe extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack_lower_bound();
        fn boot_stack_top();
    }
    clear_bss();
    logging::init();
    println!("[kernel] Hello, world!");
    trace!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as *const () as usize, etext as *const () as usize
    );
    debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as *const () as usize, erodata as *const () as usize
    );
    info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as *const () as usize, edata as *const () as usize
    );
    warn!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as *const () as usize, boot_stack_lower_bound as *const () as usize
    );
    error!("[kernel] .bss [{:#x}, {:#x})", sbss as *const () as usize, ebss as *const () as usize);

    // CI autotest success: sbi::shutdown(false)
    // CI autotest failed : sbi::shutdown(true)
    //sbi::
    // shutdown(false)
    trap::init();
    loader::load_apps();
    loader::run_next_app();
}

