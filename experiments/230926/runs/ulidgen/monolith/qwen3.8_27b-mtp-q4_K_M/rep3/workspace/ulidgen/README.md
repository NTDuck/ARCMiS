# ulidgen

`ulidgen` generates and prints one or multiple ULID (Universally Unique
Lexicographically Sortable Identifier).

## Synopsis

```
ulidgen [-n N] [-t]
```

## Options

- `-n N` — Print N consecutive ULID.
- `-t` — Read lines from standard input, and prefix each line with a ULID.

## Exit status

The `ulidgen` utility exits 0 on success, and >0 if an error occurs.

## Examples

Generating three ULID:

```
% ulidgen -n 3
01J970QT8Y3F9H32BHX76RRS7T
01J970QT8Y3F9H32BHX76RRS7V
01J970QT8Y3F9H32BHX76RRS7W
```

## See also

- <https://github.com/ulid/spec#specification>
- `uuidgen(1)`

## Authors

Leah Neukirchen <leah@vuxu.org>

## License

`ulidgen` is in the public domain.

To the extent possible under law, the creator of this work has waived all
copyright and related or neighboring rights to this work.

<http://creativecommons.org/publicdomain/zero/1.0/>
