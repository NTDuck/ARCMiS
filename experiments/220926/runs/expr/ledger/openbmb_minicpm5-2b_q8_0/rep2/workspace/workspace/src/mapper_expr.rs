// Brainstorming document: C expression evaluator → Rust translation
// Date: 2025-07-13

// ============================================================
// RECOMMENDATION SUMMARY
// ============================================================
//
// Primary Goal: Faithful recreation with idiomatic Rust ergonomics.
// We want a correct, working Rust equivalent that preserves all
// behavior (including signal processing semantics) while
// using idiomatic Rust patterns.
//
// API Surface: Preserve the exact same public API surface as the
// C library. This is important because:
//   - It may be used by existing C code
//   - The test suite expects specific function signatures
//   - The signal processing semantics depend on stable ABI
//
// Signal Processing Model: Treat as a first-class concept.
// The `history_index` and `vector_index` fields should be
// explicit struct fields, not buried in separate arrays.
// This makes the streaming semantics clear and composable.
//
// Performance Profile: Correctness > speed. The evaluator
// processes expressions lazily (streaming). No need for
// aggressive optimization. Use `Vec` for stacks.
//
// Rust Version: Use Rust 1.80+ (stable). Use edition 2021.
// Prefer `Result` over `Option` for error handling.
//
// ============================================================

// ============================================================
// APPROACH RECOMMENDATION: Hybrid (A + D) with B improvements
// ============================================================
//
// Why not pure direct translation (A)?
//   - C's imperative style with global variables and malloc
//     is not idiomatic Rust. We lose readability and safety.
//   - Stack management with manual malloc/free is error-prone.
//
// Why not pure functional (C)?
//   - The algorithm is imperative (state machine, two stacks).
//     Converting to pure functions would require significant refactoring.
//   - Mutable state is needed for the parser stacks.
//
// Recommended hybrid:
//   - Keep the imperative state machine structure (it's the core algorithm).
//   - Use `Vec<T>` for stack management instead of malloc.
//   - Use `Result<T, E>` instead of `die_unless` macros.
//   - Separate concerns into modules (lexer, parser, evaluator).
//   - Use idiomatic Rust types and patterns.
//
// ============================================================
// KEY DESIGN DECISIONS FOR THE RUST IMPLEMENTATION
// ============================================================

// 1. State Machine: Enum-based, but with clear naming
//   - Use a `State` enum with meaningful names
//   - Each variant represents a parsing phase
//   - Use pattern matching for state transitions
//
// 2. Signal Processing Semantics:
//   - Create a `SignalContext` struct that holds:
//       history_index: Vec<usize>  // indices into output history
//       vector_index: Vec<usize> // indices into vector data
//   - Expressions reference past outputs via history indices
//   - Vector indexing is supported for array-like data
//
// 3. Floating Point: Use `f64` throughout (Rust's default float).
//   - C's `float` maps to `f64` in Rust.
//   - All math functions use `std::f64::*` equivalents.
//
// 4. Memory Management:
//   - Stacks: `Vec<Item>` (heap-allocated, zero-cost)
//   - History: `Vec<usize>` (indices into output buffer)
//   - No manual memory management needed.
//
// 5. Error Handling:
//   - Use `Result<T, E>` with custom error types
//   - Return `Err(ErrorKind::ParseError)` for syntax errors
//   - Return `Err(ErrorKind::OutOfMemory)` for allocation failures
//   - Don't use `panic!` or `unwrap()` in production code.
//
// ============================================================
// FILE STRUCTURE
// ============================================================
//
// src/
//   lib.rs          # Public API (same as C)
//   lexer.rs        # Lexer: tokenize input string
//   parser.rs       # Parser: shunting-yard + evaluation
//   evaluator.rs   # Evaluation engine with history/vector support
//   math.rs         # Math function implementations
//   error.rs        # Error type definitions
//   constants.rs     # Math constants (pi, e, etc.)
//
// test/
//   test1.rs        # Complex expression test
//   test2.rs      # Simple linear expression test
//
// ============================================================
// DIFFICULTIES AND THEIR SOLUTIONS
// ============================================================
//
// Difficulty 1: State Machine Complexity
//   Solution: Use a clean enum-based state machine. Each state
//   is a variant of the `State` enum. Use match expressions
//   for transitions. Keep the stack operations explicit but
//   organized.
//
// Difficulty 2: History/Vector Semantics
//   Solution: Create a `SignalContext` struct that wraps both
//   history arrays and provides methods for:
//     - push_history(index, value): Record an output
//     - get_history(index): Get historical value
//     - get_vector_index(index): Get vector index
//   This makes the streaming semantics clear and composable.
//
// Difficulty 3: Constant Folding
//   Solution: Keep it as-is. It's a performance optimization that
//   should work in Rust. Use `eval_expr` recursively with
//   memoization if needed.
//
// Difficulty 4: Signal Processing Model
//   Solution: Make it first-class. The `SignalContext` struct
//   should be passed around naturally. Expressions can reference
//   past outputs via history indices. This is the core innovation.
//
// Difficulty 5: Math Library Equivalents
//   Solution: Use `std::f64::powf`, `std::f64::sin`, etc.
//   Note: C's `float` functions are `float::powf`, etc.
//   In Rust, all math is `f64`. We'll use `f64` throughout.
//
// ============================================================
// TESTING STRATEGY
// ============================================================
//
// The C tests use `die_unless` macros. In Rust:
//   - Use `assert!` for simple assertions
//   - Use `assert_eq!` for value comparisons
//   - Use `assert!` for non-termination checks
//   - Use `panic!` only for truly unrecoverable errors
//
// Test cases should cover:
//   - Basic arithmetic
//   - Function calls (sin, cos, sqrt, etc.)
//   - Parentheses and precedence
//   - History indexing
//   - Vector indexing
//   - Error cases (invalid syntax, out of bounds)
//
// ============================================================
// BUILD STRATEGY
// ============================================================
//
// Use cargo test to run tests.
// Use cargo clippy to check for style issues.
// Use cargo doc to generate documentation.
//
// ============================================================
// CONCLUSION
// ============================================================
//
// The translation is feasible and well-understood. The main
// challenges are:
//   1. Preserving the exact same API surface (especially the
//      signal processing semantics)
//   2. Handling the history/vector indexing correctly
//   3. Making the code idiomatic Rust while staying faithful
//
// The hybrid approach (A + D) with B improvements is recommended.
// It preserves the algorithm while using idiomatic Rust patterns.
//
// ============================================================
