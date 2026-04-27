/// 中断上下文
/// 包含中断发生时的寄存器状态
/// 以及中断处理完成后需要恢复的寄存器状态

use riscv::register::sstatus::{self, SPP, Sstatus};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,             // 内核地址空间的 token ，即内核页表的起始物理地址
    pub kernel_sp: usize,               // 内核栈指针，当前应用在内核栈空间中的内核栈栈顶的虚拟地址
    pub trap_handler: usize,            // 中断处理函数trap handler的地址
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) { self.x[2] = sp; }
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}
