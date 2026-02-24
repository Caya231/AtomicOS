use crate::println;
use super::super::state;

/// ls — list entries in the in-memory filesystem.
pub fn run(args: &str) {
    let dir = if args.trim().is_empty() { "/" } else { args.trim() };
    let fs = state::MEMFS.lock();
    let entries = fs.list_dir(dir);

    if entries.is_empty() {
        println!("(empty)");
    } else {
        for e in entries {
            let name = e.rsplit('/').next().unwrap_or(e);
            if fs.is_dir(e) {
                println!("  {}/", name);
            } else {
                println!("  {}", name);
            }
        }
    }
}
