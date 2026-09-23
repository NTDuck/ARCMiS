
# AMP

  Rust implementation of the uber simple [AMP](https://github.com/visionmedia/node-amp) protocol.

## Example

```rust
let args = ["some", "stuff", "here"];

// encode
let buf = amp::amp_encode(&args);

// decode header
let mut msg = amp::Amp::default();
amp::amp_decode(&mut msg, &buf);
assert_eq!(1, msg.version);
assert_eq!(3, msg.argc);

// decode args
for i in 0..msg.argc {
    let arg = amp::amp_decode_arg(&mut msg);
    println!("{} : {}", i, String::from_utf8(arg).unwrap());
}
```

## Implementations

 - rust: this library
 - c: [clibs/amp](https://github.com/clibs/amp) (~10m ops/s)
 - [node](https://github.com/visionmedia/node-amp) ~(1.5m ops/s)

# License

  MIT
