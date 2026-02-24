use crate::println;
use alloc::string::String;
use super::super::state;

/// rm <path> — remove a file or directory from in-memory fs.
pub fn run(args: &str) {
    let path = args.trim();
    if path.is_empty() {
        println!("rm: missing operand");
        return;
    }

    let full = if path.starts_with('/') { String::from(path) } else { alloc::format!("/{}", path) };

    if full == "/" {
        println!("rm: cannot remove root directory");
        return;
    }

    let mut fs = state::MEMFS.lock();
    if fs.files.remove(&full).is_some() {
        println!("Removed: {}", path);
        state::log_cmd(&alloc::format!("rm {}", path));
    } else {
        println!("rm: cannot remove '{}': No such file or directory", path);
    }
}
