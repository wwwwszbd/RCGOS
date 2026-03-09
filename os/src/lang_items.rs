/// 定义 panic 处理函数
/// 
/// 当程序发生 panic 时，会调用此函数。
/// 它会打印 panic 信息、调用栈跟踪，并通过 SBI 关闭系统。

use crate::sbi::shutdown;
use core::panic::PanicInfo;
use crate::stack_trace::print_stack_trace;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message());
    }
    unsafe { print_stack_trace(); }
    shutdown(true)
}