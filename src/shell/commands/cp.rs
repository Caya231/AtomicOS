use crate::println;
use alloc::string::String;
use super::super::state;

/// cp <src> <dst> — copy a file in the in-memory filesystem.
pub fn run(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        println!("cp: usage: cp <source> <dest>");
        return;
    }

    let src = if parts[0].starts_with('/') { String::from(parts[0]) } else { alloc::format!("/{}", parts[0]) };
    let dst = if parts[1].starts_with('/') { String::from(parts[1]) } else { alloc::format!("/{}", parts[1]) };

    let mut fs = state::MEMFS.lock();
    let content = match fs.files.get(&src) {
        Some(c) => c.clone(),
        None => { println!("cp: '{}': No such file", parts[0]); return; }
    };

    fs.files.insert(dst, content);
    println!("Copied {} -> {}", parts[0], parts[1]);
    state::log_cmd(&alloc::format!("cp {} {}", parts[0], parts[1]));
}
