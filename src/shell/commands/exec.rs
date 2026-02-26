use crate::println;

/// exec — load and execute an ELF64 binary from disk.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("Usage: exec <path>");
        return;
    }

    println!("[EXEC] Loading {}...", path);
    crate::log_info!("[EXEC] Loading {}...", path);

    match crate::loader::elf::load(path) {
        Ok(task_id) => {
            println!("[EXEC] Loaded '{}' as task {}", path, task_id);
            crate::log_info!("[EXEC] Loaded '{}' as task {}", path, task_id);
            
            // Automatically switch to the newly loaded program
            crate::scheduler::yield_now();
        }
        Err(e) => {
            println!("[EXEC] Failed: {}", e);
            crate::log_info!("[EXEC] Failed: {}", e);
        }
    }
}
