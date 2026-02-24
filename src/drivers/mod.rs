pub mod keyboard;
pub mod mouse;
pub mod tty;

pub fn init() {
    keyboard::init();
    mouse::init();
    tty::init();
    crate::log_info!("Drivers subsystem initialized.");
}
