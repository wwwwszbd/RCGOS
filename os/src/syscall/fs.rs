// /// 系统调用 `write` 的实现。
// ///
// /// 该函数用于将用户空间缓冲区中的数据写入到指定的文件描述符中。
// /// 支持标准输出（`FD_STDOUT`）和其他文件描述符。
// ///
// /// # 参数
// ///
// /// - `fd`: 文件描述符，指定要写入的目标文件。
// /// - `buf`: 用户空间缓冲区的指针，包含要写入的数据。
// /// - `len`: 要写入的数据长度。
// ///
// /// # 返回值
// ///
// /// - 成功时返回写入的字节数。
// /// - 失败时返回 `-1`。

// use crate::config::*;
// use crate::task::get_current_task;
// use crate::loader::{USER_STACK, get_base_i};

// const FD_STDOUT: usize = 1;

// /// 检查用户空间缓冲区的地址是否合法。
// ///
// /// 该函数用于检查用户空间缓冲区的地址是否在合法的范围内。
// /// 合法范围包括应用程序的内存空间和用户栈空间。
// ///
// /// # 参数
// ///
// /// - `slice`: 用户空间缓冲区的切片，包含要写入的数据。
// ///
// /// # 返回值
// ///
// /// - 成功时返回写入的字节数。
// /// - 失败时返回 `None`。
// fn check_addr(slice: &[u8]) -> Option<isize> {
//     let task_id = get_current_task();
//     let app_start = slice.as_ptr().addr();
//     let app_size = slice.len();
//     if !((app_start >= get_base_i(task_id) &&
//         app_start + app_size <= get_base_i(task_id) + APP_SIZE_LIMIT) ||
//         (app_start + app_size <= USER_STACK[task_id].get_sp() &&
//         app_start >= USER_STACK[task_id].get_sp() - USER_STACK_SIZE)) {
//         None
//     } else {
//         Some(app_size as isize)
//     }
// }
// /// 系统调用 `write` 的实现。
// ///
// /// 该函数用于将用户空间缓冲区中的数据写入到指定的文件描述符中。
// /// 支持标准输出（`FD_STDOUT`）和其他文件描述符。
// ///
// /// # 参数
// ///
// /// - `fd`: 文件描述符，指定要写入的目标文件。
// /// - `buf`: 用户空间缓冲区的指针，包含要写入的数据。
// /// - `len`: 要写入的数据长度。
// ///
// /// # 返回值
// ///
// /// - 成功时返回写入的字节数。
// /// - 失败时返回 `-1`。
// pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    
//     match fd {
//         FD_STDOUT => {
//             let slice = unsafe { core::slice::from_raw_parts(buf, len) };
//             match check_addr(slice) {
//                 None => -1 as isize,
//                 Some(i_len) => {
//                     let str = core::str::from_utf8(slice).unwrap();
//                     print!("{}", str);
//                     i_len
//                 }
//             }
//         }
//         _ => {
//             //panic!("Unsupported fd in sys_write!");
//             -1 as isize
//         }
//     }

//     // match fd {
//     //     FD_STDOUT => {
//     //         let slice = unsafe { core::slice::from_raw_parts(buf, len) };
//     //         let str = core::str::from_utf8(slice).unwrap();
//     //         print!("{}", str);
//     //         len as isize
//     //     }
//     //     _ => {
//     //         //panic!("Unsupported fd in sys_write!");
//     //         -1 as isize
//     //     }
//     // }
// }

//! File and filesystem-related syscalls

use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;

const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
