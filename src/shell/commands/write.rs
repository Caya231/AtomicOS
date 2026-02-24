use crate::println;

/// write <path> <text> — write text content to a file via VFS.
/// Creates the file if it doesn't exist.
pub fn run(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().splitn(2, ' ').collect();
    if parts.len() < 2 {
        println!("write: usage: write <path> <text>");
        return;
    }

    let path = crate::shell::state::resolve_path(parts[0]);
    let content = parts[1];

    let mut vfs = crate::fs::VFS.lock();

    // Create file if it doesn't exist
    if !vfs.exists(&path) {
        if let Err(e) = vfs.create(&path) {
            println!("write: create error: {}", e);
            return;
        }
    }

    match vfs.write_file(&path, content.as_bytes()) {
        Ok(n) => println!("Wrote {} bytes to {}", n, parts[0]),
        Err(e) => println!("write: {}: {}", parts[0], e),
    }
}
