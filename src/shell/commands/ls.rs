use crate::println;
use super::super::state;

/// ls [dir] — list entries in the in-memory filesystem.
/// If no dir is given, lists the current working directory.
pub fn run(args: &str) {
    let target = args.trim();
    let dir = if target.is_empty() {
        state::CWD.lock().clone()
    } else {
        state::resolve_path(target)
    };

    let fs = state::MEMFS.lock();
    let entries = fs.list_dir(&dir);

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
