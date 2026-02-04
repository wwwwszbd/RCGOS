use crate::{
    config::MAX_SYSCALL_NUM,
    task::{exit_current_and_run_next, suspend_current_and_run_next, get_current_task_block, TaskStatus},
    timer::{get_time_us, get_time_ms},
};
// use log::*;

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_us() as isize
}

//需要完整的内存管理才能正常实现
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    info!("kernel: sys_task_info");
    let task_block = get_current_task_block();
    unsafe {
        *_ti = TaskInfo {
            status: task_block.task_status,
            syscall_times: task_block.syscall_times,
            time: get_time_ms()-task_block.first_schedule_time as usize,
        };
    }
    0
}