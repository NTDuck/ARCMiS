
# AMP

  Rust implementation of the uber simple [AMP](https://github.com/visionmedia/node-amp) protocol.

## Example

```rust
use amp::{encode, Amp};

let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];

// encode
let buf = encode(&args);

// decode header
let mut msg = Amp::decode(&buf);
assert_eq!(1, msg.version);
assert_eq!(3, msg.argc);

// decode args
for i in 0..msg.argc {
  let arg = msg.decode_arg().unwrap();
  println!("{} : {}", i, String::from_utf8_lossy(&arg));
}
```

## Implementations

 - rust: this library
 - c: [clibs/amp](https://github.com/jashkenas/amp) (~10m ops/s)
 - [node](https://github.com/visionmedia/node-amp) ~(1.5m ops/s)

# License

  MIT
