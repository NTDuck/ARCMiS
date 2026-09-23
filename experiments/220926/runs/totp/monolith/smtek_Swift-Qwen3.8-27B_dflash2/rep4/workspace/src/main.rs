use std::time::{SystemTime, UNIX_EPOCH};

use totp::{from_base32, totp};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("usage: totp [seed in base32]");
        return;
    }

    let seed = &args[1];
    let mut key = [0u8; 64];

    if from_base32(seed, &mut key, key.len()) == 0 {
        eprintln!("invalid seed");
        return;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("{:06}", totp(&key, now));
}
