// 在屏幕上打印一行 Hello world from user mode program!

#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Hello, world!");
    0
}