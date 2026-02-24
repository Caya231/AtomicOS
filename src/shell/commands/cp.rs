use crate::println;
use alloc::vec;

/// cp <src> <dst> — copy a file via VFS.
pub fn run(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        println!("cp: usage: cp <source> <dest>");
        return;
    }

    let src = crate::shell::state::resolve_path(parts[0]);
    let dst = crate::shell::state::resolve_path(parts[1]);

    let vfs = crate::fs::VFS.lock();
    let mut buf = vec![0u8; 4096];
    let n = match vfs.read_file(&src, 0, &mut buf) {
        Ok(n) => n,
        Err(e) => { println!("cp: {}: {}", parts[0], e); return; }
    };
    drop(vfs);

    let mut vfs = crate::fs::VFS.lock();
    if !vfs.exists(&dst) {
        let _ = vfs.create(&dst);
    }
    match vfs.write_file(&dst, &buf[..n]) {
        Ok(_) => println!("Copied {} -> {}", parts[0], parts[1]),
        Err(e) => println!("cp: write error: {}", e),
    }
}
