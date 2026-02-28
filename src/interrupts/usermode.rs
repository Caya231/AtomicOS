/// Usermode support — int 0x80 syscall handler and Ring 3 transition.

use core::arch::naked_asm;

/// The int 0x80 handler — entered from Ring 3.
/// Saves user registers, calls Rust syscall dispatcher, restores and iretq back.
///
/// Convention: RAX=syscall number, RDI=arg0, RSI=arg1, RDX=arg2
/// Returns: RAX=result
#[unsafe(naked)]
pub extern "C" fn syscall_handler_asm() {
    naked_asm!(
        // Save all general-purpose registers
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push r11",
        "push r10",
        "push r9",
        "push r8",
        "push rbp",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbx",
        "push rcx",

        // Align the stack strictly to 16-bytes as required by System V AMD64 ABI
        // CPU pushes 5 QWORDS (40 bytes), we push 14 QWORDS (112 bytes) = 152 bytes total.
        // 152 is not divisible by 16! It's off by 8 bytes.
        "sub rsp, 8",

        // Call Rust dispatcher: dispatch(rax, rdi, rsi, rdx)
        // System V ABI: arg0=rdi, arg1=rsi, arg2=rdx, arg3=rcx
        // We need: rdi=number(was rax), rsi=arg0(was rdi), rdx=arg1(was rsi), rcx=arg2(was rdx)
        "mov rcx, rdx",   // arg2 → rcx (4th param)
        "mov rdx, rsi",   // arg1 → rdx (3rd param)
        "mov rsi, rdi",   // arg0 → rsi (2nd param)
        "mov rdi, rax",   // number → rdi (1st param)
        "call {dispatch}",

        // Un-align stack before resuming context POP routines
        "add rsp, 8",

        // Return value is in RAX — it'll be restored to user's RAX

        // Restore registers (skip rcx and rbx — we use rax as return)
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

        "iretq",
        dispatch = sym crate::syscalls::dispatch,
    );
}

/// Jump to Ring 3 and execute user code.
/// Pushes the iretq frame: SS, RSP, RFLAGS, CS, RIP.
pub fn jump_to_usermode(entry: u64, user_stack_top: u64, user_cs: u16, user_ss: u16) {
    unsafe {
        core::arch::asm!(
            "cli",                  // Disable interrupts during transition
            "push rax",             // SS (user data segment)
            "push rcx",             // RSP (user stack)
            "pushfq",               // RFLAGS — will set IF below
            "pop r11",
            "or r11, 0x200",        // Set IF (interrupt enable)
            "push r11",
            "push rdx",             // CS (user code segment)
            "push rdi",             // RIP (entry point)
            "iretq",
            in("rdi") entry,
            in("rcx") user_stack_top,
            in("rdx") user_cs as u64,
            in("rax") user_ss as u64,
            options(noreturn),
        );
    }
}
