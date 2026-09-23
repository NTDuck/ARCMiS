use totp::{from_base32, hotp, hmac_sha1, pack32, sha1, unpack32, unpack64};

fn to_hex(a: &[u8], len: usize) -> String {
    let mut s = String::new();
    for i in 0..len {
        s.push_str(&format!("{:02x}", a[i]));
    }
    s
}

#[test]
fn test_pack() {
    let mut a = [0u8; 8];

    unpack32(0x12345678, &mut a[0..4].try_into().unwrap());
    assert_eq!(a[0], 0x12);
    assert_eq!(a[1], 0x34);
    assert_eq!(a[2], 0x56);
    assert_eq!(a[3], 0x78);

    unpack64(0x123456789ABCDEF0, &mut a);
    assert_eq!(a[0], 0x12);
    assert_eq!(a[1], 0x34);
    assert_eq!(a[2], 0x56);
    assert_eq!(a[3], 0x78);
    assert_eq!(a[4], 0x9A);
    assert_eq!(a[5], 0xBC);
    assert_eq!(a[6], 0xDE);
    assert_eq!(a[7], 0xF0);

    assert_eq!(pack32(&a[0..4].try_into().unwrap()), 0x12345678);
}

#[test]
fn test_sha1() {
    let mut buf = [0u8; 512];
    let mut hash = [0u8; 20];
    let cap = buf.len();

    sha1(&mut buf, 0, cap, &mut hash);
    assert_eq!(
        to_hex(&hash, 20),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );

    let abc = b"abc";
    buf[..3].copy_from_slice(abc);
    sha1(&mut buf, 3, cap, &mut hash);
    assert_eq!(
        to_hex(&hash, 20),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );

    let fox = b"The quick brown fox jumps over the lazy dog";
    buf[..fox.len()].copy_from_slice(fox);
    sha1(&mut buf, fox.len(), cap, &mut hash);
    assert_eq!(
        to_hex(&hash, 20),
        "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
    );
}

#[test]
fn test_hmac_sha1() {
    let mut key = [0u8; 64];
    let mut text = [0u8; 64];
    let mut hash = [0u8; 20];

    /* RFC 2202 */
    for i in 0..20 {
        key[i] = 0xAA;
    }
    for i in 0..50 {
        text[i] = 0xDD;
    }

    hmac_sha1(&key, &text, 50, &mut hash);
    assert_eq!(
        to_hex(&hash, 20),
        "125d7342b9ac11cd91a39af48aa17b4f63f175d3"
    );
}

#[test]
fn test_hotp() {
    /* Appendix D */
    let mut secret = [0u8; 64];
    let data: [u8; 20] = [
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38, 0x39, 0x30,
    ];
    secret[..20].copy_from_slice(&data);

    assert_eq!(hotp(&secret, 0), 755224);
    assert_eq!(hotp(&secret, 1), 287082);
    assert_eq!(hotp(&secret, 2), 359152);
}

#[test]
fn test_from_base32() {
    let mut buf = [0u8; 10];
    let cap = buf.len();

    assert_eq!(from_base32("MZxw6===", &mut buf, cap), 3);
    assert_eq!(from_base32("MZxw6YQ=", &mut buf, cap), 4);
    assert_eq!(from_base32("MZxw6YTB", &mut buf, cap), 5);
    assert_eq!(from_base32("MZxw6YTBOI======", &mut buf, cap), 6);

    assert_eq!(&buf[..6], b"foobar");
}
