use crate::println;
use core::sync::atomic::{AtomicU64, Ordering};

/// Global tick counter incremented by the PIT timer interrupt handler.
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Called by the timer interrupt handler every tick (~18.2 Hz default PIT).
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn run(_args: &str) {
    let ticks = TICKS.load(Ordering::Relaxed);
    // Default PIT fires ~18.2 times per second
    let total_secs = ticks / 18;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    println!("up {:02}:{:02}:{:02} ({} ticks)", hours, mins, secs, ticks);
}
