use crate::println;
use alloc::vec;

/// vfstest — automated VFS integration test suite.
pub fn run(_args: &str) {
    println!("=== VFS Integration Test Suite ===");
    println!();

    let mut pass = 0;
    let mut fail = 0;

    // Test 1: mkdir
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.mkdir("/vfstest_dir") {
            Ok(_) => { println!("[PASS] mkdir /vfstest_dir"); pass += 1; },
            Err(e) => { println!("[FAIL] mkdir /vfstest_dir: {}", e); fail += 1; },
        }
    }

    // Test 2: create file
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.create("/vfstest_dir/hello.txt") {
            Ok(_) => { println!("[PASS] create /vfstest_dir/hello.txt"); pass += 1; },
            Err(e) => { println!("[FAIL] create: {}", e); fail += 1; },
        }
    }

    // Test 3: write to file
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.write_file("/vfstest_dir/hello.txt", b"Hello from VFS!") {
            Ok(n) => { println!("[PASS] write {} bytes", n); pass += 1; },
            Err(e) => { println!("[FAIL] write: {}", e); fail += 1; },
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
                    println!("[PASS] read: \"{}\"", text); pass += 1;
                } else {
                    println!("[FAIL] read mismatch: \"{}\"", text); fail += 1;
                }
            },
            Err(e) => { println!("[FAIL] read: {}", e); fail += 1; },
        }
    }

    // Test 5: readdir
    {
        let vfs = crate::fs::VFS.lock();
        match vfs.readdir("/vfstest_dir") {
            Ok(entries) => {
                if entries.len() == 1 && entries[0].name == "hello.txt" {
                    println!("[PASS] readdir: found hello.txt"); pass += 1;
                } else {
                    println!("[FAIL] readdir: unexpected entries ({})", entries.len()); fail += 1;
                }
            },
            Err(e) => { println!("[FAIL] readdir: {}", e); fail += 1; },
        }
    }

    // Test 6: unlink file
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.unlink("/vfstest_dir/hello.txt") {
            Ok(()) => { println!("[PASS] unlink /vfstest_dir/hello.txt"); pass += 1; },
            Err(e) => { println!("[FAIL] unlink: {}", e); fail += 1; },
        }
    }

    // Test 7: unlink directory
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.unlink("/vfstest_dir") {
            Ok(()) => { println!("[PASS] unlink /vfstest_dir (empty dir)"); pass += 1; },
            Err(e) => { println!("[FAIL] unlink dir: {}", e); fail += 1; },
        }
    }

    // Test 8: error handling — cat nonexistent
    {
        let vfs = crate::fs::VFS.lock();
        match vfs.lookup("/does_not_exist.txt") {
            Err(crate::fs::error::FsError::NotFound) => {
                println!("[PASS] lookup nonexistent -> NotFound"); pass += 1;
            },
            _ => { println!("[FAIL] expected NotFound error"); fail += 1; },
        }
    }

    // Test 9: /tmp mount point resolution
    {
        let mut vfs = crate::fs::VFS.lock();
        match vfs.mkdir("/tmp/testdir") {
            Ok(_) => { println!("[PASS] mkdir /tmp/testdir (via tmpfs)"); pass += 1; },
            Err(e) => { println!("[FAIL] mkdir /tmp/testdir: {}", e); fail += 1; },
        }
        let _ = vfs.unlink("/tmp/testdir"); // cleanup
    }

    // Test 10: stress — create 10 dirs without crash
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
            println!("[PASS] stress: 10 mkdir + rm without crash"); pass += 1;
        } else {
            println!("[FAIL] stress test failed"); fail += 1;
        }
    }

    println!();
    println!("=== Results: {}/{} passed ===", pass, pass + fail);
    if fail == 0 {
        println!("VFS Phase 4.1 VALIDATED!");
    } else {
        println!("{} test(s) FAILED.", fail);
    }
}
