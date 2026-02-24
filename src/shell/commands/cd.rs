use crate::println;
use super::super::state;
use alloc::string::String;

/// cd [path] — change current working directory.
/// Supports absolute paths, relative paths, `.` and `..`.
pub fn run(args: &str) {
    let target = args.trim();

    // cd with no args goes to /
    if target.is_empty() || target == "~" {
        *state::CWD.lock() = String::from("/");
        state::log_cmd("cd /");
        return;
    }

    let resolved = state::resolve_path(target);

    let fs = state::MEMFS.lock();
    if fs.is_dir(&resolved) {
        drop(fs); // release MemFs lock before locking CWD
        *state::CWD.lock() = resolved;
        state::log_cmd(&alloc::format!("cd {}", target));
    } else if fs.exists(&resolved) {
        println!("cd: {}: Not a directory", target);
    } else {
        println!("cd: {}: No such directory", target);
    }
}
