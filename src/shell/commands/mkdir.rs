use crate::println;

/// mkdir <path> — create a directory via VFS.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("mkdir: missing operand");
        return;
    }

    let full = crate::shell::state::resolve_path(path);
    let mut vfs = crate::fs::VFS.lock();
    match vfs.mkdir(&full) {
        Ok(_) => println!("Created directory: {}", path),
        Err(e) => println!("mkdir: {}: {}", path, e),
    }
}
