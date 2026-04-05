/// 系统调用接口。
///
/// 该模块定义了系统调用的接口，包括系统调用号和参数。
/// 系统调用是用户空间程序请求操作系统服务的一种机制。
///
/// # 系统调用号
///
/// 每个系统调用都有一个唯一的系统调用号，用于标识要执行的操作。
///
/// # 参数
///
/// 每个系统调用都有固定数量的参数，用于传递必要的信息给操作系统。
/// 参数的数量和类型在 `config.rs` 中定义。
///
/// # 返回值
///
/// 每个系统调用都有一个返回值，用于指示操作是否成功或返回相关信息。
/// 返回值的类型和含义在 `config.rs` 中定义。

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_TASK_INFO: usize = 410;
const SYSCALL_GET_TASK_ID: usize = 411;
const SYSCALL_SBRK: usize = 214;

mod fs;
mod process;
use crate::task::record_syscall_times;
use fs::*;
use process::*;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    record_syscall_times(syscall_id);
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        // SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0], args[1] as *mut TaskInfo),
        SYSCALL_GET_TASK_ID => sys_get_task_id(),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
