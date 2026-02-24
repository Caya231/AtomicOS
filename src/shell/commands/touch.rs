use crate::println;

/// touch <path> — create an empty file via VFS.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("touch: missing file operand");
        return;
    }

    let full = crate::shell::state::resolve_path(path);
    let mut vfs = crate::fs::VFS.lock();
    match vfs.create(&full) {
        Ok(_) => println!("Created: {}", path),
        Err(e) => println!("touch: {}: {}", path, e),
    }
}
