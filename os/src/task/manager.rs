//! 任务队列与调度器（FIFO）。
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
/// 任务管理器（就绪队列）。
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    /// 创建空的 `TaskManager`。
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// 加入一个就绪任务。
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// 取出一个就绪任务；若队列为空则返回 `None`。
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}
/// 加入一个就绪任务。
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}
/// 取出一个就绪任务。
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

pub fn find_task_in_ready_queue(pid: usize) -> Option<Arc<TaskControlBlock>> {
    let manager = TASK_MANAGER.exclusive_access();
    manager
        .ready_queue
        .iter()
        .find(|t| t.getpid() == pid)
        .map(Arc::clone)
}
