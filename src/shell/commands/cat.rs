use crate::println;
use alloc::string::String;
use super::super::state;

/// cat <file> — display contents of a file in the in-memory filesystem.
pub fn run(args: &str) {
    let filename = args.trim();
    if filename.is_empty() {
        println!("cat: missing filename");
        return;
    }

    let full = if filename.starts_with('/') { String::from(filename) } else { alloc::format!("/{}", filename) };

    let fs = state::MEMFS.lock();
    match fs.files.get(&full) {
        Some(Some(content)) => println!("{}", content),
        Some(None) => println!("cat: {}: Is a directory", filename),
        None => println!("cat: {}: No such file", filename),
    }
}
