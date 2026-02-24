use crate::println;
use alloc::string::String;

/// cd [path] — change current working directory.
/// Validates against the VFS that the target is a real directory.
pub fn run(args: &str) {
    let target = args.trim();

    if target.is_empty() || target == "~" {
        *crate::shell::state::CWD.lock() = String::from("/");
        return;
    }

    let resolved = crate::shell::state::resolve_path(target);

    let vfs = crate::fs::VFS.lock();
    if vfs.is_dir(&resolved) {
        drop(vfs);
        *crate::shell::state::CWD.lock() = resolved;
    } else if vfs.exists(&resolved) {
        println!("cd: {}: Not a directory", target);
    } else {
        println!("cd: {}: No such directory", target);
    }
}
