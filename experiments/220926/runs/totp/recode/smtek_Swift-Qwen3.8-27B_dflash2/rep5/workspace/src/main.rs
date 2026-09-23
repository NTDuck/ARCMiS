//! CLI: `totp <seed in base32>` prints the 6-digit code for the current
//! time. Exits with 64 (EX_USAGE) on bad usage.

use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

use totp::{from_base32, totp};

fn main() {
    let seed = match std::env::args().nth(1) {
        Some(seed) => seed,
        None => {
            eprintln!("usage: totp [seed in base32]");
            exit(64);
        }
    };

    let mut key = [0u8; 64];
    if from_base32(&seed, &mut key).is_err() {
        eprintln!("invalid seed");
        exit(64);
    }

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let code = totp(&key, time).unwrap();
    println!("{:06}", code);
}
