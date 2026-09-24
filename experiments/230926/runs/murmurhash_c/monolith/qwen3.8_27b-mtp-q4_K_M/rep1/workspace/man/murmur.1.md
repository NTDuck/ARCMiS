# murmur(1)

## NAME

murmur - MurmurHash3 general hash bashed lookup function implementation

## SYNOPSIS

```
murmur [-hV] [options]
```

## OPTIONS

```
--seed=[seed]  hash seed (optional)
```

## EXAMPLES

```
$ echo -n kinkajou | murmur
3067714808
$ echo -n panda | murmur --seed=10
1406483717
```

## SEE ALSO

- [murmurhash](https://github.com/jwerle/murmurhash)
