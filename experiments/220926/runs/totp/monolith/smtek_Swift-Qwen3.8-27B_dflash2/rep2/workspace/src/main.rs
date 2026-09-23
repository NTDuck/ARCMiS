use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let mut args = std::env::args();
    let _prog = args.next();
    let seed = match args.next() {
        Some(s) => s,
        None => {
            eprintln!("usage: totp [seed in base32]");
            std::process::exit(64); /* EX_USAGE */
        }
    };

    let mut key = [0u8; 64];

    if totp::from_base32(&seed, &mut key, key.len()) == 0 {
        eprintln!("invalid seed");
        std::process::exit(64); /* EX_USAGE */
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();

    println!("{:06}", totp::totp(&key, now));
}
