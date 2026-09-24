//! `example` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use murmurhash::murmurhash;

fn main() {
    let seed = 0;
    let key = "kinkajou";
    let hash = murmurhash(key.as_bytes(), seed);
    println!("murmurhash({}) = 0x{:x}", key, hash);
}
