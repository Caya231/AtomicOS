use crate::println;

/// Simulated cat. Prints placeholder content for known filenames.
pub fn run(args: &str) {
    let filename = args.trim();
    if filename.is_empty() {
        println!("cat: missing filename");
        return;
    }

    match filename {
        "README.md" => {
            println!("# AtomicOS");
            println!("A hobby x86_64 kernel written in Rust.");
        },
        "BUILD.md" => {
            println!("# Build Dependencies");
            println!("nasm, ld, gcc, grub-mkrescue, qemu, rust nightly");
        },
        _ => println!("cat: {}: No such file", filename),
    }
}
