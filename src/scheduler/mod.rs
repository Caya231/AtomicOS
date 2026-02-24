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
            _stack: stack, // ownership kept so stack isn't freed
        };

        self.ready_queue.push_back(task);
        id
    }

    /// Pick the next ready task. Returns None if queue is empty.
    pub fn schedule_next(&mut self) -> Option<Task> {
        self.ready_queue.pop_front()
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
        _stack: Box::new([]), // kernel uses the boot stack, not a heap stack
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
            current.state = TaskState::Ready;
            let current_ctx_ptr = &mut current.context as *mut Context;

            // Get next task
            let mut next = sched.ready_queue.pop_front().unwrap();
            next.state = TaskState::Running;
            let next_ctx_ptr = &next.context as *const Context;

            // Put current back in queue, set next as current
            sched.ready_queue.push_back(current);
            sched.current = Some(next);

            // Drop the lock BEFORE switching context
            drop(sched);

            // Perform the actual context switch via assembly
            unsafe { context::switch_context(current_ctx_ptr, next_ctx_ptr); }
        }
    });
}

/// Terminate the current task and switch to the next one.
pub fn exit_current() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut sched = SCHEDULER.lock();

        // Drop current task (its stack will be freed)
        let finished = sched.current.take();
        if let Some(t) = &finished {
            crate::log_info!("Task '{}' (ID {}) exited.", t.name, t.id.0);
        }
        drop(finished);

        // Switch to next or back to idle
        if let Some(mut next) = sched.ready_queue.pop_front() {
            next.state = TaskState::Running;
            let next_ctx_ptr = &next.context as *const Context;
            sched.current = Some(next);
            drop(sched);

            // Jump to the next task (no save needed, current is dead)
            unsafe { context::restore_context(next_ctx_ptr); }
        } else {
            // No tasks left — return to kernel idle
            crate::log_info!("All tasks finished. Returning to shell.");
            drop(sched);
        }
    });
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
