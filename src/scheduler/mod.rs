pub mod task;
pub mod context;

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::vec;
use spin::Mutex;
use lazy_static::lazy_static;
pub use task::{Process, ProcessId, ProcessState};
use context::Context;

/// Size of each task's kernel stack (16 KiB).
const TASK_STACK_SIZE: usize = 4096 * 4;

/// The global scheduler state.
pub struct Scheduler {
    /// Currently running process (if any).
    pub current: Option<Process>,
    /// Ready queue of processes waiting to run.
    pub ready_queue: VecDeque<Process>,
    /// Next process ID to assign.
    next_id: u64,
    /// Whether the scheduler is active (context switches enabled).
    pub active: bool,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            current: None,
            ready_queue: VecDeque::new(),
            next_id: 1,
            active: false,
        }
    }

    /// Spawn a new kernel process with the given entry point and name.
    pub fn spawn(&mut self, entry: fn(), name: &str) -> ProcessId {
        let id = ProcessId(self.next_id);
        self.next_id += 1;

        // Allocate a kernel stack for the new process
        let stack = vec![0u8; TASK_STACK_SIZE].into_boxed_slice();
        let mut stack_top = stack.as_ptr() as usize + TASK_STACK_SIZE;
        stack_top &= !0xF; // STRICT 16-byte alignment

        // Build the initial context: RIP = entry, RSP = stack_top
        let ctx = Context::new(entry as u64, stack_top as u64);
        
        // Kernel processes (like Init/Shell/Threads) just use the current kernel P4 root
        // For real userspace isolation this will be customized later.
        use x86_64::registers::control::Cr3;
        let (current_p4, _) = Cr3::read();
        let current_p4_addr = current_p4.start_address().as_u64();

        let process = Process {
            pid: id,
            parent_pid: None,
            name: alloc::string::String::from(name),
            state: ProcessState::Ready,
            exit_status: None,
            children: alloc::vec::Vec::new(),
            context: ctx,
            page_table: current_p4_addr,
            _kernel_stack: stack,
            user_allocations: alloc::vec::Vec::new(),
            fd_table: create_default_fd_table(),
            _image: None,
        };

        self.ready_queue.push_back(process);
        id
    }

    /// Pick the next ready process. Returns None if queue is empty.
    pub fn schedule_next(&mut self) -> Option<Process> {
        // Find next process that is not blocked/zombie
        // For now pop_front assumes all in ready_queue are ready/running.
        // We will refine this.
        self.ready_queue.pop_front()
    }

    /// Wakes up all processes that are currently in the Blocked state.
    /// This is used heavily by the Pipe IPC mechanism so readers/writers 
    /// retry their data transfer conditions.
    pub fn wake_all_blocked(&mut self) {
        let mut any_woken = false;
        for process in self.ready_queue.iter_mut() {
            if process.state == ProcessState::Blocked {
                process.state = ProcessState::Ready;
                any_woken = true;
            }
        }
        
        // Also check if the *current* process was somehow marked Blocked
        if let Some(current) = self.current.as_mut() {
            if current.state == ProcessState::Blocked {
                current.state = ProcessState::Ready;
                any_woken = true;
            }
        }
        
        if any_woken {
            // crate::log_info!("scheduler: wake_all_blocked activated sleeping processes");
        }
    }
}

lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

/// Initialize the scheduler. Create Process 0 (kernel/shell) as the current process.
pub fn init() {
    let mut sched = SCHEDULER.lock();
    
    use x86_64::registers::control::Cr3;
    let (current_p4, _) = Cr3::read();
    let current_p4_addr = current_p4.start_address().as_u64();
    
    // Process 0 = the kernel main thread (shell). Its context will be filled on first switch.
    let kernel_process = Process {
        pid: ProcessId(0),
        parent_pid: None,
        name: alloc::string::String::from("kernel"),
        state: ProcessState::Running,
        exit_status: None,
        children: alloc::vec::Vec::new(),
        context: Context::empty(),
        page_table: current_p4_addr,
        _kernel_stack: Box::new([]),
        user_allocations: alloc::vec::Vec::new(),
        fd_table: create_default_fd_table(),
        _image: None,
    };
    sched.current = Some(kernel_process);
    sched.active = true;
    drop(sched);

    crate::log_info!("Scheduler initialized with cooperative multitasking.");
}

/// Spawn a new kernel process from anywhere in the kernel.
pub fn spawn(entry: fn(), name: &str) -> ProcessId {
    let mut sched = SCHEDULER.lock();
    let id = sched.spawn(entry, name);
    // crate::log_info!("Spawned process '{}' with PID {}", name, id.0);
    id
}

/// Spawn a completely customized process (Used by ELF loader / Fork).
/// It allows specifying a custom Page Table (CR3) and initial context.
pub fn spawn_process(name: &str, page_table: u64, entry: u64, _user_stack_top: u64, allocations: alloc::vec::Vec<(u64, u64)>) -> ProcessId {
    let mut sched = SCHEDULER.lock();
    
    let id = ProcessId(sched.next_id);
    sched.next_id += 1;

    // Allocate a separate KERNEL stack for the process (needed for Ring 3 -> Ring 0 transitions)
    let kernel_stack = vec![0u8; TASK_STACK_SIZE].into_boxed_slice();
    let mut kernel_stack_top = kernel_stack.as_ptr() as usize + TASK_STACK_SIZE;
    kernel_stack_top &= !0xF; // Enforce 16-byte hardware alignment

    // Build the initial context: RIP = trampoline or entry, but since this is 
    // for Ring 3, the jump must happen inside the trampoline.
    let ctx = Context::new(entry, kernel_stack_top as u64);

    let process = Process {
        pid: id,
        parent_pid: None,
        name: alloc::string::String::from(name),
        state: ProcessState::Ready,
        exit_status: None,
        children: alloc::vec::Vec::new(),
        context: ctx,
        page_table,
        _kernel_stack: kernel_stack,
        user_allocations: allocations,
        fd_table: create_default_fd_table(),
        _image: None,
    };

    sched.ready_queue.push_back(process);
    
    // crate::log_info!("Spawned custom process '{}' with PID {}", name, id.0);
    id
}

/// Try to cooperatively yield the CPU to the next ready task if the scheduler isn't locked.
/// This prevents Deadlocks when the Timer Interrupt fires while the kernel is holding the lock!
pub fn try_yield_now() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = match SCHEDULER.try_lock() {
            Some(lock) => lock,
            None => return, // Don't yield if scheduler is busy! (e.g. inside a syscall setup)
        };
        
        if !sched.active || sched.ready_queue.is_empty() {
            return;
        }

        if let Some(mut current) = sched.current.take() {
            let mut next = loop {
                if let Some(n) = sched.ready_queue.pop_front() {
                    if n.state == ProcessState::Ready || n.state == ProcessState::Running {
                        break n;
                    } else {
                        sched.ready_queue.push_back(n);
                    }
                } else {
                    return;
                }
            };

            current.state = ProcessState::Ready;
            next.state = ProcessState::Running;

            let mut next_stack_top = next._kernel_stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
            next_stack_top &= !0xF;
            crate::interrupts::gdt::set_tss_rsp0(next_stack_top);
            sched.ready_queue.reserve(1);
            sched.ready_queue.push_back(current);
            sched.current = Some(next);

            let current_ctx_ptr = &mut sched.ready_queue.back_mut().unwrap().context as *mut Context;
            let next_ctx_ptr = &sched.current.as_ref().unwrap().context as *const Context;

            unsafe {
                let cr3_val = sched.current.as_ref().unwrap().page_table;
                core::arch::asm!("mov cr3, {0}", in(reg) cr3_val);
            }

            let target_pid = sched.current.as_ref().unwrap().pid.0;
            drop(sched);

            unsafe { context::switch_context(current_ctx_ptr, next_ctx_ptr); }
        }
    });
}

/// Cooperatively yield the CPU to the next ready task.
pub fn yield_now() {
    // Disable interrupts during context switch for safety
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();
        if !sched.active || sched.ready_queue.is_empty() {
            return;
        }

        // Take the current process out
        if let Some(mut current) = sched.current.take() {
            // Get next process (skipping Blocked/Zombie)
            let mut next = loop {
                if let Some(n) = sched.ready_queue.pop_front() {
                    if n.state == ProcessState::Ready || n.state == ProcessState::Running {
                        break n;
                    } else {
                        sched.ready_queue.push_back(n);
                    }
                } else {
                    // This should never fully empty if idle thread exists, but safeguard
                    return;
                }
            };

            current.state = ProcessState::Ready;
            next.state = ProcessState::Running;

            // Calculate next kernel stack top
            let mut next_stack_top = next._kernel_stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
            next_stack_top &= !0xF;
            crate::interrupts::gdt::set_tss_rsp0(next_stack_top);

            // Reserve capacity to guarantee `push_back` will NOT reallocate and move structures!
            sched.ready_queue.reserve(1);

            // Put current back in queue, set next as current
            // MOVES HAPPEN HERE: We must do this BEFORE taking pointers!
            sched.ready_queue.push_back(current);
            sched.current = Some(next);

            // NOW grab the valid pointers from their permanent heap locations within the guaranteed-stable VecDeque buffer
            let current_ctx_ptr = &mut sched.ready_queue.back_mut().unwrap().context as *mut Context;
            let next_ctx_ptr = &sched.current.as_ref().unwrap().context as *const Context;

            // Load the new process's Page Table (CR3)
            unsafe {
                let cr3_val = sched.current.as_ref().unwrap().page_table;
                core::arch::asm!(
                    "mov cr3, {0}",
                    in(reg) cr3_val
                );
            }

            // Drop the lock BEFORE switching context
            let target_pid = sched.current.as_ref().unwrap().pid.0;
            drop(sched);
            
            // crate::log_info!("yield_now: switching CPU to PID {}", target_pid);

            // Perform the actual context switch via assembly
            unsafe { context::switch_context(current_ctx_ptr, next_ctx_ptr); }
        }
    });
}

/// Terminate the current process and switch to the next one.
pub fn exit_current(exit_code: u64) {
    // Disable interrupts to ensure atomicity
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();

        // 1. Remove the current process, transform to Zombie, free User allocations
        let mut finished = sched.current.take().expect("exit_current called without an active process");
        
        // crate::log_info!("Process '{}' (PID {}) exiting with code {}.", finished.name, finished.pid.0, exit_code);
        
        finished.state = ProcessState::Zombie;
        finished.exit_status = Some(exit_code);
        
        // Free user allocations!
        for (vaddr, size) in &finished.user_allocations {
            crate::memory::paging::free_user_memory(x86_64::VirtAddr::new(*vaddr), *size);
        }
        finished.user_allocations.clear();
        
        // Phase 5.4: Drop all file descriptors immediately!
        // This drops the Arc Rc. If Rc == 0, the underlying Pipe/File is cleaned up.
        // Doing this before becoming a Zombie ensures we don't leak FDs and signal EOF to readers.
        for slot in finished.fd_table.iter_mut() {
            *slot = None;
        }
        
        // Wake up Parent if it was waiting
        if let Some(parent_pid) = finished.parent_pid {
            for proc in sched.ready_queue.iter_mut() {
                if proc.pid == parent_pid && proc.state == ProcessState::Blocked {
                    proc.state = ProcessState::Ready;
                    break;
                }
            }
        }

        // Put the Zombie back in the list so `wait` can find it later
        sched.ready_queue.push_back(finished);

        // 2. We MUST switch to the next task now
        // Get next process (skipping Blocked/Zombie)
        let mut next = loop {
            if let Some(n) = sched.ready_queue.pop_front() {
                if n.state == ProcessState::Ready || n.state == ProcessState::Running {
                    break n;
                } else {
                    sched.ready_queue.push_back(n);
                }
            } else {
                // No tasks left at all (not even the shell).
                // crate::log_info!("All tasks finished. System halted.");
                drop(sched);
                loop { x86_64::instructions::hlt(); }
            }
        };

        next.state = ProcessState::Running;
            
        let mut next_stack_top = next._kernel_stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
        next_stack_top &= !0xF;
        crate::interrupts::gdt::set_tss_rsp0(next_stack_top);
            
        // We must place it in `sched.current` before getting its context pointer.
        sched.current = Some(next);
            
        // Get the raw pointer to the next context IN its new memory location.
        let next_ctx_ptr = &sched.current.as_ref().unwrap().context as *const Context;
            
        // Load the new process's Page Table (CR3)
        unsafe {
            let cr3_val = sched.current.as_ref().unwrap().page_table;
            core::arch::asm!(
                "mov cr3, {0}",
                in(reg) cr3_val
            );
        }

        // Drop scheduler lock before jumping
        drop(sched);

        // 3. Jump to the next task without saving the current state
        unsafe {
            context::restore_context(next_ctx_ptr);
        }
    });

    unreachable!("exit_current should never return");
}

/// Get a snapshot of all processes for display purposes (used by `ps` command).
pub fn list_tasks() -> alloc::vec::Vec<(u64, alloc::string::String, alloc::string::String)> {
    let sched = SCHEDULER.lock();
    let mut result = alloc::vec::Vec::new();

    if let Some(ref current) = sched.current {
        result.push((current.pid.0, current.name.clone(), alloc::string::String::from("running")));
    }
    for proc in &sched.ready_queue {
        result.push((proc.pid.0, proc.name.clone(), alloc::format!("{:?}", proc.state)));
    }

    result
}

/// Syscall fork: Duplicate the current process (parent) into a new running process (child).
/// Returns Child PID to Parent, 0 to Child.
pub fn sys_fork() -> u64 {
    let mut sched = SCHEDULER.lock();
    
    // Extract everything we need from current to drop the borrow
    let (parent_pid, parent_name, child_allocations, parent_stack_ptr, parent_image, parent_fd_table) = {
        let current_proc = match sched.current.as_ref() {
            Some(p) => p,
            None => return u64::MAX,
        };
        (
            current_proc.pid,
            current_proc.name.clone(),
            current_proc.user_allocations.clone(),
            current_proc._kernel_stack.as_ptr(),
            None, // Phase 5.3 memory mapping isolates physical frames manually, no need to clone the legacy image!
            current_proc.fd_table.clone()
        )
    };
    
    // crate::log_info!("sys_fork: allocating P4 phys...");
    
    // 2. Clone the User Page Table and Allocations
    let child_p4_phys = match crate::memory::paging::create_new_page_table() {
        Some(addr) => addr,
        None => return u64::MAX, // Out of memory
    };
    
    // crate::log_info!("sys_fork: deep_clone_process_memory started...");
    
    // Execute Deep Copy of physical Memory Frames!
    if !crate::memory::paging::deep_clone_process_memory(child_p4_phys, &child_allocations) {
        crate::log_error!("sys_fork: Failed to deep copy memory frames!");
        return u64::MAX;
    }
    
    // crate::log_info!("sys_fork: P4 clone finished! Allocating child kernel stack...");
    
    // 3. Allocate a fresh independent Kernel Stack for the child
    let child_kernel_stack = vec![0u8; TASK_STACK_SIZE].into_boxed_slice();
    let mut child_stack_top = child_kernel_stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
    child_stack_top &= !0xF; // Strict 16-byte boundary

    // 4. Copy the User Context (TrapFrame) exactly
    // Subtract 152 bytes (19 * 8 bytes) to match exactly what is pushed by the CPU + syscall handler!
    let mut parent_stack_top = parent_stack_ptr as u64 + TASK_STACK_SIZE as u64;
    parent_stack_top &= !0xF;
    
    let trap_frame_ptr = (parent_stack_top - 152) as *const TrapFrame;
    let trap_frame = unsafe { *trap_frame_ptr };
    
    let child_trap_frame_ptr = (child_stack_top - 152) as *mut TrapFrame;
    unsafe { *child_trap_frame_ptr = trap_frame; }
    
    // Set child's Context to resume at `fork_trampoline` with RSP pointing at the TrapFrame
    let mut child_context = Context::empty();
    child_context.rsp = child_stack_top - 152;
    child_context.rip = fork_trampoline as *const () as u64;
    
    // 5. Construct Process
    let child_pid = ProcessId(sched.next_id);
    sched.next_id += 1;
    
    let child_name = alloc::format!("{}_child", parent_name);

    let child_process = Process {
        pid: child_pid,
        parent_pid: Some(parent_pid),
        name: child_name,
        state: ProcessState::Ready,
        exit_status: None,
        children: alloc::vec::Vec::new(),
        context: child_context,
        page_table: child_p4_phys.as_u64(),
        _kernel_stack: child_kernel_stack,
        user_allocations: child_allocations,
        fd_table: parent_fd_table, // Exact clone()! Bumps Arc ref counts seamlessly!
        _image: parent_image,
    };
    
    // 6. Push Child to Parent list and scheduler
    let current_proc_mut = sched.current.as_mut().unwrap();
    current_proc_mut.children.push(child_pid);
    
    sched.ready_queue.push_back(child_process);
    
    // crate::log_info!("sys_fork: Process {} created Child Process {}", parent_pid.0, child_pid.0);
    
    child_pid.0
}

/// Syscall exec: Replace the current process with a new ELF binary.
/// On success it NEVER returns here, it jumps manually into the new program.
/// Returns only if there was an error loading the file.
pub fn sys_exec(path: &str) -> Result<(), crate::loader::elf::ExecError> {
    // CRITICAL: Copy path into kernel-owned memory BEFORE we free user pages!
    // `path` is a &str pointing into user-space memory which will be unmapped below.
    let owned_path = alloc::string::String::from(path);
    
    // 1. Construct the new User Image Memory Map
    let params = match crate::loader::elf::parse_and_map_elf(&owned_path) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    // crate::log_info!("sys_exec: replacing current process with '{}'", owned_path);

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();

        let current = sched.current.as_mut().expect("sys_exec called without active process!");
        
        // 2. Free old virtual memory allocations
        for (vaddr, size) in &current.user_allocations {
            crate::memory::paging::free_user_memory(x86_64::VirtAddr::new(*vaddr), *size);
        }

        // 3. Swap in new Page Table and Allocations
        current.page_table = params.page_table;
        current.user_allocations = params.allocations;
        current.name = owned_path;
        
        // 4. Reset the Kernel Stack to a clean slate over the current frame!
        // We reset `current.context.rsp` to the top of the kernel stack where a fresh
        // Ring 3 trampoline will be orchestrated. 
        let mut kernel_stack_top = current._kernel_stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
        kernel_stack_top &= !0xF;
        
        // We use the `Context` struct purely to point to the trampoline inside ring 0!
        current.context = Context::new(crate::loader::elf::usermode_trampoline as *const () as u64, kernel_stack_top);
        
        // Inject R12 and R13 for trampoline usage
        current.context.r12 = params.entry;
        current.context.r13 = params.user_stack_top;
        
        // Securely prepare CPU for context replacement
        crate::interrupts::gdt::set_tss_rsp0(kernel_stack_top);
        
        // 5. Explicitly Load the New CR3
        unsafe {
            core::arch::asm!(
                "mov cr3, {0}",
                in(reg) current.page_table
            );
        }

        let next_ctx_ptr = &current.context as *const Context;
        
        // 6. Jump linearly into the trampoline (Wipes out old Syscall state!)
        drop(sched);
        unsafe {
            crate::scheduler::context::restore_context(next_ctx_ptr);
        }
    });

    unreachable!("sys_exec should never return on success");
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    pub rcx: u64,
    pub rbx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[unsafe(naked)]
pub extern "C" fn fork_trampoline() {
    unsafe {
        core::arch::naked_asm!(
            "xor rax, rax",  // Return 0 for child!
            "pop rcx",
            "pop rbx",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rbp",
            "pop r8",
            "pop r9",
            "pop r10",
            "pop r11",
            "pop r12",
            "pop r13",
            "pop r14",
            "pop r15",
            "iretq"
        );
    }
}

/// Syscall wait: Wait for a child process to change state to Zombie, then reap it.
/// If `target_pid` is u64::MAX (-1), wait for ANY child.
/// Returns the Exit Status of the child, or u64::MAX if no children exist.
pub fn sys_wait(target_pid: u64) -> u64 {
    loop {
        let mut sched = SCHEDULER.lock();
        let current_pid = sched.current.as_ref().map(|p| p.pid).unwrap_or(ProcessId(0));
        
        let mut child_found = false;
        let mut reaped_pid = None;
        let mut reaped_status = 0;

        // 1. Scan the ready_queue for matching Zombie children
        for i in 0..sched.ready_queue.len() {
            let proc = &sched.ready_queue[i];
            
            // Is it our child?
            if proc.parent_pid == Some(current_pid) {
                if target_pid == u64::MAX || proc.pid.0 == target_pid {
                    child_found = true;
                    if proc.state == ProcessState::Zombie {
                        reaped_pid = Some(proc.pid);
                        reaped_status = proc.exit_status.unwrap_or(0);
                        break;
                    }
                }
            }
        }

        if let Some(pid) = reaped_pid {
            // A Zombie was found! We must reap it (Remove it entirely from scheduler)
            sched.ready_queue.retain(|p| p.pid != pid);
            
            // Remove it from current process's children tracking list
            if let Some(current) = sched.current.as_mut() {
                current.children.retain(|&c| c != pid);
            }
            
            // crate::log_info!("sys_wait: Process {} reaped Zombie child {}", current_pid.0, pid.0);
            return reaped_status;
        }

        if !child_found {
            // No matching children exist computationally. Return error.
            return u64::MAX;
        }

        // 2. Child exists but is still Running/Ready. We must BLOCK and yield!
        if let Some(current) = sched.current.as_mut() {
            current.state = ProcessState::Blocked;
        }
        
        drop(sched);
        
        // Explicitly enable interrupts before yielding so the Timer can preempt us!
        // We are inside an int 0x80 gate where IF=0. If we don't enable it, IF remains 0 
        // after the context switch to other ring 0 tasks.
        x86_64::instructions::interrupts::enable();
        
        // Wait efficiently for the next interrupt (like a Timer Tick) to fire, avoiding 100% CPU loops!
        x86_64::instructions::hlt();
        
        yield_now();
    }
}

/// Helper method to create a clean FD Table pointing to the Console for Stdin/Stdout/Stderr
fn create_default_fd_table() -> alloc::vec::Vec<Option<alloc::sync::Arc<spin::Mutex<crate::fs::fd::File>>>> {
    use crate::fs::fd::File;
    let mut table = alloc::vec::Vec::with_capacity(64);
    for _ in 0..64 {
        table.push(None); // Empty table slots
    }
    table[0] = Some(File::new_console()); // STDIN
    table[1] = Some(File::new_console()); // STDOUT
    table[2] = Some(File::new_console()); // STDERR
    table
}

/// Global wrapper to wake up all blocked tasks (e.g., when pipe data arrives or space frees).
pub fn wake_all_blocked() {
    // try_lock used because this is often called mid-syscall when the lock might already
    // be taken, or just before another lock sequence.
    if let Some(mut sched) = SCHEDULER.try_lock() {
        sched.wake_all_blocked();
    }
}
