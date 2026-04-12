#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{
    get_task_id, task_info_ptr, TaskInfo, TaskStatus, SyscallInfo
};

static mut INFO: TaskInfo = TaskInfo { id: 0, status: TaskStatus::UnInit, call: [SyscallInfo { id: 0, times: 0 }; 500], time: 0, };

#[unsafe(no_mangle)]
pub fn main() -> usize {
    println!("string from task info 1111111111111 test\n");
    unsafe {
        let info_ptr = core::ptr::addr_of_mut!(INFO);
        let id = get_task_id() as usize;
        assert_eq!(0, task_info_ptr(id, info_ptr));
        assert!((*info_ptr).status == TaskStatus::Running);
    }

    // 想想为什么 write 调用是两次
    println!("string from task info test\n");
    unsafe {
        let info_ptr = core::ptr::addr_of_mut!(INFO);
        let id = get_task_id() as usize;
        assert_eq!(0, task_info_ptr(id, info_ptr));
        assert!((*info_ptr).status == TaskStatus::Running);
    }

    println!("Test task info OK!");
    0
}
