pub unsafe fn strlen(s: *const u8) -> usize {
    let mut count = 0;
    while *s.add(count) != 0 {
        count += 1;
    }
    count
}

pub unsafe fn memcpy(dst: *mut u8, src: *const u8, len: usize) {
    for i in 0..len {
        *dst.add(i) = *src.add(i);
    }
}

pub unsafe fn memset(dst: *mut u8, val: u8, len: usize) {
    for i in 0..len {
        *dst.add(i) = val;
    }
}

pub unsafe fn strcmp(s1: *const u8, s2: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 && c2 == 0 { return 0; }
        if c1 < c2 { return -1; }
        if c1 > c2 { return 1; }
        i += 1;
    }
}
