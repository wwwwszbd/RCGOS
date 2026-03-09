/// 一个只能在单处理器上使用的安全的可共享可变数据容器。
///
/// 该容器通过内部的 `RefCell` 实现了 interior mutability，
/// 同时通过 `unsafe` 标记确保了在多处理器环境下的安全使用。

use core::cell::{RefCell, RefMut};

pub struct UPSafeCell<T> {
    /// 内部数据
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// 创建一个新的 `UPSafeCell`。
    pub unsafe fn new(value: T) -> Self {
        Self { inner: RefCell::new(value) }
    }
    /// 获取对内部数据的可变引用。
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}