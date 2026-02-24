use crate::println;
use alloc::string::String;
use super::super::state;

/// mkdir <path> — create a directory in the in-memory filesystem.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("mkdir: missing operand");
        return;
    }

    let full = if path.starts_with('/') { String::from(path) } else { alloc::format!("/{}", path) };

    let mut fs = state::MEMFS.lock();
    if fs.exists(&full) {
        println!("mkdir: cannot create '{}': File exists", path);
    } else {
        fs.files.insert(full, None); // None = directory
        println!("Created directory: {}", path);
        state::log_cmd(&alloc::format!("mkdir {}", path));
    }
}
