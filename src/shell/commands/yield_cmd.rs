use crate::println;

/// yield — cooperatively yield to the next ready task.
pub fn run(_args: &str) {
    let sched = crate::scheduler::SCHEDULER.lock();
    let count = sched.ready_queue.len();
    drop(sched);

    if count == 0 {
        println!("yield: no other tasks to switch to");
    } else {
        println!("yield: switching to next task...");
        crate::scheduler::yield_now();
    }
}
