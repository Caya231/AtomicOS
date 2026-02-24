use crate::println;
use super::super::state;

/// log — display the kernel command log buffer.
pub fn run(args: &str) {
    let klog = state::KLOG.lock();

    if klog.entries.is_empty() {
        println!("(no log entries)");
        return;
    }

    let count = args.trim().parse::<usize>().unwrap_or(klog.entries.len());
    let start = if klog.entries.len() > count { klog.entries.len() - count } else { 0 };

    for entry in &klog.entries[start..] {
        println!("  {}", entry);
    }
}
