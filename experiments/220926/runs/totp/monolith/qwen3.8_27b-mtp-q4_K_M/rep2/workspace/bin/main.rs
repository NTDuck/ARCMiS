use std::env;
use std::process;

use totp::{from_base32, totp};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("usage: totp [seed in base32]");
        process::exit(64); /* EX_USAGE */
    }

    let seed = &args[1];
    let mut key = [0u8; 64];

    let cap = key.len();
    if from_base32(seed, &mut key, cap) == 0 {
        eprintln!("invalid seed");
        process::exit(64); /* EX_USAGE */
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    println!("{:06}", totp(&key, now));
}
