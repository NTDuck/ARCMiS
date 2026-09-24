# murmurhash

MurmurHash3 general hash bashed lookup function implementation.

## Install

```sh
cargo install --path .
```

## Usage

```sh
$ echo -n kinkajou | murmur
3067714808
$ echo -n panda | murmur --seed=10
1406483717
```

## API

```rust
use murmurhash::murmurhash;

let seed = 0;
let key = "kinkajou";
let hash = murmurhash(key.as_bytes(), seed);
```

## Example

```sh
$ cargo run --example example
murmurhash(kinkajou) = 0xb6d99cf8
```

## Tests

```sh
$ cargo test
```

## License

MIT
