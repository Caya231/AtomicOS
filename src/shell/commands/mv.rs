use crate::println;
use alloc::vec;

/// mv <src> <dst> — move/rename a file via VFS (copy + delete).
pub fn run(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().split_whitespace().collect();
    if parts.len() < 2 {
        println!("mv: usage: mv <source> <dest>");
        return;
    }

    let src = crate::shell::state::resolve_path(parts[0]);
    let dst = crate::shell::state::resolve_path(parts[1]);

    // Read source
    let vfs = crate::fs::VFS.lock();
    let mut buf = vec![0u8; 4096];
    let n = match vfs.read_file(&src, 0, &mut buf) {
        Ok(n) => n,
        Err(e) => { println!("mv: {}: {}", parts[0], e); return; }
    };
    drop(vfs);

    // Write to dest, remove source
    let mut vfs = crate::fs::VFS.lock();
    if !vfs.exists(&dst) {
        let _ = vfs.create(&dst);
    }
    match vfs.write_file(&dst, &buf[..n]) {
        Ok(_) => {
            let _ = vfs.unlink(&src);
            println!("Moved {} -> {}", parts[0], parts[1]);
        },
        Err(e) => println!("mv: write error: {}", e),
    }
}
