use std::env;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use totp::from_base32;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("usage: totp [seed in base32]");
        process::exit(64); /* EX_USAGE */
    }

    let seed = &args[1];
    let mut key = [0u8; 64];

    if from_base32(seed, &mut key, key.len()) == 0 {
        eprintln!("invalid seed");
        process::exit(64); /* EX_USAGE */
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();

    println!("{:06}", totp::totp(&key, now));
}
