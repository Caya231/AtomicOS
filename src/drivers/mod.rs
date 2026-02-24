pub mod keyboard;

pub fn init() {
    keyboard::init();
    crate::log_info!("Drivers subsystem initialized.");
}
