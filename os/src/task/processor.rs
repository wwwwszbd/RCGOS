//! 处理器抽象与调度控制流。
use super::__switch;
use super::{TaskContext, TaskControlBlock};
use super::{TaskStatus, fetch_task};
use crate::timer::get_time_ms;
use crate::task::shutdown_if_no_tasks;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
/// 处理器管理结构。
pub struct Processor {
    /// 当前正在运行的任务。
    current: Option<Arc<TaskControlBlock>>,
    /// 空闲控制流上下文，用于触发调度与任务切换。
    idle_task_cx: TaskContext,
}

impl Processor {
    /// 创建空的 `Processor`。
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    /// 获取空闲上下文的可变指针。
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    /// 取出当前任务（移动语义）。
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    /// 获取当前任务（克隆 `Arc`）。
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}
/// 调度循环：不断取出就绪任务并切换执行。
pub fn run_tasks() -> ! {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        let Some(task) = fetch_task() else {
            drop(processor);
            shutdown_if_no_tasks();
        };
        let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
        let mut task_inner = task.inner_exclusive_access();
        let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
        task_inner.task_status = TaskStatus::Running;
        if task_inner.first_schedule_time == 0 {
            task_inner.first_schedule_time = get_time_ms();
        }
        drop(task_inner);
        processor.current = Some(task);
        drop(processor);
        unsafe {
            __switch(idle_task_cx_ptr, next_task_cx_ptr);
        }
    }
}
/// 取出当前任务并置空。
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}
/// 获取当前正在运行的任务。
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}
/// 获取当前任务地址空间的 token。
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}
/// 获取当前任务的陷入上下文引用。
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}
/// 切回空闲控制流以触发下一轮调度。
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
