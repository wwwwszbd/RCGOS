use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_byte_buffer_checked, translated_ref, translated_refmut, translated_str},
    task::{
        MAX_SIG, SignalAction, SignalFlags, TaskStatus, current_process,
        current_user_token, exit_current_and_run_next, get_task_snapshot, pid2process,
        suspend_current_and_run_next,
    },
    timer::get_time_ms,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

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
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub time: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

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
            core::mem::size_of::<TimeVal>(),
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
    if offset == time_val_bytes.len() {
        0
    } else {
        -1
    }
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
        let Some(buffers) = translated_byte_buffer_checked(token, dst as *const u8, src.len())
        else {
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
            let sid = i + j;
            let times = snapshot.syscall_times[sid] as usize;
            let base = j * SYSCALL_INFO_SIZE;
            buf[base..base + USIZE_SIZE].copy_from_slice(&sid.to_ne_bytes());
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
    current_process().getpid() as isize
}

pub fn sys_getpid() -> isize {
    current_process().getpid() as isize
}

pub fn sys_fork() -> isize {
    let current_process = current_process();
    let new_process = current_process.fork();
    let new_pid = new_process.getpid();
    let new_process_inner = new_process.inner_exclusive_access();
    let task = new_process_inner.tasks[0].as_ref().unwrap();
    let trap_cx = task.inner_exclusive_access().get_trap_cx();
    trap_cx.x[10] = 0;
    new_pid as isize
}

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
        let process = current_process();
        let argc = args_vec.len();
        process.exec(all_data.as_slice(), args_vec);
        argc as isize
    } else {
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == p.getpid())
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

pub fn sys_kill(pid: usize, signal: i32) -> isize {
    if let Some(process) = pid2process(pid) {
        if let Some(flag) = SignalFlags::from_bits(1u32 << (signal as u32)) {
            process.inner_exclusive_access().signals |= flag;
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sbrk(_size: i32) -> isize {
    -1
}

pub fn sys_sigaction(
    signum: i32,
    _action: *const SignalAction,
    _old_action: *mut SignalAction,
) -> isize {
    if signum < 0 || signum as usize > MAX_SIG {
        return -1;
    }
    0
}

pub fn sys_sigprocmask(_mask: u32) -> isize {
    0
}

pub fn sys_sigreturn() -> isize {
    0
}

