# Brainstorm: Translating a C Expression Parser to Rust

## Summary of the codebase

The source is a small C expression parser: a hand-written recursive-descent
parser over a token stream (identifiers, integer/float literals, operators,
parentheses), producing an AST of binary/unary operators and literals. It
includes a lexer (string scanning with `strtol`/`strtod`), an AST node struct
with a `kind` enum and `union`-style payload, a precedence-climbing or
recursive-descent parser, and a small test suite that feeds expression strings
and checks the resulting tree shape and evaluated values.

## Core difficulties

1. **C's `union`-style AST payloads** — C uses a tagged struct with a union;
   Rust needs an `enum` with per-variant payloads (or `Box`ed children) to be
   memory-safe and exhaustive.
2. **Manual memory management** — C allocates nodes with `malloc` and frees
   them; Rust ownership/`Box` replaces this, but ownership rules must be
   respected (e.g., `&mut` vs owned children, no double-free).
3. **Error handling** — C returns error codes or sets a global `errno`/flag;
   Rust should use `Result<Expr, ParseError>` with typed errors.
4. **String handling** — C uses `char*` and `strlen`; Rust uses `&str` and
   `str::len` (bytes vs chars matters for non-ASCII, though C expressions
   are ASCII).
5. **Precedence and associativity** — must be preserved exactly; the C code
   may rely on subtle precedence rules (e.g., unary minus vs `**` if present).
6. **Test fidelity** — tests must assert the same tree shapes/values; Rust's
   `assert_eq!` needs `Debug`/`PartialEq` impls on the AST.

## Candidate approaches

### Approach A: Direct port (line-by-line)
Mirror the C structure: same functions, same control flow, `Box<Expr>` for
nodes, `Result` for errors. Minimal design change, easiest to verify
equivalence against the C tests.

### Approach B: Rewrite with a parser combinator / token-based design
Use a cleaner Rust idiom: tokenize into a `Vec<Token>`, parse with a
`Parser` struct holding a position index, build the AST with an `enum`.
More idiomatic, easier to extend, but a larger behavioral delta to verify.

## Recommended approach

**Approach A (direct port)** for the first pass: it keeps the translation
mechanical and makes it straightforward to prove the Rust version matches the
C version's behavior on the existing test suite. Once tests pass, the code can
be refactored toward Approach B idioms if desired.

## Concrete sub-tasks

1. **Port the lexer**: convert the C token scanner to a Rust function
   `fn next_token(input: &str, pos: &mut usize) -> Result<Token, LexError>`,
   handling identifiers, integer/float literals, operators, and whitespace.
2. **Define the AST**: create `enum Expr { Lit(f64), Ident(String),
   Unary(Operator, Box<Expr>), Binary(Operator, Box<Expr>, Box<Expr>) }`
   with `Debug`, `Clone`, and `PartialEq` derives.
3. **Port the parser**: translate the recursive-descent / precedence-climbing
   functions to Rust, threading `&mut Parser` state and returning
   `Result<Expr, ParseError>`.
4. **Port the evaluator** (if present): implement `fn eval(expr: &Expr) ->
   f64` with the same operator semantics and error cases.
5. **Port the test suite**: convert each C test case to a Rust `#[test]`
   function asserting the same AST shape and/or evaluated value.
6. **Build and verify**: run `cargo build` and `cargo test`, fix any
   compilation errors or test failures, and confirm all tests pass.
