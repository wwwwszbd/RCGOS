// os/src/sbi.rs
use sbi_rt::{NoReason, Shutdown, SystemFailure, system_reset};

// 串口输出字符
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

// 关机功能
pub fn shutdown(failure: bool) -> ! {
    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!()
}