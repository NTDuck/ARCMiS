# Translation Plan: C FFT Library → Rust

## Overview
This document outlines the strategy and task breakdown for translating a C FFT library to Rust. The plan is structured as a sequence of seed tasks in `tasks.json`, each with detailed instructions for the translator.

## Strategy
1. **Domain-Specific Translation**: Focus on numerical computing patterns (FFT algorithms, complex number handling, memory management) rather than general Rust idioms.
2. **Seed Tasks First**: Identify 3-6 critical tasks that establish the foundation; subsequent tasks build on these.
3. **Pattern Extraction**: Identify common C→Rust patterns used throughout the codebase (e.g., pointer arithmetic, malloc/free, loops, conditionals).
4. **Gradual Handoff**: Each task produces a working module before moving to the next, enabling incremental testing and verification.

## Key Translation Challenges
- **Memory Management**: C's `malloc`/`free` vs Rust's `Box`/`Vec`/`unsafe` blocks
- **Complex Numbers**: C's `complex.h` vs Rust's `std::arch::complex` or custom structs
- **Loops & Pointers**: C-style iteration vs Rust's iterator patterns
- **Algorithm Preservation**: FFT algorithm correctness must be maintained exactly

## Output Structure
- `rust/fft/` — Main library crate
- `rust/fft/src/` — Source files
- `rust/fft/Cargo.toml` — Rust project manifest
- `rust/fft/tests/` — Test modules
- `rust/fft/tests/cargo_check/` — Verification tests
