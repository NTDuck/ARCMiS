# Translation Design: C FFT library → Rust

## 1. Source project analysis

### Structure
```
c/
├── Makefile          # builds libfft.a from src/fft.c; `make test` compiles tests/test.c and runs it
├── src/
│   ├── fft.h         # public API + complex-number macros
│   └── fft.c         # implementation
└── tests/
    └── test.c        # single test: 8-point in-place FFT of [1,-1,1,-1,1,-1,1,-1]
```

### Public API (src/fft.h)
- `struct fft_complex { float real; float imag; }` — 32-bit float complex, `typedef`'d as `fft_complex_t`.
- `void fft(const fft_complex_t *restrict x, fft_complex_t *restrict X, size_t logsize);`
  Out-of-place forward FFT of `2^logsize` points.
- `void fft_inplace(fft_complex_t *x, size_t logsize);`
  In-place forward FFT of `2^logsize` points.
- A family of macros implementing complex arithmetic on `fft_complex_t`:
  `FFT_COMPLEX_ADD/SUB/MUL/SELFMUL/COPY/SWAP/SETONE/UNITROOT_RECIP`.
  `FFT_COMPLEX_UNITROOT_RECIP(result, N)` sets `result = e^{-i·2π/N}` using `cos`/`sin`
  (note: computed in `float` precision because the struct fields are `float`).

### Implementation notes (src/fft.c)
- `fft_clz(n)`: count-leading-zeros via `_Generic` over `__builtin_clz{,l,ll}` on
  GCC/Clang, with a portable fallback loop.
- `next_reversed_n(reversed_n, shift)`: bit-manipulation that yields the next
  bit-reversed index; uses `fft_clz(~reversed_n)` and shifts by
  `INTBITS(size_t) - logsize`.
- `rader` / `rader_inplace`: bit-reversal permutation (the "Rader" step).
- `DO_BUTTERFLY(begin, end, step)` macro: one radix-2 DIT butterfly pass with
  twiddle factor `unit = e^{-i·2π/step}`, specialized for `i==0` and `i==1`
  (no twiddle multiplication), generic loop for the rest.
- `fft_raw`: runs butterfly passes for step = 2, 4, 8, …, 2^logsize.
- `fft` = `rader` + `fft_raw`; `fft_inplace` = `rader_inplace` + `fft_raw`.
- `likely`/`unlikely` are `__builtin_expect` hints (no semantic effect).
- No heap allocation, no global state, no I/O. Pure function of its inputs.

### Build / test
- `make` → `libfft.a` (static library, `-O3 -g -fprofile-arcs -ftest-coverage -fPIC`, links `-lm`).
- `make test` → compiles `tests/test.c` against the library, runs `test_1`.
- The test asserts exact equality of the real parts of the 8-point FFT output
  (`{0,0,0,0,8,0,0,0}`).

## 2. Third-party dependency analysis

The C project has **no third-party dependencies** — only libc (`assert.h`,
`limits.h`) and libm (`math.h`: `cos`, `sin`, `M_PI`).

Rust counterpart mapping:
| C dependency | Rust counterpart | Notes |
|---|---|---|
| libc / libm (`cos`, `sin`, `M_PI`) | Rust std (`f32::cos`, `f32::sin`, `std::f32::consts::PI`) | Built-in, no crate needed. |
| `__builtin_clz` | `usize::leading_zeros()` | Built-in, no crate needed. |
| `__builtin_expect` (likely/unlikely) | `std::intrinsics::likely/unlikely` (unstable) — **omit** | Pure branch hints; safe to drop. |

**Result: zero external crates.** The Rust project uses only the standard
library. This keeps `cargo test` hermetic and fast.

## 3. Target project design (Rust)

### Layout
```
rust/
├── Cargo.toml          # package name "fft", edition 2021, no dependencies
├── src/
│   └── lib.rs          # the whole library (single module, mirroring fft.h + fft.c)
└── tests/
    └── test.rs         # integration test mirroring tests/test.c
```

A single `src/lib.rs` is appropriate: the C code is one translation unit of
~150 lines. Splitting into more modules would add indirection without benefit.

### Type mapping
- `fft_complex_t` → `pub struct Complex { pub real: f32, pub imag: f32 }`
  (kept as a plain struct with public fields to mirror the C struct; `Copy` +
  `Clone` derived).
- `size_t` → `usize`.
- `logsize` parameter stays `usize`.

### API mapping
- `pub fn fft(x: &[Complex], out: &mut [Complex], logsize: usize)`
  — out-of-place. The C `restrict` contract (x and X disjoint) is expressed by
  taking `&[Complex]` and `&mut [Complex]` (borrow checker enforces disjointness).
  Document that `out.len() >= 1 << logsize`.
- `pub fn fft_inplace(x: &mut [Complex], logsize: usize)`
  — in-place.
- Complex arithmetic macros → inherent methods on `Complex`:
  `add`, `sub`, `mul`, `copy` (just `Copy`), `swap` (slice `swap`),
  `set_one`, `unit_root_recip(n: usize) -> Complex`.
  `FFT_COMPLEX_UNITROOT_RECIP` → `Complex::unit_root_recip(n)` computing
  `f32::cos(2π/n)` / `-f32::sin(2π/n)` in **f32** to match C float precision.

### Implementation mapping
- `fft_clz(n)` → `n.leading_zeros() as usize` (identical semantics for
  `usize`; the C fallback loop is unnecessary).
- `next_reversed_n(reversed_n, shift)` → same bit-manipulation with `usize`,
  `usize::BITS` in place of `INTBITS(size_t)`.
- `rader` / `rader_inplace` → private functions, same loops.
- `DO_BUTTERFLY` macro → private function
  `fn butterfly(x: &mut [Complex], step: usize)` operating on the whole slice
  (the C macro's `begin`/`end` pointers become the slice bounds).
- `fft_raw` → private function calling `butterfly` for steps 2, 4, 8, …
- `likely`/`unlikely` → plain `if` (drop the hints).

### Test mapping
`tests/test.rs`:
```rust
use fft::Complex;
use fft::fft_inplace;

#[test]
fn test_8_point() {
    let mut data = [
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        /* ... 8 entries total, alternating 1/-1 ... */
    ];
    fft_inplace(&mut data, 3);
    let expected_real = [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0];
    for i in 0..8 {
        assert_eq!(data[i].real, expected_real[i]);
    }
}
```
This mirrors the C test exactly (exact `==` on real parts, which is safe here
because the values are exact powers-of-two sums representable in f32).

Optionally add a second test for `fft` (out-of-place) to cover that path, since
the C test only exercises `fft_inplace`.

### Cargo.toml
```toml
[package]
name = "fft"
version = "0.1.0"
edition = "2021"

[lib]
name = "fft"
path = "src/lib.rs"
```
No `[dependencies]`.

## 4. Risks and mitigations

1. **Bit-reversal index arithmetic on `usize`**: `next_reversed_n` relies on
   `INTBITS(size_t)` (64 on the build host). Rust `usize` is also 64-bit on
   the target platform, so semantics match. Risk: on a 32-bit target the C
   code and Rust code would both use 32 bits, so they stay consistent.
   Mitigation: keep the expression structurally identical; add a unit test
   that checks `next_reversed_n` produces the expected sequence for small
   `logsize` (e.g. logsize=3 → 0,4,2,6,1,5,3,7).
2. **f32 vs f64 precision**: C uses `float` (f32) throughout, including the
   `cos`/`sin` twiddle computation. The Rust port must use `f32` (not `f64`)
   to keep bit-identical results, especially for the exact-equality test.
   Mitigation: use `f32` everywhere; the test values are exact in f32.
3. **`restrict` semantics**: C `restrict` is a compiler hint; the Rust borrow
   checker enforces it. No behavioral risk.
4. **Dropping `likely`/`unlikely`**: no semantic change; only a possible minor
   performance difference. Acceptable.
5. **`M_PI` portability**: C `M_PI` is not standard C; Rust `std::f32::consts::PI`
   is the canonical constant. No risk.
6. **Out-of-place `fft` not covered by the C test**: the Rust port should add
   a test for `fft` to ensure the `rader` (non-inplace) path is exercised.

## 5. Verification plan
- `cargo test` must pass (the required test command).
- The 8-point in-place test must produce exactly `{0,0,0,0,8,0,0,0}` on real parts.
- An additional out-of-place test and a bit-reversal sequence test are recommended
  for confidence.
