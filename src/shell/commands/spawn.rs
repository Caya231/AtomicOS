use crate::println;

/// spawn <name> — spawn a demo background task.
pub fn run(args: &str) {
    let name = args.trim();
    if name.is_empty() {
        println!("spawn: usage: spawn <task_name>");
        println!("  Available demo tasks: counter, ticker, hello");
        return;
    }

    match name {
        "counter" => {
            let id = crate::syscalls::sys_spawn(task_counter, "counter");
            println!("Spawned 'counter' as task {}", id);
        },
        "ticker" => {
            let id = crate::syscalls::sys_spawn(task_ticker, "ticker");
            println!("Spawned 'ticker' as task {}", id);
        },
        "hello" => {
            let id = crate::syscalls::sys_spawn(task_hello, "hello");
            println!("Spawned 'hello' as task {}", id);
        },
        _ => println!("spawn: unknown task '{}'", name),
    }
}

/// Demo task: counts to 5 then exits.
fn task_counter() {
    for i in 1..=5 {
        crate::println!("[counter] tick {}", i);
        // Small busy-wait to make output visible
        for _ in 0..500_000 { core::hint::spin_loop(); }
        crate::scheduler::yield_now();
    }
    crate::println!("[counter] done!");
    crate::scheduler::exit_current();
}

/// Demo task: prints 3 ticks then exits.
fn task_ticker() {
    for _ in 0..3 {
        crate::println!("[ticker] *");
        for _ in 0..300_000 { core::hint::spin_loop(); }
        crate::scheduler::yield_now();
    }
    crate::println!("[ticker] finished.");
    crate::scheduler::exit_current();
}

/// Demo task: prints hello and exits immediately.
fn task_hello() {
    crate::println!("[hello] Hello from a background task!");
    crate::scheduler::exit_current();
}
