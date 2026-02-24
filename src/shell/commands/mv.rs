use crate::println;
use alloc::string::String;
use super::super::state;

/// mv <src> <dst> — move/rename a file in the in-memory filesystem.
pub fn run(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        println!("mv: usage: mv <source> <dest>");
        return;
    }

    let src = if parts[0].starts_with('/') { String::from(parts[0]) } else { alloc::format!("/{}", parts[0]) };
    let dst = if parts[1].starts_with('/') { String::from(parts[1]) } else { alloc::format!("/{}", parts[1]) };

    let mut fs = state::MEMFS.lock();
    let content = match fs.files.remove(&src) {
        Some(c) => c,
        None => { println!("mv: '{}': No such file", parts[0]); return; }
    };

    fs.files.insert(dst, content);
    println!("Moved {} -> {}", parts[0], parts[1]);
    state::log_cmd(&alloc::format!("mv {} {}", parts[0], parts[1]));
}
