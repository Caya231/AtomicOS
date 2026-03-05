use crate::syscall::*;

pub const SYS_EXIT:  u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_GETPID: u64 = 3;
pub const SYS_FORK: u64   = 4;
pub const SYS_EXEC: u64   = 5;
pub const SYS_WAIT: u64   = 6;

// File Descriptor Syscalls
pub const SYS_OPEN:  u64 = 7;
pub const SYS_CLOSE: u64 = 8;
pub const SYS_READ:  u64 = 9;
pub const SYS_DUP:   u64 = 10;
pub const SYS_DUP2:  u64 = 11;
pub const SYS_PIPE:  u64 = 12;

// Memory Syscalls
pub const SYS_BRK:   u64 = 13;

pub fn exit(status: i32) -> ! {
    unsafe { syscall1(SYS_EXIT, status as u64) };
    loop {}
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    unsafe {
        let res = syscall3(SYS_WRITE, fd as u64, buf.as_ptr() as u64, buf.len() as u64);
        res as isize
    }
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    unsafe {
        let res = syscall3(SYS_READ, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64);
        res as isize
    }
}

pub fn open(path: &str) -> isize {
    unsafe {
        let res = syscall2(SYS_OPEN, path.as_ptr() as u64, path.len() as u64);
        res as isize
    }
}

pub fn close(fd: usize) -> isize {
    unsafe {
        let res = syscall1(SYS_CLOSE, fd as u64);
        res as isize
    }
}

pub fn fork() -> isize {
    unsafe {
        let res = syscall0(SYS_FORK);
        res as isize
    }
}

pub fn exec(path: &str) -> isize {
    unsafe {
        let res = syscall2(SYS_EXEC, path.as_ptr() as u64, path.len() as u64);
        res as isize
    }
}

pub fn wait(pid: isize) -> isize {
    unsafe {
        let res = syscall1(SYS_WAIT, pid as u64);
        res as isize
    }
}

pub fn dup(fd: usize) -> isize {
    unsafe {
        let res = syscall1(SYS_DUP, fd as u64);
        res as isize
    }
}

pub fn dup2(old_fd: usize, new_fd: usize) -> isize {
    unsafe {
        let res = syscall2(SYS_DUP2, old_fd as u64, new_fd as u64);
        res as isize
    }
}

pub fn pipe(fds: &mut [u32; 2]) -> isize {
    unsafe {
        let res = syscall1(SYS_PIPE, fds.as_mut_ptr() as u64);
        res as isize
    }
}

pub fn yield_now() {
    unsafe { syscall0(SYS_YIELD) };
}

pub fn getpid() -> u64 {
    unsafe { syscall0(SYS_GETPID) }
}

/// Changes the location of the program break (expansion of the data segment).
pub fn brk(addr: *mut u8) -> *mut u8 {
    unsafe {
        let res = syscall1(SYS_BRK, addr as u64);
        res as *mut u8
    }
}
