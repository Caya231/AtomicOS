use crate::println;
use super::super::state;

/// kill <pid> — terminate a simulated process.
pub fn run(args: &str) {
    let pid_str = args.trim();
    if pid_str.is_empty() {
        println!("kill: usage: kill <pid>");
        return;
    }

    let pid: u32 = match pid_str.parse() {
        Ok(v) => v,
        Err(_) => { println!("kill: invalid pid: {}", pid_str); return; }
    };

    if pid == 0 {
        println!("kill: cannot kill kernel (pid 0)");
        return;
    }

    let mut table = state::PROCS.lock();
    if let Some(pos) = table.procs.iter().position(|p| p.pid == pid) {
        let name = table.procs[pos].name.clone();
        table.procs.remove(pos);
        println!("Terminated process {} (pid {})", name, pid);
        state::log_cmd(&alloc::format!("kill {}", pid));
    } else {
        println!("kill: no such process: {}", pid);
    }
}
