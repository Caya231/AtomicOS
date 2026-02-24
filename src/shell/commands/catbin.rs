use crate::println;

/// catbin <addr> [len] — hex dump a region of kernel memory.
pub fn run(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.trim().split_whitespace().collect();
    if parts.is_empty() || parts[0].is_empty() {
        println!("catbin: usage: catbin <hex_addr> [length]");
        return;
    }

    let addr = match u64::from_str_radix(parts[0].trim_start_matches("0x"), 16) {
        Ok(a) => a,
        Err(_) => { println!("catbin: invalid address: {}", parts[0]); return; }
    };

    let len: usize = if parts.len() > 1 {
        parts[1].parse().unwrap_or(64)
    } else {
        64
    };
    let len = len.min(256); // cap at 256 bytes

    println!("Dumping {} bytes at 0x{:016x}:", len, addr);
    for row in (0..len).step_by(16) {
        let mut hex = alloc::string::String::new();
        let mut ascii = alloc::string::String::new();
        for col in 0..16 {
            if row + col < len {
                let byte = unsafe { *((addr as usize + row + col) as *const u8) };
                hex.push_str(&alloc::format!("{:02x} ", byte));
                ascii.push(if byte >= 0x20 && byte <= 0x7e { byte as char } else { '.' });
            }
        }
        println!("  {:08x}  {:<48} |{}|", addr as usize + row, hex, ascii);
    }
}
