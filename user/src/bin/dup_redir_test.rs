#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{close, dup, exit, fork, pipe, read, waitpid, write};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut pipe_fd = [0usize; 2];
    if pipe(&mut pipe_fd) != 0 {
        return -1;
    }

    let pid = fork();
    if pid < 0 {
        return -1;
    }
    if pid == 0 {
        close(pipe_fd[0]);
        close(1);
        let new_fd = dup(pipe_fd[1]);
        if new_fd != 1 {
            exit(-1);
        }
        close(pipe_fd[1]);
        if write(1, b"ABC") != 3 {
            exit(-1);
        }
        0
    } else {
        close(pipe_fd[1]);
        let mut buf = [0u8; 8];
        let mut got = 0usize;
        while got < 3 {
            let n = read(pipe_fd[0], &mut buf[got..]) as isize;
            if n <= 0 {
                break;
            }
            got += n as usize;
        }
        close(pipe_fd[0]);
        if got != 3 || &buf[..3] != b"ABC" {
            return -1;
        }
        let mut exit_code: i32 = 0;
        if waitpid(pid as usize, &mut exit_code) != pid {
            return -1;
        }
        if exit_code != 0 {
            return -1;
        }
        println!("dup_redir_test passed!");
        0
    }
}

