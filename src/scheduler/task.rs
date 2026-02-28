use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use super::context::Context;

/// Unique process identifier (PID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(pub u64);

/// Process state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

/// A single process unit.
pub struct Process {
    pub pid: ProcessId,
    pub parent_pid: Option<ProcessId>,
    pub name: String,
    pub state: ProcessState,
    pub exit_status: Option<u64>,
    pub children: Vec<ProcessId>,
    pub context: Context,
    
    // Address Space Root Table PTR (CR3) for this process
    pub page_table: u64,
    
    /// Owned kernel stack memory — kept alive as long as the process exists.
    pub _kernel_stack: Box<[u8]>,
    
    // Virtual Memory Blocks dynamically allocated to User (Tracked for cleanup)
    pub user_allocations: Vec<(u64, u64)>, // (VirtAddr_Start, Size)

    /// Process File Descriptor Table
    pub fd_table: Vec<Option<alloc::sync::Arc<spin::Mutex<crate::fs::fd::File>>>>,

    /// Optional program image memory (For legacy compatibility before full VFS elf parsing is moved to Page Mapping)
    pub _image: Option<Box<[u8]>>,
}
