mod action;
mod context;
mod manager;
mod pid;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::MAX_SYSCALL_NUM;
use crate::fs::{OpenFlags, list_apps, open_file};
use crate::sbi::shutdown;
use crate::timer::get_time_ms;
use crate::mm::VirtAddr;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use manager::remove_from_pid2task;
pub use action::{SignalAction, SignalActions};
pub use context::TaskContext;
pub use manager::{add_task, fetch_task, pid2task};
pub use pid::{KernelStack, PidHandle, pid_alloc};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
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

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    remove_from_pid2task(task.getpid());
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;
    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    inner.fd_table.clear();
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

lazy_static! {
    /// initproc 进程：负责拉起用户态 shell
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}
/// 将 initproc 加入调度队列
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    task_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            task_inner.frozen = true;
            task_inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if task_inner.signals.contains(SignalFlags::SIGCONT) {
                task_inner.signals ^= SignalFlags::SIGCONT;
                task_inner.frozen = false;
            }
        }
        _ => {
            task_inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let handler = task_inner.signal_actions.table[sig].handler;
    if handler != 0 {
        // 标记正在处理的信号，并从 pending 集合中清除
        task_inner.handling_sig = sig as isize;
        task_inner.signals ^= signal;

        // 备份用户态 TrapContext，用于 sigreturn 恢复
        let trap_ctx = task_inner.get_trap_cx();
        task_inner.trap_ctx_backup = Some(*trap_ctx);

        // 将返回地址改为用户注册的 handler
        trap_ctx.sepc = handler;

        // a0 = signum
        trap_ctx.x[10] = sig;
    } else {
        // 默认处理：当前实现仅打印提示
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
}

fn check_pending_signals() {
    for sig in 0..(MAX_SIG + 1) {
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if task_inner.signals.contains(signal) && (!task_inner.signal_mask.contains(signal)) {
            let mut masked = true;
            let handling_sig = task_inner.handling_sig;
            if handling_sig == -1 {
                masked = false;
            } else {
                let handling_sig = handling_sig as usize;
                if !task_inner.signal_actions.table[handling_sig]
                    .mask
                    .contains(signal)
                {
                    masked = false;
                }
            }
            if !masked {
                drop(task_inner);
                drop(task);
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}

pub fn handle_signals() {
    loop {
        check_pending_signals();
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let task_inner = task.inner_exclusive_access();
            (task_inner.frozen, task_inner.killed)
        };
        if !frozen || killed {
            break;
        }
        suspend_current_and_run_next();
    }
}
