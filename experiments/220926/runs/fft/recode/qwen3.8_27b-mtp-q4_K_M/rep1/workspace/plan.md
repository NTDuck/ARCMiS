# FFT C → Rust Translation Plan

## Overview
Port the C `libfft.a` (iterative radix-2 FFT over `struct fft_complex {float real, imag}`)
to a single Rust crate. No third-party dependencies. `cargo test` is the test command.

## Name mapping (C → Rust)
| C symbol | Rust symbol | Notes |
|---|---|---|
| `struct fft_complex` / `fft_complex_t` | `pub struct Complex` | fields `real`, `imag` kept |
| `FFT_COMPLEX_ADD` | `Complex::add` | macro → associated fn |
| `FFT_COMPLEX_SUB` | `Complex::sub` | macro → associated fn |
| `FFT_COMPLEX_MUL` | `Complex::mul` | macro → associated fn |
| `FFT_COMPLEX_SELFMUL` | `Complex::self_mul` | macro → `&mut self` method |
| `FFT_COMPLEX_COPY` | Rust value copy | no stub needed |
| `FFT_COMPLEX_SWAP` | `std::mem::swap` | no stub needed |
| `FFT_COMPLEX_SETONE` | inline `Complex{1.0,0.0}` | no stub needed |
| `FFT_COMPLEX_UNITROOT_RECIP` | `Complex::unitroot_recip` | macro → associated fn |
| `fft_clz` | `usize::leading_zeros` | use std builtin, no stub |
| `next_reversed_n` | `next_reversed_n` | name kept |
| `rader` | `rader` | name kept |
| `rader_inplace` | `rader_inplace` | name kept |
| `DO_BUTTERFLY` | `do_butterfly` | macro → fn |
| `fft_raw` | `fft_raw` | name kept |
| `fft` | `fft` | name kept (param `X` → `x`) |
| `fft_inplace` | `fft_inplace` | name kept |
| `main` (test) | `#[test] fn test_inplace_dc_bin` | C main → cargo test |

## Part A — source files (bottom-up dependency order)
1. `src/lib.rs` — the only source file. Implement in this order within the file:
   - `Complex` struct + `add`, `sub`, `mul`, `self_mul`, `unitroot_recip`
   - `next_reversed_n` (uses `usize::leading_zeros`)
   - `rader` (uses `next_reversed_n`)
   - `rader_inplace` (uses `next_reversed_n`)
   - `do_butterfly` (uses `Complex` ops)
   - `fft_raw` (uses `do_butterfly`)
   - `fft` (uses `rader`, `fft_raw`)
   - `fft_inplace` (uses `rader_inplace`, `fft_raw`)

## Part B — test files (bottom-up dependency order)
1. `tests/test.rs` — `#[test] fn test_inplace_dc_bin` (uses `fft_inplace`, `Complex`)

## Notes
- `INTBITS(size_t)` → `usize::BITS` (32 on this platform).
- `likely`/`unlikely` → plain branches (no effect on correctness).
- `restrict` → Rust borrow checker enforces disjointness.
- `M_PI` → `std::f32::consts::PI`.
