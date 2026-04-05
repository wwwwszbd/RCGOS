#![no_std]
#![feature(linkage)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

use syscall::*;

pub const STDOUT: usize = 1;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    //clear_bss();
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

const MAX_SYSCALL_NUM: usize = 500;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SyscallInfo {
    pub id: usize,
    pub times: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub time: usize,
}

impl TaskInfo {
    pub fn new() -> Self {
        TaskInfo {
            id: 0,
            status: TaskStatus::UnInit,
            call: core::array::from_fn(|i| SyscallInfo { id: i, times: 0 }),
            time: 0,
        }
    }
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_() -> isize { 
    sys_yield() 
}

pub fn get_time() -> isize {
    sys_get_time()
}

pub fn get_task_id() -> isize {
    sys_get_task_id()
}

pub fn task_info_ptr(id: usize, info: *mut TaskInfo) -> isize {
    sys_task_info(id, info)
}

pub fn task_info(id: usize, info: &mut TaskInfo) -> isize {
    task_info_ptr(id, info as *mut TaskInfo)
}

pub fn sbrk(size: i32) -> isize {
    sys_sbrk(size)
}

#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot find main!");
}
