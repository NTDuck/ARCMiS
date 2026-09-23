# AMP

Rust implementation of the uber simple [AMP](https://github.com/visionmedia/node-amp) protocol.

## Example

```rust
use amp::{amp_encode, amp_decode, amp_decode_arg, Amp};

let args = ["some", "stuff", "here"];

// encode
let buf = amp_encode(&args);

// decode header
let mut msg = Amp::default();
amp_decode(&mut msg, &buf);
assert_eq!(1, msg.version);
assert_eq!(3, msg.argc);

// decode args
for i in 0..msg.argc {
    let arg = amp_decode_arg(&mut msg).unwrap();
    println!("{} : {}", i, String::from_utf8_lossy(&arg));
}
```

## Implementations

- rust: this library
- c: [clibs/amp](https://github.com/clibs/amp) (~10m ops/s)
- [node](https://github.com/visionmedia/node-amp) ~(1.5m ops/s)

# License

MIT
