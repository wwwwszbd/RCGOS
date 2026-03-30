/// 任务模块

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

// use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_app_data, get_num_app};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
pub use task::{TaskControlBlock, TaskStatus};
use crate::timer::get_time_ms;
use crate::timer::get_time_us;
pub use context::TaskContext;

/// 切换的开始时间
pub static mut SWITCH_TIME_START: usize = 0;
/// 切换的总时间
pub static mut SWITCH_TIME_COUNT: usize = 0;
/// 任务切换函数
unsafe fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext) {
    unsafe {
        SWITCH_TIME_START = get_time_us();
    }
    unsafe {
        switch::__switch(current_task_cx_ptr, next_task_cx_ptr);
    }
    unsafe {
        SWITCH_TIME_COUNT += get_time_us() - SWITCH_TIME_START;
    }
}
/// 获取任务切换时间
pub fn get_switch_time_count() -> usize {
    unsafe { SWITCH_TIME_COUNT }
}

/// 任务管理器
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

/// 任务管理器内部结构体
struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
    stop_watch: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,    // 任务数量
            inner: unsafe { UPSafeCell::new(TaskManagerInner {
                tasks,                                  // 任务数组
                current_task: 0,                        // 当前任务索引
                stop_watch: 0,                          // 计时器（用于统计时间）
            })},
        }
    };
}
/// 任务管理器内部方法
impl TaskManagerInner {
    /// 刷新停止watch
    fn refresh_stop_watch(&mut self) -> usize {
        let start_time = self.stop_watch;
        self.stop_watch = get_time_ms();
        self.stop_watch - start_time
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn user_time_start() {
    TASK_MANAGER.user_time_start()
}


pub fn user_time_end() {
    TASK_MANAGER.user_time_end()
}

pub fn get_current_task_block() -> TaskControlBlock {
    TASK_MANAGER.get_current_task_block()
}

pub fn record_syscall_times(syscall_id: usize) {
    TASK_MANAGER.record_syscall_times(syscall_id);
}

pub fn get_current_task() -> usize {
    TASK_MANAGER.get_current_task()
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

impl TaskManager {
    /// 获取当前任务索引
    fn get_current_task(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.current_task
    }
    /// 记录系统调用次数
    fn record_syscall_times(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id] += 1;
    }
    /// 获取当前任务控制块
    fn get_current_task_block(&self) -> TaskControlBlock {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].clone()
    }

    /// 统计内核时间，从现在开始算的是用户时间
    fn user_time_start(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].user_time += inner.refresh_stop_watch();
    }

    /// 统计用户时间，从现在开始算的是内核时间
    fn user_time_end(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// 运行第一个任务
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        task0.first_schedule_time = get_time_ms();
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        inner.refresh_stop_watch();
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(
                &mut _unused as *mut TaskContext,
                next_task_cx_ptr,
            );
        }
        panic!("unreachable in run_first_task!");
    }
    /// 查找下一个可运行任务
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }
    /// 运行下一个任务
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            if inner.tasks[next].first_schedule_time == 0 {
                inner.tasks[next].first_schedule_time = get_time_ms();
            }
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(
                    current_task_cx_ptr,
                    next_task_cx_ptr,
                );
            }
            // 回到用户模式
        } else {
            println!("All applications completed!");
            println!("task switch time: {} us", get_switch_time_count());
            shutdown(false);
        }
    }
    /// 标记当前任务为挂起状态
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
        inner.tasks[current].task_status = TaskStatus::Ready;
    }
    /// 标记当前任务为退出状态
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
        //println!("[task {} exited. user_time: {} ms, kernel_time: {} ms.", current, inner.tasks[current].user_time, inner.tasks[current].kernel_time);
        inner.tasks[current].task_status = TaskStatus::Exited;
    }
}