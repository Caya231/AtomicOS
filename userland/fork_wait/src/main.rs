#![no_std]
#![no_main]

#[macro_use]
extern crate atomiclibc;

#[no_mangle]
pub extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    printf!("Forking now...\n");
    let pid = atomiclibc::unistd::fork();
    
    if pid == 0 {
        // Child
        let child_pid = atomiclibc::unistd::getpid();
        printf!("I am the child! My PID is %d\n", child_pid);
        let mut sum = 0;
        for i in 0..100000 {
            sum += i;
        }
        printf!("Child done calculating. Exiting with 42.\n");
        atomiclibc::unistd::exit(42);
        loop {}
    } else if pid > 0 {
        // Parent
        printf!("I am the parent! Waiting for child %d to finish...\n", pid);
        let status = atomiclibc::unistd::wait(pid);
        printf!("Child finished with status: %d\n", status);
        0
    } else {
        printf!("Fork failed!\n");
        -1
    }
}
