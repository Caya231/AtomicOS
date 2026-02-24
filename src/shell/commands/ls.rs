use crate::println;
use crate::fs::inode::FileType;

/// ls [dir] — list entries using the VFS.
pub fn run(args: &str) {
    let target = args.trim();
    let dir = if target.is_empty() {
        crate::shell::state::CWD.lock().clone()
    } else {
        crate::shell::state::resolve_path(target)
    };

    let vfs = crate::fs::VFS.lock();
    match vfs.readdir(&dir) {
        Ok(entries) => {
            if entries.is_empty() {
                println!("(empty)");
            } else {
                for e in entries {
                    if e.inode.file_type == FileType::Directory {
                        println!("  {}/", e.name);
                    } else {
                        println!("  {}  ({}B)", e.name, e.inode.size);
                    }
                }
            }
        },
        Err(e) => println!("ls: {}: {}", dir, e),
    }
}
