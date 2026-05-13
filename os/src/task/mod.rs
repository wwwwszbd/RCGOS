mod action;
mod context;
mod id;
mod manager;
mod process;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use self::id::TaskUserRes;
use crate::config::MAX_SYSCALL_NUM;
use crate::fs::{OpenFlags, list_apps, open_file};
use crate::sbi::shutdown;
use crate::timer::{get_time_ms, remove_timer};
use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use process::ProcessControlBlock;

pub use action::SignalAction;
pub use context::TaskContext;
pub use id::{IDLE_PID, KernelStack, PidHandle, kstack_alloc, pid_alloc};
pub use manager::{add_task, fetch_task, pid2process, remove_from_pid2process, remove_task, wakeup_task};
pub use processor::{
    current_process, current_task, current_trap_cx, current_trap_cx_user_va,
    current_user_token, run_tasks, schedule, take_current_task,
};
pub use signal::{MAX_SIG, SignalFlags};
pub use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub fn run_first_task() -> ! {
    println!("after initproc!");
    list_apps();
    add_initproc();
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

pub fn block_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Blocked;
    drop(task_inner);
    schedule(task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let process = task.process.upgrade().unwrap();
    let tid = task_inner.res.as_ref().unwrap().tid;
    task_inner.exit_code = Some(exit_code);
    task_inner.res = None;
    task_inner.task_status = TaskStatus::Exited;
    drop(task_inner);
    drop(task);
    if tid == 0 {
        let pid = process.getpid();
        if pid == IDLE_PID {
            println!("[kernel] Idle process exit with exit_code {} ...", exit_code);
            shutdown(exit_code != 0);
        }
        remove_from_pid2process(pid);
        let mut process_inner = process.inner_exclusive_access();
        process_inner.is_zombie = true;
        process_inner.exit_code = exit_code;
        {
            let mut initproc_inner = INITPROC.inner_exclusive_access();
            for child in process_inner.children.iter() {
                child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
                initproc_inner.children.push(child.clone());
            }
        }
        let mut recycle_res = Vec::<TaskUserRes>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            remove_inactive_task(Arc::clone(task));
            let mut task_inner = task.inner_exclusive_access();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }
        drop(process_inner);
        recycle_res.clear();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.children.clear();
        process_inner.memory_set.recycle_data_pages();
        process_inner.fd_table.clear();
        while process_inner.tasks.len() > 1 {
            process_inner.tasks.pop();
        }
    }
    drop(process);
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

#[derive(Copy, Clone)]
pub struct TaskSnapshot {
    pub id: usize,
    pub status: TaskStatus,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time_ms: usize,
}

pub fn get_task_snapshot(pid: usize) -> Option<TaskSnapshot> {
    let task = if let Some(cur) = current_task() {
        if cur.process.upgrade().unwrap().getpid() == pid {
            cur
        } else {
            let process = pid2process(pid)?;
            let inner = process.inner_exclusive_access();
            inner.tasks.get(0)?.as_ref()?.clone()
        }
    } else {
        let process = pid2process(pid)?;
        let inner = process.inner_exclusive_access();
        inner.tasks.get(0)?.as_ref()?.clone()
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

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = {
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        ProcessControlBlock::new(v.as_slice())
    };
}

pub fn add_initproc() {
    let _initproc = INITPROC.clone();
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    process_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.signals |= signal;
}

pub fn handle_signals() {}

pub fn remove_inactive_task(task: Arc<TaskControlBlock>) {
    remove_task(Arc::clone(&task));
    remove_timer(Arc::clone(&task));
}

