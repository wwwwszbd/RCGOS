#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{
    task_info, TaskInfo, TaskStatus
};

#[unsafe(no_mangle)]
pub fn main() -> usize {
    println!("string from task info 1111111111111 test\n");
    let mut info = TaskInfo::new();
    // 注意本次 task info 调用也计入
    assert_eq!(0, task_info(&mut info));
    assert!(info.status == TaskStatus::Running);

    // 想想为什么 write 调用是两次
    println!("string from task info test\n");
    assert_eq!(0, task_info(&mut info));
    assert!(info.status == TaskStatus::Running);

    println!("Test task info OK!");
    0
}
