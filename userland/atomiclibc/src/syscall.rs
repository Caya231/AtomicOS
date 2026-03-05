use core::arch::asm;

// Syscall wrapper
#[inline(always)]
pub unsafe fn syscall0(n: u64) -> u64 {
    let ret: u64;
    asm!(
        "int 0x80",
        in("rax") n,
        out("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall1(n: u64, a1: u64) -> u64 {
    let ret: u64;
    asm!(
        "int 0x80",
        in("rax") n,
        in("rdi") a1,
        out("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall2(n: u64, a1: u64, a2: u64) -> u64 {
    let ret: u64;
    asm!(
        "int 0x80",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        out("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall3(n: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let ret: u64;
    asm!(
        "int 0x80",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        out("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}
