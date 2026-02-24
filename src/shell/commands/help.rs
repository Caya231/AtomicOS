use crate::println;

pub fn run(_args: &str) {
    println!("AtomicOS Shell - Available commands:");
    println!("");
    println!("  echo <text>       Print text to terminal");
    println!("  ls [dir]          List files in directory");
    println!("  cat <file>        Show file contents");
    println!("  clear             Clear the screen");
    println!("  help              Show this help message");
    println!("  date              Show current date/time (RTC)");
    println!("  whoami            Show current user");
    println!("  pwd               Show working directory");
    println!("  uptime            Show time since boot");
    println!("  version           Show kernel version");
    println!("  neofetch          Show system info with logo");
    println!("");
    println!("  ps                List active processes");
    println!("  kill <pid>        Terminate a process");
    println!("  mkdir <name>      Create a directory");
    println!("  rm <path>         Remove a file or directory");
    println!("  cp <src> <dst>    Copy a file");
    println!("  mv <src> <dst>    Move/rename a file");
    println!("  catbin <addr>     Hex dump memory at address");
    println!("  objdump           Inspect kernel ELF info");
    println!("  shellscript <..>  Run commands separated by ;");
    println!("  log [n]           Show last n kernel log entries");
}
