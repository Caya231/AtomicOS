use alloc::boxed::Box;
use alloc::string::String;
use super::context::Context;

/// Unique task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskId(pub u64);

/// Task state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Finished,
}

/// A single task (thread of execution).
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TaskState,
    pub context: Context,
    /// Owned stack memory — kept alive as long as the task exists.
    pub _stack: Box<[u8]>,
    /// Optional program image memory (ELF loaded programs).
    pub _image: Option<Box<[u8]>>,
}
