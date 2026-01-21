#![no_std]
#![no_main]

#[macro_use]
mod console;
mod sbi;
mod lang_items;

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));

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
    clear_bss();  // 清零.bss段
    // 测试输出
    println!("Hello, world!");
    // 测试panic
    panic!("Shutdown machine!");
}

