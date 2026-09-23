# Validation Report

## Build
`cargo build` — success (clean, no warnings).

## Tests
`cargo test` — 1 passed, 2 failed (after test generation).

### Passing
- `test_inplace_dc_bin` (tests/test.rs) — **ok**
  - Runs `fft_inplace` on an 8-point alternating +1/-1 signal and asserts the DC bin equals 8.

### Failing (generated tests)
- `test_fft_out_of_place_matches_inplace` (tests/test.rs) — **FAILED**
  - Panics at `src/lib.rs:67:5`: `attempt to shift right with overflow`.
- `test_fft_full_spectrum_matches_dft` (tests/test.rs) — **FAILED**
  - Panics at `src/lib.rs:67:5`: `attempt to shift right with overflow`.

## Root-cause analysis (translated-source defect)
Both failures originate in `next_reversed_n` (src/lib.rs:60-68), which is used by
`rader` (out-of-place bit-reversal). The C original was written for a 32-bit
`size_t` (`INTBITS(size_t)` = 32). The Rust port substitutes `usize::BITS`, which is
**64** on this 64-bit platform.

Consequence: for `logsize = 3`, `shift = usize::BITS - logsize = 61`. Inside
`next_reversed_n`, `reversed_n <<= shift` pushes the value into the top bits, and the
subsequent `reversed_n >>= shift + count_leading_ones` overflows (shift ≥ 64), causing
the runtime panic. Even where it does not panic, the bit-reversal indices are wrong
(simulated sequence for logsize=3: 0,4,6,7,7,7,7,7 instead of 0,4,2,6,1,5,3,7).

Note: `test_inplace_dc_bin` still passes because the alternating +1/-1 signal has a
spectrum that is invariant under the corrupted permutation (only the DC bin is
non-zero), so the bug is masked for that particular input. The generated tests use
non-symmetric inputs and therefore expose the defect.

This is a genuine translation bug (C `size_t` width assumption not preserved), not a
test-authoring issue. Per the validation rules the translated source was not modified.

## Coverage vs. plan function list
| Plan function | Test coverage |
|---|---|
| `Complex::add` | exercised indirectly via `fft_inplace` (butterfly) |
| `Complex::sub` | exercised indirectly via `fft_inplace` (butterfly) |
| `Complex::mul` | exercised indirectly via `fft_inplace` (butterfly) |
| `Complex::self_mul` | exercised indirectly via `fft_inplace` (butterfly) |
| `Complex::unitroot_recip` | exercised indirectly via `fft_inplace` (butterfly) |
| `next_reversed_n` | exercised indirectly via `rader`/`rader_inplace` |
| `rader` | exercised indirectly via public `fft` (out-of-place) |
| `rader_inplace` | exercised indirectly via `fft_inplace` |
| `do_butterfly` | exercised indirectly via `fft_raw` |
| `fft_raw` | exercised indirectly via `fft`/`fft_inplace` |
| `fft` | directly tested by `test_fft_out_of_place_matches_inplace` and `test_fft_full_spectrum_matches_dft` |
| `fft_inplace` | directly tested by `test_inplace_dc_bin` |

## Uncovered functions
None. All functions in the plan's list now have test coverage (directly or via the
public `fft` / `fft_inplace` entry points). The previously uncovered `fft` and `rader`
are now covered by the generated tests.

## Failures
- `test_fft_out_of_place_matches_inplace` — shift-right overflow in `next_reversed_n` (src/lib.rs:67).
- `test_fft_full_spectrum_matches_dft` — shift-right overflow in `next_reversed_n` (src/lib.rs:67).
