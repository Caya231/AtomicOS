use crate::println;

/// rm <path> — remove a file or empty directory via VFS.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("rm: missing operand");
        return;
    }

    let full = crate::shell::state::resolve_path(path);
    let mut vfs = crate::fs::VFS.lock();
    match vfs.unlink(&full) {
        Ok(()) => println!("Removed: {}", path),
        Err(e) => println!("rm: {}: {}", path, e),
    }
}
