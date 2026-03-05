use crate::unistd;

// The entry point expected by our ELF linker script.
// It sets up basic arguments (argc=0, argv=null) and calls `main`.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    extern "C" {
        fn main(argc: isize, argv: *const *const u8) -> isize;
    }

    // Call the user's main function
    let ret = unsafe { main(0, core::ptr::null()) };

    // Exit the process cleanly
    unistd::exit(ret as i32);
    loop {}
}
