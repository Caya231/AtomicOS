use crate::{print, println};
use alloc::vec;

/// cat <file> — read file contents via VFS.
pub fn run(args: &str) {
    let filename = args.trim();
    if filename.is_empty() {
        println!("cat: missing filename");
        return;
    }

    let path = crate::shell::state::resolve_path(filename);
    let vfs = crate::fs::VFS.lock();

    // Read up to 4 KiB
    let mut buf = vec![0u8; 4096];
    match vfs.read_file(&path, 0, &mut buf) {
        Ok(n) => {
            if let Ok(text) = core::str::from_utf8(&buf[..n]) {
                print!("{}", text);
                if !text.ends_with('\n') { println!(); }
            } else {
                println!("cat: {}: Binary file ({} bytes)", filename, n);
            }
        },
        Err(e) => println!("cat: {}: {}", filename, e),
    }
}
