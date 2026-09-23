# Plan: C Expression Evaluator → Rust

## Overview
Translate a C expression evaluator with lexer, parser, evaluator, stack machine, function table, and history tracking into idiomatic Rust.

## Architecture Decisions
1. **Vec-based stack** (capacity 256) instead of fixed-size array
2. **enum-based AST** for expression/term/yexpression nodes
3. **Ownership model** throughout — no GC, no reference counting
4. **History tracking** with negative indices for two-down lookback
5. **Vector indexing** with signed indices
6. **Constant folding** during evaluation

## File Structure
```
src/
  lexer.rs          - Tokenizer with all token types
  parser.rs          - Parser with state machine (EXPR, TERM, YYEQ, YEQEQ, VAR)
  evaluator.rs     - Stack machine evaluator
  ast.rs             - AST node definitions
  memory.rs          - Memory helpers (malloc/free equivalent)
  main.rs            - Entry point
tests/
  test.rs           - Test suite (2 tests)
```

## Key Design Notes
- Use `Vec<T>` with `.reserve(256)` for stack
- Use `Box<[T]>` for stack storage to avoid heap allocation per push/pop
- History uses `Vec<Vec<f64>>` with negative indices
- Function table: `Vec<Box<dyn Fn(...)>` or trait object array
- Constant folding: evaluate constants during AST construction
