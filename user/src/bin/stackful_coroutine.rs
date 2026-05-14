// Ported to rCore-Tutorial-v3 user space (RISC-V, no_std).
// Original references:
// - https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust/
// - https://github.com/cfsamson/example-greenthreads
#![no_std]
#![no_main]
//#![feature(asm)]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use core::arch::naked_asm;

//#[macro_use]
use alloc::vec;
use alloc::vec::Vec;

use user_lib::exit;

// Per-task stack size in bytes.
// This demo allocates stacks eagerly and also does a lot of printing, so too-small stacks may
// overflow and crash. The minimal safe value depends on optimization level and the platform.
const DEFAULT_STACK_SIZE: usize = 4096;
const MAX_TASKS: usize = 5;
// A global pointer to the Runtime for `guard()` / `yield_task()`.
static mut RUNTIME: usize = 0;

pub struct Runtime {
    tasks: Vec<Task>,
    current: usize,
}

#[derive(PartialEq, Eq, Debug)]
enum State {
    Available,
    Running,
    Ready,
}

struct Task {
    id: usize,
    stack: Vec<u8>,
    ctx: TaskContext,
    state: State,
}

#[derive(Debug, Default)]
#[repr(C)] // not strictly needed but Rust ABI is not guaranteed to be stable
pub struct TaskContext {
    // 15 u64
    x1: u64,  // ra: return address
    x2: u64,  //sp
    x8: u64,  //s0,fp
    x9: u64,  //s1
    x18: u64, //x18-27: s2-11
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    nx1: u64, // next return address (jump target when resuming)
}

impl Task {
    fn new(id: usize) -> Self {
        // Allocate a fixed stack for each task up front.
        // The important part is that the allocated buffer must not move in memory after we take
        // raw pointers into it (this is why the Vec lives inside the Task).
        Task {
            id: id,
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: TaskContext::default(),
            state: State::Available,
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        // This will be our base task, which will be initialized in the `running` state
        let base_task = Task {
            id: 0,
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: TaskContext::default(),
            state: State::Running,
        };

        // We initialize the rest of our tasks.
        let mut tasks = vec![base_task];
        let mut available_tasks: Vec<Task> = (1..MAX_TASKS).map(|i| Task::new(i)).collect();
        tasks.append(&mut available_tasks);

        Runtime { tasks, current: 0 }
    }

    /// Store a pointer to the Runtime so `guard()` / `yield_task()` can call back into it.
    ///
    /// Safety assumptions:
    /// - the Runtime lives for the whole program (it is created in `main` and never dropped);
    /// - access is effectively single-threaded in this demo.
    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Runtime = self;
            RUNTIME = r_ptr as usize;
        }
    }

    /// This is where we start running our runtime. If it is our base task, we call yield until
    /// it returns false (which means that there are no tasks scheduled) and we are done.
    pub fn run(&mut self) {
        while self.t_yield() {}
        println!("All tasks finished!");
    }

    /// This is our return function. The only place we use this is in our `guard` function.
    /// If the current task is not our base task we set its state to Available. It means
    /// we're finished with it. Then we yield which will schedule a new task to be run.
    fn t_return(&mut self) {
        if self.current != 0 {
            self.tasks[self.current].state = State::Available;
            self.t_yield();
        }
    }

    /// This is the heart of our runtime. Here we go through all tasks and see if anyone is in the `Ready` state.
    /// If no task is `Ready` we're all done. This is an extremely simple scheduler using only a round-robin algorithm.
    ///
    /// If we find a task that's ready to be run we change the state of the current task from `Running` to `Ready`.
    /// Then we call switch which will save the current context (the old context) and load the new context
    /// into the CPU which then resumes based on the context it was just passed.
    ///
    /// NOTE: keep this function non-inlined.
    ///
    /// The context switch uses a naked function + hand-written register save/restore. If the
    /// optimizer inlines/reorders the surrounding control flow, the assumptions made by our
    /// switch routine may no longer hold.
    #[inline(never)]
    fn t_yield(&mut self) -> bool {
        let mut pos = self.current;
        while self.tasks[pos].state != State::Ready {
            pos += 1;
            if pos == self.tasks.len() {
                pos = 0;
            }
            if pos == self.current {
                return false;
            }
        }

        if self.tasks[self.current].state != State::Available {
            self.tasks[self.current].state = State::Ready;
        }

        self.tasks[pos].state = State::Running;
        let old_pos = self.current;
        self.current = pos;

        unsafe {
            switch(&mut self.tasks[old_pos].ctx, &self.tasks[pos].ctx);
        }

        // We should never reach here; the return value only exists to keep the optimizer from
        // treating the call as divergent in some builds.
        self.tasks.len() > 0
    }

    /// Create a new task by preparing its initial context.
    ///
    /// When we spawn a new task we first check if there are any available tasks.
    /// If we run out of tasks we panic in this scenario but there are several (better) ways to handle that.
    /// We keep things simple for now.
    ///
    /// Then we take a raw pointer to the end (high address) of the task's stack buffer.
    ///
    /// Finally, we build the initial context:
    /// - `x1` (ra) points to `guard()`, so returning from the task calls back into the runtime;
    /// - `nx1` points to the task entry function `f`;
    /// - `x2` (sp) points to the prepared stack top.
    ///
    /// Then we mark the task as `Ready`.
    pub fn spawn(&mut self, f: fn()) {
        let available = self
            .tasks
            .iter_mut()
            .find(|t| t.state == State::Available)
            .expect("no available task.");

        println!("RUNTIME: spawning task {}", available.id);
        let size = available.stack.len();
        unsafe {
            let s_ptr = available.stack.as_mut_ptr().offset(size as isize);

            // Ensure the stack pointer is aligned (RISC-V requires at least 8-byte alignment
            // for 64-bit values; keeping it aligned avoids faults in some code paths).
            let s_ptr = (s_ptr as usize & !7) as *mut u8;

            available.ctx.x1 = guard as *const () as u64;
            available.ctx.nx1 = f as *const () as u64;
            // Leave a small gap below stack top to avoid clobbering metadata and keep room for
            // potential call frames.
            available.ctx.x2 = s_ptr.offset(-32) as u64;
        }
        available.state = State::Ready;
    }
}

/// This is our guard function that we place on top of the stack. All this function does is set the
/// state of our current task and then `yield` which will then schedule a new task to be run.
fn guard() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_return();
    };
}

/// Yield execution by calling into the global Runtime.
///
/// Safety assumptions:
/// - `RUNTIME` points to a valid Runtime for the whole program;
/// - there is no concurrent access to the Runtime in this demo.
pub fn yield_task() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_yield();
    };
}

/// Context switch routine.
///
/// Saves callee-saved registers of the old task into `old`, restores those of the new task from
/// `new`, and then jumps to `new.nx1`.
///
/// Requirements:
/// - must be a naked function (no prologue/epilogue);
/// - must not be inlined, otherwise surrounding codegen may break the assumed control flow.
///
/// see: https://github.com/rust-lang/rfcs/blob/master/text/1201-naked-fns.md
/// see: https://doc.rust-lang.org/nightly/reference/inline-assembly.html
/// see: https://doc.rust-lang.org/nightly/rust-by-example/unsafe/asm.html
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch(old: *mut TaskContext, new: *const TaskContext) {
    // a0: _old, a1: _new
    naked_asm!(
        "
        sd x1, 0x00(a0)
        sd x2, 0x08(a0)
        sd x8, 0x10(a0)
        sd x9, 0x18(a0)
        sd x18, 0x20(a0)
        sd x19, 0x28(a0)
        sd x20, 0x30(a0)
        sd x21, 0x38(a0)
        sd x22, 0x40(a0)
        sd x23, 0x48(a0)
        sd x24, 0x50(a0)
        sd x25, 0x58(a0)
        sd x26, 0x60(a0)
        sd x27, 0x68(a0)
        sd x1, 0x70(a0)

        ld x1, 0x00(a1)
        ld x2, 0x08(a1)
        ld x8, 0x10(a1)
        ld x9, 0x18(a1)
        ld x18, 0x20(a1)
        ld x19, 0x28(a1)
        ld x20, 0x30(a1)
        ld x21, 0x38(a1)
        ld x22, 0x40(a1)
        ld x23, 0x48(a1)
        ld x24, 0x50(a1)
        ld x25, 0x58(a1)
        ld x26, 0x60(a1)
        ld x27, 0x68(a1)
        ld t0, 0x70(a1)

        jr t0
        "
    );
}

#[unsafe(no_mangle)]
pub fn main() {
    println!("stackful_coroutine begin...");
    println!("TASK  0(Runtime) STARTING");
    let mut runtime = Runtime::new();
    runtime.init();
    runtime.spawn(|| {
        println!("TASK  1 STARTING");
        let id = 1;
        for i in 0..4 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 1 FINISHED");
    });
    runtime.spawn(|| {
        println!("TASK 2 STARTING");
        let id = 2;
        for i in 0..8 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 2 FINISHED");
    });
    runtime.spawn(|| {
        println!("TASK 3 STARTING");
        let id = 3;
        for i in 0..12 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 3 FINISHED");
    });
    runtime.spawn(|| {
        println!("TASK 4 STARTING");
        let id = 4;
        for i in 0..16 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 4 FINISHED");
    });
    runtime.run();
    println!("stackful_coroutine PASSED");
    exit(0);
}
