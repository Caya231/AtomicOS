pub mod pio;

use pio::AtaDevice;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PRIMARY_ATA: Mutex<AtaDevice> = Mutex::new(AtaDevice::new(0x1F0, 0x3F6, true));
}

pub fn init() {
    let mut dev = PRIMARY_ATA.lock();
    if dev.identify().is_ok() {
        crate::log_info!("ATA PIO: Primary master disk detected.");
    } else {
        crate::log_warn!("ATA PIO: No disk detected.");
    }
}
