pub mod pio;

use pio::AtaDevice;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::port::Port;

lazy_static! {
    pub static ref PRIMARY_ATA: Mutex<AtaDevice> = Mutex::new(AtaDevice::new(0x1F0, 0x3F6, true));
}

pub fn init() {
    // Disable ATA interrupts (nIEN bit) on both primary and secondary
    // bus BEFORE doing any commands — prevents unhandled IRQ 14/15 double faults
    unsafe {
        Port::<u8>::new(0x3F6).write(0x02); // Primary control: nIEN = 1
        Port::<u8>::new(0x376).write(0x02); // Secondary control: nIEN = 1
    }

    let mut dev = PRIMARY_ATA.lock();
    if dev.identify().is_ok() {
        crate::log_info!("ATA PIO: Primary master disk detected.");
    } else {
        crate::log_warn!("ATA PIO: No disk detected.");
    }
}
