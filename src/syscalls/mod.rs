use crate::scheduler;

/// Syscall numbers (passed in RAX from userland).
pub const SYS_EXIT:  u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_GETPID: u64 = 3;
pub const SYS_FORK: u64   = 4;
pub const SYS_EXEC: u64   = 5;
pub const SYS_WAIT: u64   = 6;

// File Descriptor Syscalls (Phase 5.4)
pub const SYS_OPEN:  u64 = 7;
pub const SYS_CLOSE: u64 = 8;
pub const SYS_READ:  u64 = 9;
pub const SYS_DUP:   u64 = 10;
pub const SYS_DUP2:  u64 = 11;
pub const SYS_PIPE:  u64 = 12;

/// Central syscall dispatcher — called from the int 0x80 handler.
/// Arguments come from registers: rax=number, rdi=arg0, rsi=arg1, rdx=arg2.
/// Returns result in rax.
pub extern "C" fn dispatch(number: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    // Enable interrupts so that system calls can be preempted by hardware timers!
    // Since int 0x80 goes through an Interrupt Gate, the CPU automatically masks IF=0. 
    x86_64::instructions::interrupts::enable();
    
    match number {
        SYS_EXIT => {
            let exit_code = arg0;
            scheduler::exit_current(exit_code);
            0 // unreachable, but needed for type
        }
        SYS_READ => {
            let fd = arg0 as usize;
            let ptr = arg1 as *mut u8;
            let len = arg2 as usize;
            
            if fd >= 64 || len == 0 || len > 1024 * 1024 { return u64::MAX; }
            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
            
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            // Re-borrow the Arc to drop the scheduler lock early!
            let file_arc = match current.fd_table[fd].clone() {
                Some(f) => f,
                None => return u64::MAX,
            };
            
            drop(sched); // Critical: Unlock scheduler before blocking OS ops!
            
            let mut file = file_arc.lock();
            if !file.readable { return u64::MAX; }
            
            use crate::fs::fd::FileType;
            match &mut file.file_type {
                FileType::Console => {
                    // For now, Console Read is a simplified generic mock because Phase 5.4 
                    // doesn't focus on TTY line disciplines. 
                    slice[0] = b'\n';
                    1
                }
                FileType::Regular => {
                    // FAT32 Mock read for Phase 5.4 - Just return 0 (EOF) for now as we test Pipes
                    0
                }
                FileType::PipeRead(pipe_inner) => {
                    // Read from pipe lock
                    let mut inner = pipe_inner.lock();
                    loop {
                        if !inner.is_empty() {
                            let read_bytes = inner.read(slice);
                            // Wake up any writers waiting for space!
                            scheduler::wake_all_blocked(); 
                            return read_bytes as u64;
                        }
                        
                        if inner.active_writers() == 0 {
                            return 0; // EOF
                        }
                        
                        // Wait for writers to push data!
                        drop(inner);
                        drop(file);
                        
                        // Block current process and Yield!
                        let mut sched = scheduler::SCHEDULER.lock();
                        sched.current.as_mut().unwrap().state = scheduler::ProcessState::Blocked;
                        drop(sched);
                        scheduler::yield_now();
                        
                        // Re-acquire locks after waking up to try reading again
                        file = file_arc.lock();
                        // Refetch inner reference after lock manipulation
                        match &file.file_type {
                            FileType::PipeRead(p) => inner = p.lock(),
                            _ => return u64::MAX,
                        }
                    }
                }
                _ => u64::MAX,
            }
        }
        SYS_WRITE => {
            let fd = arg0 as usize;
            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            
            if fd >= 64 || len == 0 || len > 1024 * 1024 { return u64::MAX; }
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            let file_arc = match current.fd_table[fd].clone() {
                Some(f) => f,
                None => return u64::MAX,
            };
            
            drop(sched); // Yield scheduler lock
            
            use crate::fs::fd::FileType;
            let mut file = file_arc.lock();
            if !file.writable { return u64::MAX; }
            
            match &mut file.file_type {
                FileType::Console => {
                    if let Ok(s) = core::str::from_utf8(slice) {
                        print_no_newline(s);
                    }
                    len as u64
                }
                FileType::Regular => {
                    // FAT32 Mock write for Phase 5.4
                    len as u64
                }
                FileType::PipeWrite(pipe_inner) => {
                    let mut inner = pipe_inner.lock();
                    loop {
                        if !inner.is_full() {
                            let written = inner.write(slice);
                            // Wake up any readers waiting for data!
                            scheduler::wake_all_blocked();
                            return written as u64;
                        }
                        
                        if inner.active_readers() == 0 {
                            return u64::MAX; // Broken pipe
                        }
                        
                        // Wait for readers to pull data!
                        drop(inner);
                        drop(file);
                        
                        let mut sched = scheduler::SCHEDULER.lock();
                        sched.current.as_mut().unwrap().state = scheduler::ProcessState::Blocked;
                        drop(sched);
                        scheduler::yield_now();
                        
                        file = file_arc.lock();
                        match &file.file_type {
                            FileType::PipeWrite(p) => inner = p.lock(),
                            _ => return u64::MAX,
                        }
                    }
                }
                _ => u64::MAX,
            }
        }
        SYS_YIELD => {
            scheduler::yield_now();
            0
        }
        SYS_GETPID => {
            let sched = scheduler::SCHEDULER.lock();
            sched.current.as_ref().map_or(0, |t| t.pid.0)
        }
        SYS_FORK => {
            scheduler::sys_fork()
        }
        SYS_EXEC => {
            let ptr = arg0 as *const u8;
            let len = arg1 as usize;
            if len > 4096 { return u64::MAX; }
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            if let Ok(path) = core::str::from_utf8(slice) {
                if let Err(e) = scheduler::sys_exec(path) {
                    crate::log_error!("sys_exec failed: {}", e);
                    u64::MAX
                } else {
                    unreachable!()
                }
            } else {
                u64::MAX
            }
        }
        SYS_WAIT => {
            let target_pid = arg0;
            scheduler::sys_wait(target_pid)
        }
        SYS_OPEN => {
            let ptr = arg0 as *const u8;
            let len = arg1 as usize;
            if len > 4096 { return u64::MAX; }
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            let path = core::str::from_utf8(slice).unwrap_or("");
            
            // FIXME: This is a simplfied VFS pass-through focusing only on FAT32 for Phase 5.4 requirements
            // A real VFS open would return an Inode handle. Here we just assume it's valid if length > 0
            if path.len() == 0 { return u64::MAX; }
            
            use alloc::sync::Arc;
            use spin::Mutex;
            use crate::fs::fd::File;
            
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            // Find free FD
            let mut fd = None;
            for i in 0..64 {
                if current.fd_table[i].is_none() {
                    fd = Some(i);
                    break;
                }
            }
            
            if let Some(fd_idx) = fd {
                current.fd_table[fd_idx] = Some(File::new_regular(path, true, true));
                fd_idx as u64
            } else {
                u64::MAX // Table Full
            }
        }
        SYS_CLOSE => {
            let fd = arg0 as usize;
            if fd >= 64 { return u64::MAX; }
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            // Drop Reference
            current.fd_table[fd] = None;
            0
        }
        SYS_DUP => {
            let old_fd = arg0 as usize;
            if old_fd >= 64 { return u64::MAX; }
            
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            // Get Arc pointing to original file
            if let Some(file_arc) = current.fd_table[old_fd].clone() {
                // Find next free FD
                for i in 0..64 {
                    if current.fd_table[i].is_none() {
                        current.fd_table[i] = Some(file_arc); // Increments Arc Rc!
                        return i as u64;
                    }
                }
            }
            u64::MAX // Table full or invalid old_fd
        }
        SYS_DUP2 => {
            let old_fd = arg0 as usize;
            let new_fd = arg1 as usize;
            if old_fd >= 64 || new_fd >= 64 { return u64::MAX; }
            if old_fd == new_fd { return new_fd as u64; } // No-op
            
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            if let Some(file_arc) = current.fd_table[old_fd].clone() {
                // If there's an existing file in new_fd, this assignment safely drops its Arc
                current.fd_table[new_fd] = Some(file_arc);
                return new_fd as u64;
            }
            u64::MAX // Invalid old_fd
        }
        SYS_PIPE => {
            let fds_ptr = arg0 as *mut [u32; 2]; // Pass pointer to [u32; 2] from user
            let mut sched = scheduler::SCHEDULER.lock();
            let current = sched.current.as_mut().unwrap();
            
            // Find two available FDs
            let mut fd0 = None;
            let mut fd1 = None;
            for i in 0..64 {
                if current.fd_table[i].is_none() {
                    if fd0.is_none() { fd0 = Some(i); continue; }
                    if fd1.is_none() { fd1 = Some(i); break; }
                }
            }
            
            if fd0.is_none() || fd1.is_none() {
                return u64::MAX; // Table full
            }
            
            let fd_read = fd0.unwrap();
            let fd_write = fd1.unwrap();
            
            use alloc::sync::Arc;
            use spin::Mutex;
            use crate::fs::fd::{File, FileType};
            
            let inner = crate::fs::pipe::PipeInner::new();
            
            // Pipe initially has 1 reader and 1 writer
            inner.lock().add_reader();
            inner.lock().add_writer();
            
            let read_file = Arc::new(Mutex::new(File {
                file_type: FileType::PipeRead(inner.clone()),
                path: alloc::string::String::from("pipe"),
                offset: 0,
                readable: true,
                writable: false,
            }));
            
            let write_file = Arc::new(Mutex::new(File {
                file_type: FileType::PipeWrite(inner),
                path: alloc::string::String::from("pipe"),
                offset: 0,
                readable: false,
                writable: true,
            }));
            
            current.fd_table[fd_read] = Some(read_file);
            current.fd_table[fd_write] = Some(write_file);
            
            unsafe {
                (*fds_ptr)[0] = fd_read as u32;
                (*fds_ptr)[1] = fd_write as u32;
            }
            
            0
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

/// sys_exit: terminate the current process with dummy status 0.
pub fn sys_exit() -> ! {
    scheduler::exit_current(0);
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
    sched.current.as_ref().map_or(0, |t| t.pid.0)
}

pub fn init() {
    crate::log_info!("Syscall interface initialized.");
}
