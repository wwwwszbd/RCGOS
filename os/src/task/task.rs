use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub user_time: usize,
    pub kernel_time: usize, 
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub first_schedule_time: usize,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit, // 未初始化
    Ready, // 准备运行
    Running, // 正在运行
    Exited, // 已退出
}