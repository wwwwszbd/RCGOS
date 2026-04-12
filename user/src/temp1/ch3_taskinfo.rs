#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_task_id, task_info_ptr, TaskInfo, TaskStatus, SyscallInfo};

static mut INFO: TaskInfo = TaskInfo { id: 0, status: TaskStatus::UnInit, call: [SyscallInfo { id: 0, times: 0 }; 500], time: 0, };

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Hello, world! ch3 task");
    unsafe {
        let id = get_task_id() as usize;
        let ans = task_info_ptr(id, core::ptr::addr_of_mut!(INFO));
        println!("{}", ans);
    }
    0
}
