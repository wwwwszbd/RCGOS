#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use user_lib::{
    condvar_create, condvar_signal, condvar_wait, exit, mutex_create, mutex_lock, mutex_unlock,
    thread_create, waittid,
};

const THREAD_NUM: usize = 3;

struct Barrier {
    mutex_id: usize,
    condvar_id: usize,
    count: UnsafeCell<usize>,
}

impl Barrier {
    pub fn new() -> Self {
        Self {
            mutex_id: mutex_create() as usize,
            condvar_id: condvar_create() as usize,
            count: UnsafeCell::new(0),
        }
    }
    pub fn block(&self) {
        mutex_lock(self.mutex_id);
        let count = self.count.get();
        // SAFETY: Here, the accesses of the count is in the
        // critical section protected by the mutex.
        unsafe {
            *count = *count + 1;
        }
        if unsafe { *count } == THREAD_NUM {
            condvar_signal(self.condvar_id);
        } else {
            condvar_wait(self.condvar_id, self.mutex_id);
            condvar_signal(self.condvar_id);
        }
        mutex_unlock(self.mutex_id);
    }
}

unsafe impl Sync for Barrier {}

struct Shared {
    barrier_ab: Barrier,
    barrier_bc: Barrier,
}

fn thread_fn(shared: usize) {
    let shared = unsafe { &*(shared as *const Shared) };
    for _ in 0..300 {
        print!("a");
    }
    shared.barrier_ab.block();
    for _ in 0..300 {
        print!("b");
    }
    shared.barrier_bc.block();
    for _ in 0..300 {
        print!("c");
    }
    exit(0)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let shared = Box::new(Shared {
        barrier_ab: Barrier::new(),
        barrier_bc: Barrier::new(),
    });
    let shared_ptr = Box::into_raw(shared) as usize;
    let mut v: Vec<isize> = Vec::new();
    for _ in 0..THREAD_NUM {
        v.push(thread_create(thread_fn as *const () as usize, shared_ptr));
    }
    for tid in v.into_iter() {
        waittid(tid as usize);
    }
    unsafe {
        drop(Box::from_raw(shared_ptr as *mut Shared));
    }
    println!("\nOK!");
    0
}
