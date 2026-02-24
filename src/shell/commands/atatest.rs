use crate::println;

/// atatest — automated ATA PIO read/write test.
pub fn run(_args: &str) {
    println!("=== ATA PIO Disk Test ===");

    let ata = crate::drivers::ata::PRIMARY_ATA.lock();

    if !ata.detected {
        println!("[ATA TEST] SKIP: no disk detected");
        return;
    }

    let test_lba: u32 = 10;

    // Build a test pattern
    let mut write_buf = [0u8; 512];
    for i in 0..512 {
        write_buf[i] = (i & 0xFF) as u8;
    }

    // Write
    match ata.write_sector(test_lba, &write_buf) {
        Ok(()) => println!("[ATA TEST] write LBA {} OK", test_lba),
        Err(e) => { println!("[ATA TEST] write FAIL: {}", e); return; },
    }

    // Read back
    let mut read_buf = [0u8; 512];
    match ata.read_sector(test_lba, &mut read_buf) {
        Ok(()) => println!("[ATA TEST] read LBA {} OK", test_lba),
        Err(e) => { println!("[ATA TEST] read FAIL: {}", e); return; },
    }

    // Compare
    let mut match_ok = true;
    for i in 0..512 {
        if read_buf[i] != write_buf[i] {
            println!("[ATA TEST] MISMATCH at byte {} (wrote {:#04x}, read {:#04x})", i, write_buf[i], read_buf[i]);
            match_ok = false;
            break;
        }
    }

    if match_ok {
        println!("[ATA TEST] data match OK — 512 bytes verified");
    }

    // Test sector 0 read
    let mut sec0 = [0u8; 512];
    match ata.read_sector(0, &mut sec0) {
        Ok(()) => println!("[ATA TEST] read LBA 0 OK (first 8 bytes: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x})",
            sec0[0], sec0[1], sec0[2], sec0[3], sec0[4], sec0[5], sec0[6], sec0[7]),
        Err(e) => println!("[ATA TEST] read LBA 0 FAIL: {}", e),
    }

    println!("=== ATA Test Complete ===");
}
