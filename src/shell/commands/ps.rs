use crate::println;
use super::super::state;

/// ps — list active processes (simulated).
pub fn run(_args: &str) {
    state::log_cmd("ps");
    let table = state::PROCS.lock();
    println!("  PID  STATE      NAME");
    println!("  ---  ---------  ----");
    for p in &table.procs {
        println!("  {:>3}  {:9}  {}", p.pid, p.state, p.name);
    }
}
