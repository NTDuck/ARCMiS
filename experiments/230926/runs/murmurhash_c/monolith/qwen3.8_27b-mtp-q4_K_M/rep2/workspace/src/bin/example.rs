//! `example` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use murmurhash::murmurhash;

fn main() {
    let seed = 0;
    let key = "kinkajou"; // // 0xb6d99cf8
    let hash = murmurhash(key.as_bytes(), seed);
    println!("murmurhash({key}) = 0x{hash:x}");
}
