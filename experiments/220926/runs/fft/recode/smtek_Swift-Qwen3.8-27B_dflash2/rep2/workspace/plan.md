# Implementation Plan: C FFT Library → Rust

## Context
Translate the C radix-2 iterative FFT library (`src/fft.h`, `src/fft.c`,
`tests/test.c`) into a std-only Rust crate. Skeleton files already exist in
this workspace (`Cargo.toml`, `src/lib.rs`, `src/complex.rs`, `src/fft.rs`,
`tests/test.rs`) with all symbols stubbed via `unimplemented!()`. Fill them in
bottom-up. Test command: `cargo test`.

## Name mapping (C → Rust)
| C symbol | Rust symbol | Notes |
|---|---|---|
| `struct fft_complex` / `fft_complex_t` | `FftComplex` | fields `real`/`imag` kept |
| `FFT_COMPLEX_ADD/SUB/MUL` macros | `FftComplex::add/sub/mul` | free-style inherent fns |
| `FFT_COMPLEX_SELFMUL` | `FftComplex::self_mul` | in-place `&mut self` |
| `FFT_COMPLEX_COPY` | `Copy`/`Clone` derive | no function |
| `FFT_COMPLEX_SWAP` | `std::mem::swap` / slice `swap` | no function |
| `FFT_COMPLEX_SETONE` | `FftComplex::set_one` | |
| `FFT_COMPLEX_UNITROOT_RECIP` | `FftComplex::unit_root_recip` | `f32::cos/sin`, `f32::consts::PI` |
| `fft_clz` | `usize::leading_zeros()` | `INTBITS(size_t)` → `usize::BITS` |
| `next_reversed_n` | `next_reversed_n` (private fn) | same algorithm |
| `rader` / `rader_inplace` | `rader` / `rader_inplace` (private fns) | slices |
| `DO_BUTTERFLY` macro | `butterfly(x, step)` (private fn) | |
| `fft_raw` | `fft_raw` (private fn) | |
| `fft` / `fft_inplace` | `pub fn fft` / `pub fn fft_inplace` | slices, `logsize: usize` |
| `likely`/`unlikely` | dropped (or `core::hint::cold`) | no correctness impact |
| `tests/test.c` main | `tests/test.rs` `#[test] fn test_8point` | |

## Part A — source files (bottom-up dependency order)
1. `src/complex.rs` — implement `FftComplex` methods (`add`, `sub`, `mul`,
   `self_mul`, `set_one`, `unit_root_recip`). Depends on: std only.
   Preserve f32 and exact operation order (no `mul_add`).
2. `src/fft.rs` — implement `next_reversed_n` (guard shifts < `usize::BITS`;
   `shift = usize::BITS - logsize`), `rader`, `rader_inplace`, `butterfly`,
   `fft_raw` (steps 2, 4, then 8, 16, …, size via `step *= 2`), `fft`,
   `fft_inplace`. Depends on: `src/complex.rs`.
3. `src/lib.rs` — already complete (module decls + re-exports); verify only.

## Part B — test files (bottom-up dependency order)
1. `tests/test.rs` — already written; verify it passes with `cargo test`
   (asserts `data[4].real == 8.0` exactly in f32). Depends on: lib.

## Key risks to watch while filling stubs
- Shifts by >= `usize::BITS` panic in debug: verify `next_reversed_n` shifts.
- Keep f32 and operation order for the exact-equality assertion.
- `leading_zeros(0)` == BITS; C never passes 0, keep invariant.
