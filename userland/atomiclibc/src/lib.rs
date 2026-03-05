#![no_std]

pub mod syscall;
pub mod unistd;
pub mod stdio;
pub mod string;
pub mod malloc;
pub mod crt0;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let _ = unistd::write(2, b"Userland Panic!\n");
    unistd::exit(1);
    loop {}
}
