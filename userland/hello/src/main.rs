#![no_std]
#![no_main]

#[macro_use]
extern crate atomiclibc;

#[no_mangle]
pub extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    printf!("Hello from Userland! My PID is: %d\n", atomiclibc::unistd::getpid());
    0
}
