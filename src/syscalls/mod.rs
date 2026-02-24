use crate::{println, scheduler};

/// Kernel syscall interface — callable by tasks.

/// sys_write: write a string to the VGA terminal.
pub fn sys_write(msg: &str) {
    println!("{}", msg);
}

/// sys_yield: cooperatively yield the CPU.
pub fn sys_yield() {
    scheduler::yield_now();
}

/// sys_exit: terminate the current task.
pub fn sys_exit() -> ! {
    scheduler::exit_current();
    // If exit_current returns (no more tasks), halt
    loop { x86_64::instructions::hlt(); }
}

/// sys_spawn: create a new task.
pub fn sys_spawn(entry: fn(), name: &str) -> u64 {
    let id = scheduler::spawn(entry, name);
    id.0
}

/// sys_getpid: return current task ID.
pub fn sys_getpid() -> u64 {
    let sched = scheduler::SCHEDULER.lock();
    sched.current.as_ref().map_or(0, |t| t.id.0)
}

pub fn init() {
    crate::log_info!("Syscall interface initialized.");
}
