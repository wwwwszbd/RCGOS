/// 任务上下文
use crate::trap::trap_return;

/// 任务上下文结构体
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

/// 任务上下文的方法
impl TaskContext {
    /// 创建一个零初始化的任务上下文
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// 跳转到恢复上下文
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        // unsafe extern "C" { fn _restore(); }
        // Self {
        //     ra: _restore as *const () as usize,
        //     sp: kstack_ptr,
        //     s: [0; 12],
        // }
        Self {
            ra: trap_return as *const () as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}