use crate::println;

pub fn run(_args: &str) {
    println!("AtomicOS Shell - Available commands:");
    println!("");
    println!("  echo <text>     Print text to terminal");
    println!("  ls              List files (simulated)");
    println!("  cat <file>      Show file contents (simulated)");
    println!("  clear           Clear the screen");
    println!("  help            Show this help message");
    println!("  date            Show current date/time (RTC)");
    println!("  whoami          Show current user");
    println!("  pwd             Show working directory");
    println!("  uptime          Show time since boot");
    println!("  version         Show kernel version");
    println!("  neofetch        Show system info with logo");
}
