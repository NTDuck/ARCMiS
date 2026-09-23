# FFT Library Translation Plan: C → Rust

## Strategy

1. **Analyze the C source** thoroughly to understand the FFT algorithm, data structures, and API surface.
2. **Design idiomatic Rust equivalents** — prefer `Vec`, `Box`, `Option`, `Result` over C pointers.
3. **Use `nix` or `bindgen`** for FFI if the C library has a C API; otherwise pure Rust.
4. **Leverage Rust's standard library** — no need to reinvent FFT algorithms. Use existing crates like `rustfft` or implement from scratch based on the C implementation.
5. **Write comprehensive tests** in Rust's `#[cfg(test)]` module.
6. **Maintain the same public API** as the C library for FFI compatibility.

## Key Decisions

- **FFI approach**: If the C code uses POSIX-style FFI (`extern "C"`), we'll create a Rust FFI layer. Otherwise, pure Rust is fine.
- **Algorithm**: Use `rustfft` crate for the FFT implementation, or port the C algorithm manually.
- **Testing**: Write unit tests for correctness, integration tests for FFI bindings.

## Tasks

1. Analyze C source code
2. Design Rust API
3. Implement core FFT algorithm
4. Write tests
5. Create FFI bindings
6. Integration testing
