use crate::println;

pub fn run(_args: &str) {
    println!("AtomicOS v0.2.0 (x86_64)");
    println!("Kernel:  Rust no_std + alloc");
    println!("Boot:    Multiboot2 / GRUB");
    println!("Build:   GNU Toolchain (nasm + ld)");
}
