#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{task_info, TaskInfo};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Hello, world! ch3 task");
    let mut info = TaskInfo::new();
    let ans = task_info(&mut info);
    println!("{}", ans);
    0
}