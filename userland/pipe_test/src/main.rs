#![no_std]
#![no_main]

#[macro_use]
extern crate atomiclibc;

#[no_mangle]
pub extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    printf!("Starting pipe test...\n");

    let mut fds = [0u32; 2];
    if atomiclibc::unistd::pipe(&mut fds) < 0 {
        printf!("Pipe failed!\n");
        return -1;
    }

    let read_fd = fds[0] as usize;
    let write_fd = fds[1] as usize;
    printf!("Pipe created: read=%d, write=%d\n", read_fd, write_fd);

    let pid = atomiclibc::unistd::fork();

    if pid == 0 {
        // Child process
        atomiclibc::unistd::close(read_fd); // Close unused read end
        
        let msg = "Hello from the child via pipe!";
        printf!("Child: writing to pipe...\n");
        atomiclibc::unistd::write(write_fd, msg.as_bytes());
        atomiclibc::unistd::close(write_fd);
        
        printf!("Child: exiting.\n");
        atomiclibc::unistd::exit(0);
        loop {}
    } else if pid > 0 {
        // Parent process
        atomiclibc::unistd::close(write_fd); // Close unused write end
        
        let mut buf = [0u8; 64];
        printf!("Parent: waiting to read from pipe...\n");
        let bytes_read = atomiclibc::unistd::read(read_fd, &mut buf);
        
        if bytes_read > 0 {
            // Null terminate the manual slice
            if (bytes_read as usize) < buf.len() {
                buf[bytes_read as usize] = 0;
            } else {
                buf[63] = 0;
            }
            
            // We use printf with %s assuming the buffer valid utf8 string
            let s = unsafe { core::str::from_utf8_unchecked(&buf[..(bytes_read as usize)]) };
            printf!("Parent read %d bytes: '%s'\n", bytes_read, s);
        } else {
            printf!("Parent read failed or 0 bytes.\n");
        }
        
        atomiclibc::unistd::close(read_fd);
        atomiclibc::unistd::wait(pid);
        printf!("Pipe test completed.\n");
        0
    } else {
        printf!("Fork failed!\n");
        -1
    }
}
