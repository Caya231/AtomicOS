use crate::println;

/// objdump — display info about the running kernel ELF binary.
pub fn run(_args: &str) {
    println!("kernel.bin: file format elf64-x86-64");
    println!("");
    println!("Sections:");
    println!("  Idx  Name          Size       VMA");
    println!("    0  .boot         00001000   0000000000100000");
    println!("    1  .text         00010000   0000000000101000");
    println!("    2  .rodata       00004000   0000000000111000");
    println!("    3  .data         00002000   0000000000115000");
    println!("    4  .bss          00008000   0000000000117000");
    println!("");
    println!("SYMBOL TABLE (excerpt):");
    println!("  0000000000100000  _start");
    println!("  0000000000101000  vga::init");
    println!("  0000000000101200  serial::init");
    println!("  0000000000102000  interrupts::init");
    println!("  0000000000103000  memory::init");
    println!("  0000000000104000  shell::exec_command");
    println!("");
    println!("(simulated — will read real ELF headers with loader)");
}
