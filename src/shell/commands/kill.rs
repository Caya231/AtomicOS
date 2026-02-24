use crate::println;

/// kill <pid> — terminate a task via the scheduler.
pub fn run(args: &str) {
    let pid_str = args.trim();
    if pid_str.is_empty() {
        println!("kill: usage: kill <pid>");
        return;
    }

    let pid: u64 = match pid_str.parse() {
        Ok(v) => v,
        Err(_) => { println!("kill: invalid pid: {}", pid_str); return; }
    };

    if pid == 0 {
        println!("kill: cannot kill kernel (pid 0)");
        return;
    }

    // Remove task from scheduler ready queue
    let mut sched = crate::scheduler::SCHEDULER.lock();
    if let Some(pos) = sched.ready_queue.iter().position(|t| t.id.0 == pid) {
        let task = sched.ready_queue.remove(pos).unwrap();
        println!("Terminated task '{}' (pid {})", task.name, pid);
    } else {
        println!("kill: no such process: {}", pid);
    }
}
