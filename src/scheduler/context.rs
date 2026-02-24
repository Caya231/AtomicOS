use core::arch::{asm, naked_asm};

/// CPU register context saved/restored during context switches.
/// All callee-saved registers on x86_64 System V ABI.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Context {
    pub rsp: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
}

impl Context {
    /// Create an empty context (used for kernel boot task).
    pub fn empty() -> Self {
        Context {
            rsp: 0, rbp: 0, rbx: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0,
        }
    }

    /// Create a new context for a fresh task.
    /// `entry` = function pointer, `stack_top` = top of the allocated stack.
    pub fn new(entry: u64, stack_top: u64) -> Self {
        // Stack must be 16-byte aligned per System V ABI.
        // We push a fake return address (task_entry_trampoline) on the stack
        // so that when we "ret" into this context, it jumps to entry.
        let aligned_sp = (stack_top - 8) & !0xF; // align to 16, minus 8 for the return addr

        Context {
            rsp: aligned_sp,
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: entry,
        }
    }
}

/// Switch context from `old` to `new`.
/// Saves callee-saved registers into `old`, restores from `new`.
///
/// # Safety
/// Both pointers must be valid Context structs with valid stack pointers.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: *mut Context, new: *const Context) {
    naked_asm!(
        // Save callee-saved registers into `old` (rdi = old ptr)
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], rbp",
        "mov [rdi + 0x10], rbx",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], r13",
        "mov [rdi + 0x28], r14",
        "mov [rdi + 0x30], r15",
        // Save return address as RIP
        "lea rax, [rip + 2f]",
        "mov [rdi + 0x38], rax",

        // Restore callee-saved registers from `new` (rsi = new ptr)
        "mov rsp, [rsi + 0x00]",
        "mov rbp, [rsi + 0x08]",
        "mov rbx, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov r13, [rsi + 0x20]",
        "mov r14, [rsi + 0x28]",
        "mov r15, [rsi + 0x30]",

        // Jump to the new task's RIP
        "jmp [rsi + 0x38]",

        // This is where we return when switched back to `old`
        "2:",
        "ret",
    );
}

/// Restore context without saving (used when current task is dead).
///
/// # Safety
/// The context pointer must be valid.
#[unsafe(naked)]
pub unsafe extern "C" fn restore_context(new: *const Context) {
    naked_asm!(
        // rdi = new context ptr
        "mov rsp, [rdi + 0x00]",
        "mov rbp, [rdi + 0x08]",
        "mov rbx, [rdi + 0x10]",
        "mov r12, [rdi + 0x18]",
        "mov r13, [rdi + 0x20]",
        "mov r14, [rdi + 0x28]",
        "mov r15, [rdi + 0x30]",
        "jmp [rdi + 0x38]",
    );
}
