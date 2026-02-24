use crate::println;

/// ps — list active tasks from the real scheduler.
pub fn run(_args: &str) {
    let tasks = crate::scheduler::list_tasks();
    println!("  PID  STATE      NAME");
    println!("  ---  ---------  ----");
    for (pid, name, state) in &tasks {
        println!("  {:>3}  {:9}  {}", pid, state, name);
    }
}
