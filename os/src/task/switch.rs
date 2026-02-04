global_asm!(include_str!("switch.S"));

use super::TaskContext;
use core::arch::global_asm;

unsafe extern "C" {
    pub fn __switch(
        current_task_cx_ptr: *mut TaskContext,
        next_task_cx_ptr: *const TaskContext
    );
}