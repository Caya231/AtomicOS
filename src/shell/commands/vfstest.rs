use alloc::vec;

/// vfstest — automated VFS integration test suite.
/// Output goes to both VGA (println) and serial (log_info).
pub fn run(_args: &str) {
    macro_rules! test_log {
        ($($arg:tt)*) => {
            crate::println!($($arg)*);
            crate::log_info!($($arg)*);
        }
    }

    test_log!("=== VFS Integration Test Suite ===");

    let mut pass = 0u32;
    let mut fail = 0u32;

    // Test 1: mkdir
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.mkdir("/vfstest_dir") {
            Ok(_) => { test_log!("[PASS] mkdir /vfstest_dir"); pass += 1; },
            Err(e) => { test_log!("[FAIL] mkdir /vfstest_dir: {}", e); fail += 1; },
        }
    }

    // Test 2: create file
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.create("/vfstest_dir/hello.txt") {
            Ok(_) => { test_log!("[PASS] create /vfstest_dir/hello.txt"); pass += 1; },
            Err(e) => { test_log!("[FAIL] create: {}", e); fail += 1; },
        }
    }

    // Test 3: write to file
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.write_file("/vfstest_dir/hello.txt", b"Hello from VFS!") {
            Ok(n) => { test_log!("[PASS] write {} bytes", n); pass += 1; },
            Err(e) => { test_log!("[FAIL] write: {}", e); fail += 1; },
        }
    }

    // Test 4: read back file
    {
        let vfs = crate::fs::VFS.lock();
        let mut buf = vec![0u8; 64];
        match vfs.read_file("/vfstest_dir/hello.txt", 0, &mut buf) {
            Ok(n) => {
                let text = core::str::from_utf8(&buf[..n]).unwrap_or("???");
                if text == "Hello from VFS!" {
                    test_log!("[PASS] read: \"{}\"", text); pass += 1;
                } else {
                    test_log!("[FAIL] read mismatch: \"{}\"", text); fail += 1;
                }
            },
            Err(e) => { test_log!("[FAIL] read: {}", e); fail += 1; },
        }
    }

    // Test 5: readdir
    {
        let vfs = crate::fs::VFS.lock();
        match vfs.readdir("/vfstest_dir") {
            Ok(entries) => {
                if entries.len() == 1 && entries[0].name == "hello.txt" {
                    test_log!("[PASS] readdir: found hello.txt"); pass += 1;
                } else {
                    test_log!("[FAIL] readdir: unexpected entries ({})", entries.len()); fail += 1;
                }
            },
            Err(e) => { test_log!("[FAIL] readdir: {}", e); fail += 1; },
        }
    }

    // Test 6: unlink file
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.unlink("/vfstest_dir/hello.txt") {
            Ok(()) => { test_log!("[PASS] unlink /vfstest_dir/hello.txt"); pass += 1; },
            Err(e) => { test_log!("[FAIL] unlink: {}", e); fail += 1; },
        }
    }

    // Test 7: unlink empty directory
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.unlink("/vfstest_dir") {
            Ok(()) => { test_log!("[PASS] unlink /vfstest_dir (empty dir)"); pass += 1; },
            Err(e) => { test_log!("[FAIL] unlink dir: {}", e); fail += 1; },
        }
    }

    // Test 8: error handling — lookup nonexistent
    {
        let vfs = crate::fs::VFS.lock();
        match vfs.lookup("/does_not_exist.txt") {
            Err(crate::fs::error::FsError::NotFound) => {
                test_log!("[PASS] lookup nonexistent -> NotFound"); pass += 1;
            },
            _ => { test_log!("[FAIL] expected NotFound error"); fail += 1; },
        }
    }

    // Test 9: /tmp mount point
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.mkdir("/tmp/testdir") {
            Ok(_) => { test_log!("[PASS] mkdir /tmp/testdir (via tmpfs)"); pass += 1; },
            Err(e) => { test_log!("[FAIL] mkdir /tmp/testdir: {}", e); fail += 1; },
        }
        let _ = vfs.unlink("/tmp/testdir");
    }

    // Test 10: stress — 10 dirs
    {
        let mut vfs = crate::fs::VFS.lock();
        let mut ok = true;
        for i in 0..10 {
            let name = alloc::format!("/stress_{}", i);
            if vfs.mkdir(&name).is_err() { ok = false; break; }
        }
        for i in 0..10 {
            let name = alloc::format!("/stress_{}", i);
            let _ = vfs.unlink(&name);
        }
        if ok {
            test_log!("[PASS] stress: 10 mkdir + rm OK"); pass += 1;
        } else {
            test_log!("[FAIL] stress test failed"); fail += 1;
        }
    }

    test_log!("=== Results: {}/{} passed ===", pass, pass + fail);
    if fail == 0 {
        test_log!("RAMFS Phase 4.2 VALIDATED!");
    } else {
        test_log!("{} test(s) FAILED.", fail);
    }
}
