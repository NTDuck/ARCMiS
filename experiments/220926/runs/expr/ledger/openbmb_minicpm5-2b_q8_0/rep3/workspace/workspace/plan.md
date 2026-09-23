# Plan: C Expression Evaluator → Rust

## Structure
```
rust/
├── src/
│   └── lib.rs          # Main library with all components
├── tests/
│   └── lib_test.rs    # Test suite (2 tests)
├── Cargo.toml
└── plan.md              # This file
```

## Key Design Decisions
1. **No GC** → Vec with capacity 256 for stack, Box for AST nodes
2. **State machine** → enum for parser states (~20 variants)
3. **Immutable data** → Vec/String slices over mutable stacks
4. **Ownership** → Box/Vec with ownership transfer
5. **History tracking** → Negative indices for two-down lookback
6. **Vector indexing** → Signed indices in Rust

## Files to Create
1. `Cargo.toml` - Dependencies, edition 2021
2. `src/lib.rs` - All components
3. `tests/lib_test.rs` - 2 test cases
