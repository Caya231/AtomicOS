use crate::println;

/// shellscript "<cmd1; cmd2; cmd3>" — execute multiple commands sequentially.
pub fn run(args: &str) {
    let script = args.trim();
    if script.is_empty() {
        println!("shellscript: usage: shellscript <cmd1; cmd2; cmd3>");
        return;
    }

    // Split by semicolons and execute each command
    let commands: alloc::vec::Vec<&str> = script.split(';').collect();
    for (i, cmd) in commands.iter().enumerate() {
        let trimmed = cmd.trim();
        if trimmed.is_empty() { continue; }
        println!("[{}] > {}", i + 1, trimmed);
        crate::shell::exec_command(trimmed);
    }
}
