use crate::println;

/// exec — load and execute an ELF64 binary from disk.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("Usage: exec <path>");
        println!("  Example: exec /disk/test.elf");
        return;
    }

    println!("[EXEC] Loading {}...", path);

    match crate::loader::elf::load(path) {
        Ok(task_id) => {
            println!("[EXEC] Loaded '{}' as task {}", path, task_id);
            println!("[EXEC] Use 'yield' to switch to it, 'ps' to see tasks.");
        }
        Err(e) => {
            println!("[EXEC] Failed: {}", e);
        }
    }
}
