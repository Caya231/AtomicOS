use crate::{println, scheduler};

/// Syscall numbers (passed in RAX from userland).
pub const SYS_EXIT:  u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_GETPID: u64 = 3;

/// Central syscall dispatcher — called from the int 0x80 handler.
/// Arguments come from registers: rax=number, rdi=arg0, rsi=arg1, rdx=arg2.
/// Returns result in rax.
pub fn dispatch(number: u64, arg0: u64, arg1: u64, _arg2: u64) -> u64 {
    match number {
        SYS_EXIT => {
            crate::log_info!("syscall: exit");
            scheduler::exit_current();
            0 // unreachable, but needed for type
        }
        SYS_WRITE => {
            // arg0 = pointer to buffer, arg1 = length
            let ptr = arg0 as *const u8;
            let len = arg1 as usize;
            if len > 4096 { return u64::MAX; } // sanity limit
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            if let Ok(s) = core::str::from_utf8(slice) {
                print_no_newline(s);
            }
            len as u64
        }
        SYS_YIELD => {
            scheduler::yield_now();
            0
        }
        SYS_GETPID => {
            let sched = scheduler::SCHEDULER.lock();
            sched.current.as_ref().map_or(0, |t| t.id.0)
        }
        _ => {
            crate::log_warn!("syscall: unknown number {}", number);
            u64::MAX // error
        }
    }
}

/// Print without trailing newline.
fn print_no_newline(s: &str) {
    use core::fmt::Write;
    let _ = crate::vga::WRITER.lock().write_str(s);
    let _ = crate::serial::SERIAL1.lock().write_str(s);
}

// ── Kernel-side wrappers (called directly from kernel code, not via int 0x80) ──

/// sys_write: write a string to the VGA terminal (kernel-side).
pub fn sys_write(msg: &str) {
    crate::println!("{}", msg);
}

/// sys_yield: cooperatively yield the CPU.
pub fn sys_yield() {
    scheduler::yield_now();
}

/// sys_exit: terminate the current task.
pub fn sys_exit() -> ! {
    scheduler::exit_current();
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
