#![no_std]
#![feature(abi_x86_interrupt)]

pub mod vga;
pub mod serial;
pub mod interrupts;
pub mod memory;
pub mod scheduler;
pub mod syscalls;
pub mod drivers;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    vga::init();
    serial::init();
    interrupts::init();
    log_info!("AtomicOS Kernel started.");
    
    memory::init();
    log_info!("AtomicOS Memory intialized.");

    scheduler::init();
    syscalls::init();
    drivers::init();
    println!("AtomicOS is successfully running!");

    x86_64::instructions::interrupts::enable();

    x86_64::instructions::interrupts::enable();

    // Main event loop
    loop {
        use crate::drivers::keyboard::scancodes::KeyCode;
        let key = crate::drivers::keyboard::read_char();
        
        match key {
            KeyCode::Char(c) => print!("{}", c),
            KeyCode::Enter => println!(),
            KeyCode::Backspace => crate::vga::WRITER.lock().backspace(),
            KeyCode::Unknown => {}
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    log_error!("{}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
