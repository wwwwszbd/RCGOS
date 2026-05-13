#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

static SYNC_TESTS: &[(&str, i32)] = &[
    ("adder_mutex_blocking\0", 0),
    ("adder_mutex_spin\0", 0),
    ("condsync_condvar\0", 0),
    ("barrier_condvar\0", 0),
    ("sync_sem\0", 0),
    ("condsync_sem\0", 0),
    ("mpsc_sem\0", 0),
];

use user_lib::{exec, fork, waitpid};

fn run_tests(tests: &[(&str, i32)]) -> i32 {
    let mut pass_num = 0;
    let mut argv: [*const u8; 1] = [core::ptr::null::<u8>()];
    for test in tests {
        println!("sync_tests: Running {}", test.0);
        argv[0] = test.0.as_ptr();
        let pid = fork();
        if pid == 0 {
            exec(test.0, &argv[..]);
            panic!("unreachable!");
        } else {
            let mut exit_code: i32 = 0;
            let wait_pid = waitpid(pid as usize, &mut exit_code);
            assert_eq!(pid, wait_pid);
            if exit_code == test.1 {
                pass_num += 1;
            }
            println!(
                "sync_tests: Test {} in Process {} exited with code {}",
                test.0, pid, exit_code
            );
        }
    }
    pass_num
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let ok = run_tests(SYNC_TESTS);
    if ok == SYNC_TESTS.len() as i32 {
        println!("sync_tests passed!");
        0
    } else {
        println!("sync_tests failed: {}/{}", ok, SYNC_TESTS.len());
        -1
    }
}
