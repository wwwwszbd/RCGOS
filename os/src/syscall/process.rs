/// 系统调用实现

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{translated_byte_buffer, translated_byte_buffer_checked},
    task::{current_user_token, change_program_brk, exit_current_and_run_next, get_task_snapshot, get_current_task, suspend_current_and_run_next, TaskStatus},
    timer::{get_time_us, get_time_ms},
};
// use log::*;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// 任务描述信息
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
    /// 任务状态在生命周期中的状态
    pub status: TaskStatus,
    /// 任务调用的系统调用次数（每个 syscall 对应一个条目）
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    /// 任务运行的总时间（单位：毫秒）
    pub time: usize,
}

/// 任务退出并提交退出码
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

// pub fn sys_get_time() -> isize {
//     get_time_us() as isize
// }
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    if ts.is_null() {
        return get_time_us() as isize;
    }
    let token = current_user_token();
    let time_val = TimeVal {
        sec: get_time_ms() / 1000,
        usec: (get_time_ms() % 1000) * 1000,
    };
    
    // 使用translated_byte_buffer将时间值写回用户空间
    let buffers = translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    if buffers.is_empty() {
        panic!("sys_get_time: buffers is null");
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
    
    0
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
    get_current_task() as isize
}

pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
