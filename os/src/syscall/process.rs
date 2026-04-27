//! 进程与任务相关系统调用。
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_byte_buffer_checked, translated_ref, translated_refmut, translated_str},
    task::{
        MAX_SIG, SignalAction, SignalFlags, add_task, change_program_brk, current_task,
        current_user_token, exit_current_and_run_next, get_task_snapshot, pid2task,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::get_time_ms,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// 系统调用统计条目。
#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SyscallInfo {
    pub id: usize,
    pub times: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TaskInfo {
    pub id: usize,
    /// 任务状态。
    pub status: TaskStatus,
    /// 系统调用统计（每个 syscall 对应一个条目）。
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    /// 运行时间（单位：毫秒）。
    pub time: usize,
}

/// 退出当前任务并切换到下一个任务。
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// 获取时间（毫秒）。
///
/// - 若 `ts` 为 null，则直接返回当前时间（毫秒）。
/// - 否则将 `TimeVal` 写入用户地址 `ts`，返回 0。
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    if ts.is_null() {
        return get_time_ms() as isize;
    }
    let token = current_user_token();
    let time_val = TimeVal {
        sec: get_time_ms() / 1000,
        usec: (get_time_ms() % 1000) * 1000,
    };

    let Some(buffers) =
        translated_byte_buffer_checked(token, ts as *const u8, core::mem::size_of::<TimeVal>())
    else {
        return -1;
    };
    if buffers.is_empty() {
        return -1;
    }
    let time_val_bytes = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>()
        )
    };

    let mut offset = 0;
    for buffer in buffers {
        let len = buffer.len().min(time_val_bytes.len() - offset);
        if len == 0 {
            break;
        }
        buffer[..len].copy_from_slice(&time_val_bytes[offset..offset + len]);
        offset += len;
    }

    if offset == time_val_bytes.len() { 0 } else { -1 }
}

pub fn sys_task_info(id: usize, ti: *mut TaskInfo) -> isize {
    if ti.is_null() {
        return -1;
    }
    let snapshot = match get_task_snapshot(id) {
        Some(s) => s,
        None => return -1,
    };
    let token = current_user_token();
    let ti_addr = ti as usize;

    fn copy_to_user(token: usize, dst: usize, src: &[u8]) -> bool {
        let Some(buffers) = translated_byte_buffer_checked(token, dst as *const u8, src.len()) else {
            return false;
        };
        if buffers.is_empty() {
            return false;
        }
        let mut offset = 0;
        for buffer in buffers {
            let len = buffer.len().min(src.len().saturating_sub(offset));
            if len == 0 {
                break;
            }
            buffer[..len].copy_from_slice(&src[offset..offset + len]);
            offset += len;
        }
        offset == src.len()
    }

    let off_id = core::mem::offset_of!(TaskInfo, id);
    let off_status = core::mem::offset_of!(TaskInfo, status);
    let off_call = core::mem::offset_of!(TaskInfo, call);
    let off_time = core::mem::offset_of!(TaskInfo, time);

    if !copy_to_user(token, ti_addr + off_id, &snapshot.id.to_ne_bytes()) {
        return -1;
    }
    let status = snapshot.status as u8;
    if !copy_to_user(token, ti_addr + off_status, &[status]) {
        return -1;
    }

    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    const SYSCALL_INFO_SIZE: usize = core::mem::size_of::<SyscallInfo>();
    const BATCH: usize = 32;
    let mut buf = [0u8; BATCH * SYSCALL_INFO_SIZE];
    let mut i = 0usize;
    while i < MAX_SYSCALL_NUM {
        let n = (MAX_SYSCALL_NUM - i).min(BATCH);
        for j in 0..n {
            let id = i + j;
            let times = snapshot.syscall_times[id] as usize;
            let base = j * SYSCALL_INFO_SIZE;
            buf[base..base + USIZE_SIZE].copy_from_slice(&id.to_ne_bytes());
            buf[base + USIZE_SIZE..base + USIZE_SIZE * 2].copy_from_slice(&times.to_ne_bytes());
        }
        let dst = ti_addr + off_call + i * SYSCALL_INFO_SIZE;
        if !copy_to_user(token, dst, &buf[..n * SYSCALL_INFO_SIZE]) {
            return -1;
        }
        i += n;
    }

    if !copy_to_user(token, ti_addr + off_time, &snapshot.time_ms.to_ne_bytes()) {
        return -1;
    }
    0
}

pub fn sys_get_task_id() -> isize {
    current_task().unwrap().getpid() as isize
}

pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    trap_cx.x[10] = 0;
    add_task(new_task);
    new_pid as isize
}

/// 使用 ELF 数据替换当前进程地址空间，并设置用户态参数栈（argc/argv）。
pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe {
            args = args.add(1);
        }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(), args_vec);
        // 返回 argc；sys_exec 会修改 TrapContext，a0 会在返回用户态前被覆盖
        argc as isize
    } else {
        -1
    }
}

/// 等待子进程退出并回收资源。
///
/// # Errors
/// - 返回 -1：不存在满足条件的子进程。
/// - 返回 -2：存在满足条件的子进程，但尚未退出。
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}

pub fn sys_kill(pid: usize, signum: i32) -> isize {
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signum) {
            // 合法信号：向目标任务的 pending 集合中插入
            let mut task_ref = task.inner_exclusive_access();
            if task_ref.signals.contains(flag) {
                return -1;
            }
            task_ref.signals.insert(flag);
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigprocmask(mask: u32) -> isize {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        let old_mask = inner.signal_mask;
        if let Some(flag) = SignalFlags::from_bits(mask) {
            inner.signal_mask = flag;
            old_mask.bits() as isize
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigreturn() -> isize {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        inner.handling_sig = -1;
        // restore the trap context
        let trap_ctx = inner.get_trap_cx();
        *trap_ctx = inner.trap_ctx_backup.take().unwrap();
        // Here we return the value of a0 in the trap_ctx,
        // otherwise it will be overwritten after we trap
        // back to the original execution of the application.
        trap_ctx.x[10] as isize
    } else {
        -1
    }
}

fn check_sigaction_error(signal: SignalFlags, action: usize, old_action: usize) -> bool {
    if action == 0
        || old_action == 0
        || signal == SignalFlags::SIGKILL
        || signal == SignalFlags::SIGSTOP
    {
        true
    } else {
        false
    }
}

pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if signum as usize > MAX_SIG {
        return -1;
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag, action as usize, old_action as usize) {
            return -1;
        }
        let prev_action = inner.signal_actions.table[signum as usize];
        *translated_refmut(token, old_action) = prev_action;
        inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
        0
    } else {
        -1
    }
}
