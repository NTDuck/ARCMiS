murmurhash
==========

> MurmurHash3 general hash bashed lookup function implementation

## about

[MurmurHash](http://en.wikipedia.org/wiki/MurmurHash) is a non-cryptographic hash function suitable
for general hash-based lookup. This implementation implements version 3
of MurmurHash.

## install

```sh
$ cargo build
$ cargo install --path .
```

## example

```rust
use murmurhash::murmurhash;

fn main() {
    let seed = 0u32;
    let key = "kinkajou";
    let hash = murmurhash(key.as_bytes(), seed); // 0xb6d99cf8
}
```

A command line executable is also available:

```sh
$ echo -n kinkajou | murmur
3067714808
```

```sh
$ echo -n panda | murmur --seed=10
1406483717
```

## api

```rust
pub fn murmurhash(key: &[u8], seed: u32) -> u32;
```

Returns a murmur hash of `key` based on `seed` using the MurmurHash3 algorithm.

## license

MIT
