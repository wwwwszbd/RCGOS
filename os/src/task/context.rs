#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        unsafe extern "C" { fn __pre_restore(); }
        Self {
            ra: __pre_restore as *const () as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}