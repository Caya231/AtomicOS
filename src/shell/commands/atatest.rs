/// atatest — automated ATA PIO read/write test.
pub fn run(_args: &str) {
    crate::println!("=== ATA PIO Disk Test ===");
    crate::log_info!("=== ATA PIO Disk Test ===");

    let ata = crate::drivers::ata::PRIMARY_ATA.lock();

    if !ata.detected {
        crate::println!("[ATA TEST] SKIP: no disk detected");
        crate::log_info!("[ATA TEST] SKIP: no disk detected");
        return;
    }

    let test_lba: u32 = 10;

    // Build test pattern: 0x00..0xFF repeated
    let mut write_buf = [0u8; 512];
    for i in 0..512 {
        write_buf[i] = (i & 0xFF) as u8;
    }

    // Write sector
    if let Err(e) = ata.write_sector(test_lba, &write_buf) {
        crate::println!("[ATA TEST] write FAIL: {}", e);
        crate::log_info!("[ATA TEST] write FAIL: {}", e);
        return;
    }
    crate::println!("[ATA TEST] write LBA {} OK", test_lba);
    crate::log_info!("[ATA TEST] write LBA {} OK", test_lba);

    // Read sector back
    let mut read_buf = [0u8; 512];
    if let Err(e) = ata.read_sector(test_lba, &mut read_buf) {
        crate::println!("[ATA TEST] read FAIL: {}", e);
        crate::log_info!("[ATA TEST] read FAIL: {}", e);
        return;
    }
    crate::println!("[ATA TEST] read LBA {} OK", test_lba);
    crate::log_info!("[ATA TEST] read LBA {} OK", test_lba);

    // Compare byte-by-byte
    let mut mismatch = false;
    for i in 0..512 {
        if read_buf[i] != write_buf[i] {
            crate::println!("[ATA TEST] MISMATCH byte {} (wrote {:#04x}, read {:#04x})", i, write_buf[i], read_buf[i]);
            crate::log_info!("[ATA TEST] MISMATCH byte {} (wrote {:#04x}, read {:#04x})", i, write_buf[i], read_buf[i]);
            mismatch = true;
            break;
        }
    }
    if !mismatch {
        crate::println!("[ATA TEST] data match OK — 512 bytes verified");
        crate::log_info!("[ATA TEST] data match OK — 512 bytes verified");
    }

    // Test reading sector 0
    let mut sec0 = [0u8; 512];
    if let Err(e) = ata.read_sector(0, &mut sec0) {
        crate::println!("[ATA TEST] read LBA 0 FAIL: {}", e);
        crate::log_info!("[ATA TEST] read LBA 0 FAIL: {}", e);
    } else {
        crate::println!("[ATA TEST] read LBA 0 OK");
        crate::log_info!("[ATA TEST] read LBA 0 OK");
    }

    crate::println!("=== ATA Test Complete ===");
    crate::log_info!("=== ATA Test Complete ===");
}
