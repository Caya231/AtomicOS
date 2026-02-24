use crate::println;

/// Simulated filesystem listing. Will be expanded when a real FS is implemented.
const FAKE_FILES: &[&str] = &[
    "kernel.bin",
    "grub.cfg",
    "boot.asm",
    "README.md",
    "BUILD.md",
];

pub fn run(_args: &str) {
    for f in FAKE_FILES {
        println!("  {}", f);
    }
}
