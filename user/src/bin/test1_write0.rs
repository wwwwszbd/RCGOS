#![no_std]
#![no_main]

use core::arch::asm;

#[macro_use]
extern crate user_lib;
extern crate core;
use core::slice;
use user_lib::{write, STDOUT};

/// 正确输出：
/// Test write0 OK!

const STACK_SIZE: usize = 0x1000;

unsafe fn r_sp() -> usize {
    let mut sp: usize;
    asm!("mv {}, sp", out(reg) sp);
    sp
}

unsafe fn stack_range() -> (usize, usize) {
    let sp = r_sp();
    let top = (sp + STACK_SIZE - 1) & (!(STACK_SIZE - 1));
    (top - STACK_SIZE, top)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    // 使用一个合法的但超出应用内存范围的地址，而不是空指针
    let invalid_address = 0x1000 as *const u8; // 一个较小的合法地址，但不在应用内存范围内
    
    assert_eq!(
        write(STDOUT, unsafe {
            slice::from_raw_parts(invalid_address, 10)
        }),
        -1
    );
    let (bottom, top) = unsafe { stack_range() };
    
    assert_eq!(
        write(STDOUT, unsafe {
            slice::from_raw_parts((top - 5) as *const _, 10)
        }),
        -1
    );
    assert_eq!(
        write(STDOUT, unsafe {
            slice::from_raw_parts((bottom - 0x1001) as *const _, 10)
        }),
        -1
    );
    // TODO: test string located in .data section
    //println!("User stack range: bottom={:#x}, top={:#x}", bottom, top);
    println!("Test write0 OK!");
    0
}