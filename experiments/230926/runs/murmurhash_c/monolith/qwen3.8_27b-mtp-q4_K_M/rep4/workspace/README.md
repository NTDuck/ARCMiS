# murmurhash

`murmurhash` is a Rust implementation of the [MurmurHash3](https://github.com/aappleby/smhash) general hash based lookup function.

## Features

- Fast
- Non-cryptographic
- Portable (little endian word extraction, works on big endian hosts too)

## Installation

```sh
cargo install --path .
```

## Usage

```sh
$ echo -n kinkajou | murmur
3067714808
```

### Options

```sh
$ murmur -h
usage: murmur [-hV] [options]

options:

  --seed=[seed]  hash seed (optional)
```

### Seed

```sh
$ echo -n panda | murmur --seed=10
1406483717
```

## Library

```rust
use murmurhash::murmurhash;

fn main() {
    let seed = 0;
    let key = "kinkajou";
    let hash = murmurhash(key.as_bytes(), seed);
    println!("murmurhash({}) = 0x{:x}", key, hash);
}
```

## Examples

```sh
$ cargo run --example example
murmurhash(kinkajou) = 0xb6d99cf8
```

## Tests

```sh
$ cargo test
```

## Man Page

```sh
$ man man/murmur.1
```

## License

MIT
