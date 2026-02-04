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
    clear_bss();
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

const MAX_SYSCALL_NUM: usize = 500;

#[derive(Debug)]
pub struct TaskInfo {
    pub status: TaskStatus,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time: usize,
}

impl TaskInfo {
    pub fn new() -> Self {
        TaskInfo {
            status: TaskStatus::UnInit,
            syscall_times: [0; MAX_SYSCALL_NUM],
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

pub fn task_info(info: &mut TaskInfo) -> isize {
    sys_task_info(info)
}

#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot find main!");
}



// #![feature(panic_info_message)]
fn clear_bss() {
    unsafe extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as *const () as usize..end_bss as *const () as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}