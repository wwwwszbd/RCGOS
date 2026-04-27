//! 文件系统接口与常用设备。
mod inode;
mod pipe;
mod stdio;

use crate::mm::UserBuffer;
/// 内核侧文件抽象：供 sys_read/sys_write 等统一访问。
pub trait File: Send + Sync {
    /// 是否可读。
    fn readable(&self) -> bool;
    /// 是否可写。
    fn writable(&self) -> bool;
    /// 从文件读入到用户缓冲区。
    fn read(&self, buf: UserBuffer) -> usize;
    /// 将用户缓冲区写入文件。
    fn write(&self, buf: UserBuffer) -> usize;
}

pub use inode::{OpenFlags, list_apps, open_file};
pub use pipe::make_pipe;
pub use stdio::{Stdin, Stdout};
