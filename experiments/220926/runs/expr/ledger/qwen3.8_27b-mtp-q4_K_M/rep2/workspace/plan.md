# Plan: Translating a C Expression Parser/Evaluator to Rust

## Core Difficulties

- **Operator precedence and associativity**: The C grammar has many precedence levels (unary, multiplicative, additive, shift, relational, equality, bitwise, logical, ternary, assignment) that must be reproduced exactly.
- **Recursive descent vs. table-driven parsing**: Choosing a parsing strategy that is both readable and correct for nested expressions.
- **Type system and implicit conversions**: C's implicit integer promotions, pointer arithmetic, and signed/unsigned wrap-around semantics need explicit modeling in Rust.
- **Error handling**: C code often relies on undefined behavior or silent truncation; Rust requires explicit `Result`/`Option` handling for overflow, division by zero, and type mismatches.
- **Memory and ownership**: The C evaluator may use raw pointers and manual memory management; Rust's ownership model changes how intermediate AST nodes and evaluation state are represented.

## Recommended Approach

1. **Define the AST first** as Rust enums (`Expr`, `BinaryOp`, `UnaryOp`, `Literal`) with `#[derive(Debug, Clone)]`.
2. **Implement a recursive descent parser** with one function per precedence level, using a simple token stream (lexer) as input.
3. **Implement the evaluator** as a pure function `fn eval(expr: &Expr, env: &Env) -> Result<Value, EvalError>` where `Value` is an enum covering integers, floats, booleans, and pointers.
4. **Port the test suite** from C to Rust using `#[cfg(test)]` modules, ensuring every edge case (precedence, overflow, short-circuit evaluation) is covered.
5. **Use `thiserror`** for structured error types and `clap` if a CLI entry point is needed.

## Module Structure

```
src/
  lib.rs          – public API, re-exports
  lexer.rs        – tokenization (Token enum, Lexer struct)
  parser.rs       – recursive descent parser (Parser struct, Expr AST)
  ast.rs          – AST node definitions (Expr, BinaryOp, UnaryOp, Literal)
  eval.rs         – evaluator (Value enum, Env, eval function)
  error.rs        – error types (LexError, ParseError, EvalError)
  main.rs         – CLI entry point (optional)
tests/
  integration.rs  – end-to-end expression tests
```

## Seed Tasks

1. **Lexer**: Implement tokenization of C expressions into a `Token` enum (numbers, identifiers, operators, parentheses).
2. **Parser**: Implement the recursive descent parser that builds an `Expr` AST with correct precedence and associativity.
3. **Evaluator**: Implement the tree-walking evaluator with an environment map, handling arithmetic, comparisons, logical ops, and short-circuit evaluation.
4. **Tests and CLI**: Port the C test suite to Rust integration tests and wire up a `main.rs` CLI that reads an expression and prints the result.
