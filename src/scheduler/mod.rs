pub mod task;
pub mod context;

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::vec;
use spin::Mutex;
use lazy_static::lazy_static;
use task::{Task, TaskId, TaskState};
use context::Context;

/// Size of each task's kernel stack (16 KiB).
const TASK_STACK_SIZE: usize = 4096 * 4;

/// The global scheduler state.
pub struct Scheduler {
    /// Currently running task (if any).
    pub current: Option<Task>,
    /// Ready queue of tasks waiting to run.
    pub ready_queue: VecDeque<Task>,
    /// Next task ID to assign.
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

    /// Spawn a new task with the given entry point and name.
    pub fn spawn(&mut self, entry: fn(), name: &str) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        // Allocate a stack for the new task
        let stack = vec![0u8; TASK_STACK_SIZE].into_boxed_slice();
        let stack_top = stack.as_ptr() as usize + TASK_STACK_SIZE;

        // Build the initial context: RIP = entry, RSP = stack_top
        let ctx = Context::new(entry as u64, stack_top as u64);

        let task = Task {
            id,
            name: alloc::string::String::from(name),
            state: TaskState::Ready,
            context: ctx,
            _stack: stack,
            _image: None,
        };

        self.ready_queue.push_back(task);
        id
    }

    /// Pick the next ready task. Returns None if queue is empty.
    pub fn schedule_next(&mut self) -> Option<Task> {
        self.ready_queue.pop_front()
    }

    /// Spawn a task from a raw entry address (used by ELF loader).
    /// The `image` holds the program memory and is kept alive as the task's stack owner.
    pub fn spawn_raw(&mut self, entry: u64, name: &str, image: alloc::vec::Vec<u8>) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        // Allocate a separate stack for the program
        let stack = vec![0u8; TASK_STACK_SIZE].into_boxed_slice();
        let stack_top = stack.as_ptr() as usize + TASK_STACK_SIZE;

        let ctx = Context::new(entry, stack_top as u64);

        let task = Task {
            id,
            name: alloc::string::String::from(name),
            state: TaskState::Ready,
            context: ctx,
            _stack: stack,
            _image: Some(image.into_boxed_slice()), // keep program memory alive
        };

        self.ready_queue.push_back(task);
        id
    }
}

lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

/// Initialize the scheduler. Create Task 0 (kernel/shell) as the current task.
pub fn init() {
    let mut sched = SCHEDULER.lock();
    // Task 0 = the kernel main thread (shell). Its context will be filled on first switch.
    let kernel_task = Task {
        id: TaskId(0),
        name: alloc::string::String::from("kernel"),
        state: TaskState::Running,
        context: Context::empty(),
        _stack: Box::new([]),
        _image: None,
    };
    sched.current = Some(kernel_task);
    sched.active = true;
    drop(sched);

    crate::log_info!("Scheduler initialized with cooperative multitasking.");
}

/// Spawn a new task from anywhere in the kernel.
pub fn spawn(entry: fn(), name: &str) -> TaskId {
    let mut sched = SCHEDULER.lock();
    let id = sched.spawn(entry, name);
    crate::log_info!("Spawned task '{}' with ID {}", name, id.0);
    id
}

/// Cooperatively yield the CPU to the next ready task.
pub fn yield_now() {
    // Disable interrupts during context switch for safety
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();
        if !sched.active || sched.ready_queue.is_empty() {
            return;
        }

        // Take the current task out
        if let Some(mut current) = sched.current.take() {
            // Get next task
            let mut next = sched.ready_queue.pop_front().unwrap();

            current.state = TaskState::Ready;
            next.state = TaskState::Running;

            // Calculate next kernel stack top
            let next_stack_top = next._stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
            crate::interrupts::gdt::set_tss_rsp0(next_stack_top);

            // Put current back in queue, set next as current
            // MOVES HAPPEN HERE: We must do this BEFORE taking pointers!
            sched.ready_queue.push_back(current);
            sched.current = Some(next);

            // NOW grab the valid pointers from their permanent heap locations
            let current_ctx_ptr = &mut sched.ready_queue.back_mut().unwrap().context as *mut Context;
            let next_ctx_ptr = &sched.current.as_ref().unwrap().context as *const Context;

            // Drop the lock BEFORE switching context
            drop(sched);

            // Perform the actual context switch via assembly
            unsafe { context::switch_context(current_ctx_ptr, next_ctx_ptr); }
        }
    });
}

/// Terminate the current task and switch to the next one.
pub fn exit_current() {
    // Disable interrupts to ensure atomicity
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();

        // 1. Remove the current task. It will be dropped and its memory freed.
        let finished = sched.current.take();
        if let Some(t) = &finished {
            crate::log_info!("Task '{}' (ID {}) exited.", t.name, t.id.0);
        }
        drop(finished);

        // 2. We MUST switch to the next task now, since the current one is dead.
        if let Some(mut next) = sched.ready_queue.pop_front() {
            next.state = TaskState::Running;
            
            let next_stack_top = next._stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
            crate::interrupts::gdt::set_tss_rsp0(next_stack_top);
            
            // We must place it in `sched.current` before getting its context pointer.
            sched.current = Some(next);
            
            // Get the raw pointer to the next context IN its new memory location.
            let next_ctx_ptr = &sched.current.as_ref().unwrap().context as *const Context;
            
            // Drop scheduler lock before jumping
            drop(sched);

            // 3. Jump to the next task without saving the current state (since it's dead).
            unsafe {
                context::restore_context(next_ctx_ptr);
            }
        } else {
            // No tasks left at all (not even the shell).
            crate::log_info!("All tasks finished. System halted.");
            drop(sched);
            loop {
                x86_64::instructions::hlt();
            }
        }
    });

    unreachable!("exit_current should never return");
}

/// Get a snapshot of all tasks for display purposes (used by `ps` command).
pub fn list_tasks() -> alloc::vec::Vec<(u64, alloc::string::String, alloc::string::String)> {
    let sched = SCHEDULER.lock();
    let mut result = alloc::vec::Vec::new();

    if let Some(ref current) = sched.current {
        result.push((current.id.0, current.name.clone(), alloc::string::String::from("running")));
    }
    for task in &sched.ready_queue {
        result.push((task.id.0, task.name.clone(), alloc::format!("{:?}", task.state)));
    }

    result
}
