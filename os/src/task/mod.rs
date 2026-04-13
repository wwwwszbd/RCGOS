mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::MAX_SYSCALL_NUM;
use crate::loader::{get_app_data_by_name, list_apps};
use crate::sbi::shutdown;
use crate::timer::get_time_ms;
use crate::mm::VirtAddr;
use alloc::sync::Arc;
pub use context::TaskContext;
pub use manager::{add_task, fetch_task};
pub use pid::{KernelStack, PidHandle, pid_alloc};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
pub use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub fn run_first_task() -> ! {
    let initproc = get_app_data_by_name("initproc").unwrap();
    add_task(Arc::new(TaskControlBlock::new(initproc)));
    println!("after initproc!");
    list_apps();
    run_tasks()
}

pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);
    schedule(task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;
    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(task);
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

pub fn record_syscall_times(syscall_id: usize) {
    if syscall_id >= MAX_SYSCALL_NUM {
        return;
    }
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        inner.syscall_times[syscall_id] += 1;
    }
}

pub fn change_program_brk(size: i32) -> Option<usize> {
    let task = current_task()?;
    let mut inner = task.inner_exclusive_access();
    let old_break = inner.program_brk;
    let new_brk = inner.program_brk as isize + size as isize;
    if new_brk < inner.heap_bottom as isize {
        return None;
    }
    let heap_bottom = inner.heap_bottom;
    let result = if size < 0 {
        inner
            .memory_set
            .shrink_to(VirtAddr(heap_bottom), VirtAddr(new_brk as usize))
    } else {
        inner
            .memory_set
            .append_to(VirtAddr(heap_bottom), VirtAddr(new_brk as usize))
    };
    if result {
        inner.program_brk = new_brk as usize;
        Some(old_break)
    } else {
        None
    }
}

#[derive(Copy, Clone)]
pub struct TaskSnapshot {
    pub id: usize,
    pub status: TaskStatus,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time_ms: usize,
}

pub fn get_task_snapshot(pid: usize) -> Option<TaskSnapshot> {
    let task = if let Some(cur) = current_task() {
        if cur.getpid() == pid {
            cur
        } else {
            manager::find_task_in_ready_queue(pid)?
        }
    } else {
        manager::find_task_in_ready_queue(pid)?
    };
    let inner = task.inner_exclusive_access();
    let time_ms = if inner.first_schedule_time == 0 {
        0
    } else {
        get_time_ms().saturating_sub(inner.first_schedule_time)
    };
    Some(TaskSnapshot {
        id: pid,
        status: inner.task_status,
        syscall_times: inner.syscall_times,
        time_ms,
    })
}

pub fn shutdown_if_no_tasks() -> ! {
    shutdown(false)
}
