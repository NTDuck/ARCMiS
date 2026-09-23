# FFT Library Translation Plan

## Overview
Translate a C FFT library to Rust. This is a substantial refactoring task requiring careful consideration of performance, memory safety, and idiomatic Rust patterns.

## Strategy

### 1. Architecture Translation
- Map C data structures (arrays, pointers, structs) to Rust types (Vec, Array, slices)
- Preserve API surface area (public functions, structs, traits)
- Maintain backward compatibility where possible

### 2. Performance Considerations
- Use `num-complex` crate for complex number support
- Consider `ring` crate for FFT implementations
- Benchmark critical paths
- Avoid unnecessary allocations

### 3. Safety & Correctness
- Use `unsafe` blocks sparingly with careful bounds checking
- Prefer `Vec` over raw pointers for safety
- Add comprehensive tests including edge cases

### 4. Testing Strategy
- Unit tests for each FFT algorithm
- Integration tests for full pipeline
- Performance benchmarks comparing C vs Rust
- Property-based tests for correctness

### 5. Build System
- Use `Cargo.toml` with `[package]` section
- Consider `cargo-fuzz` for fuzz testing
- Add `[profile.release]` for optimized builds

## Key Decisions
1. Use `ring` crate for FFT algorithms (preferred over manual implementation)
2. Use `num-complex` for complex number operations
3. Keep API surface minimal but complete
4. Add documentation with examples
