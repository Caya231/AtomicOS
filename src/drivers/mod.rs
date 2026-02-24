pub mod keyboard;
pub mod mouse;
pub mod tty;
pub mod ata;

pub fn init() {
    keyboard::init();
    mouse::init();
    tty::init();
    ata::init();
    crate::log_info!("Drivers subsystem initialized.");
}
