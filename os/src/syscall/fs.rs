

const FD_STDOUT: usize = 1;
use crate::batch::{get_current_app_range, get_user_stack_range};

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let app_range = get_current_app_range();
    let stack_range = get_user_stack_range();
    //println!("[app_range]: [{:#x}, {:#x})", app_range.0,app_range.1);
    //println!("[stack_range]: [{:#x}, {:#x})", stack_range.0,stack_range.1);
    let buf_begin_pointer = buf as usize;
    let buf_end_pointer = unsafe{buf.offset(len as isize)} as usize;
    //println!("[buf_begin_pointer]: {:#x}", buf_begin_pointer);
    //println!("[buf_end_pointer]: {:#x}", buf_end_pointer);
    // 检查缓冲区是否完全在应用内存范围内
    let in_app_range = (buf_begin_pointer >= app_range.0 && buf_begin_pointer < app_range.1) && 
                      (buf_end_pointer >= app_range.0 && buf_end_pointer < app_range.1);
    
    // 检查缓冲区是否完全在栈内存范围内
    let in_stack_range = (buf_begin_pointer >= stack_range.0 && buf_begin_pointer < stack_range.1) && 
                        (buf_end_pointer >= stack_range.0 && buf_end_pointer < stack_range.1);
    
    // 如果缓冲区既不完全在应用内存内，也不完全在栈内存内，则返回错误
    if !in_app_range && !in_stack_range {
        println!("out of range!");
        return -1 as isize;
    }

    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            //panic!("Unsupported fd in sys_write!");
            -1 as isize
        }
    }
}