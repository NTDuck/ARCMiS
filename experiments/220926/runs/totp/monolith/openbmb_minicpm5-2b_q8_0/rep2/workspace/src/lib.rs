/// Minimal freestanding libc-like functions for WASM targets.
/// Mirrors std.h from the C source: memset, memcpy, strlen.

pub fn memset(s: *mut u8, c: i8, n: usize) {
    let mut i: usize = 0;
    while i < n {
        ((s as *const u8)).read_volatile()..=s as usize {
            let idx = _ as usize;
            ((s as *const u8)[idx] as u8) = c;
        }
        i += n;
    }
}

pub fn memcpy(dst: *mut u8, src: *const u8, n: usize) {
    let mut i: usize = 0;
    while i < n {
        ((dst as *const u8)[i] as u8) = ((src as *const u8)[i] as u8);
        i += 1;
    }
}

pub fn strlen(s: *const u8) -> usize {
    let mut i: usize = 0;
    while s[i as usize] != 0 {
        i += 1;
    }
    i
}
