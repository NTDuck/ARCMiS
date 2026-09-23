# Translation Design: libmapper expression evaluator (C) → Rust

## 1. Source project overview

A small, self-contained expression parser/evaluator extracted from **libmapper**
(BSD-licensed, author Stephen Sinclair). It parses a string of the form
`y=<expression>` and evaluates it against a single input variable `x`, with
support for history lookback (`x{-N}`) and (partially) vector indexing (`x[N]`).

### Files
| File | Role |
|------|------|
| `mapper_expr.h` | Public C API (3 functions) |
| `mapper_expr.c` | Lexer, stack-based parser, evaluator, function table |
| `test.c` | Two smoke tests (float mode, int mode) |
| `Makefile` | Builds `test` binary with gcov coverage flags |
| `README`, `COPYING` | Provenance + BSD license |

### Public API (C)
```c
mapper_expr mapper_expr_new_from_string(const char *str,
                                        int input_is_float,
                                        int output_is_float,
                                        int vector_size);
int mapper_expr_evaluate(mapper_expr expr,
                         void* input_vector,
                         void* output_vector);
void mapper_expr_free(mapper_expr expr);
```

### Language features implemented
- **Operators:** `+ - * /` (binary), unary `-` (NEGATE state).
- **Functions (29):** pow, sin, cos, tan, abs, sqrt, log, log10, exp, floor,
  round, ceil, asin, acos, atan, atan2, sinh, cosh, tanh, logb, exp2, log2,
  hypot, cbrt, trunc, min, max, pi.
- **Variables:** `x` (input), `y` (output, LHS of `y=`).
- **History indexing:** `x{-N}` — look back N samples in the input history
  buffer. Negative index = how far back.
- **Vector indexing:** `x[N]` — partially implemented; parser rejects
  `N > 0` ("Vector indexing not yet implemented") and validates bounds.
- **Type system:** each node is either `int` or `float`. The parser inserts
  `TOK_TOFLOAT` coercion nodes when an int and float operand meet. The
  `input_is_float` / `output_is_float` flags control the top-level type.
- **Constant folding:** if a sub-expression contains no variable references,
  it is evaluated immediately at parse time and replaced by a literal.
- **History buffer:** `input_history` / `output_history` arrays of
  `mapper_signal_value_t` (union of `float` and `int32`), sized by the
  oldest history index seen during parsing.

### Parser architecture
A **state-machine / stack-based** parser (not recursive descent). A stack of
`stack_obj_t` (either a `state_t` or an `exprnode`) drives the parse. States:
`YEQUAL_Y, YEQUAL_EQ, EXPR, EXPR_RIGHT, TERM, TERM_RIGHT, VALUE, NEGATE,
VAR_RIGHT, VAR_VECTINDEX, VAR_HISTINDEX, CLOSE_VECTINDEX, CLOSE_HISTINDEX,
OPEN_PAREN, CLOSE_PAREN, COMMA, END`.

The expression tree is a **linked list** of `exprnode` (each node has a
`token_t`, `is_float` flag, `history_index`, `vector_index`, `next` pointer).
This is a flat list, not a tree — the evaluator walks it in order.

### Evaluation
`mapper_expr_evaluate` walks the linked list, maintaining an operand stack.
- `TOK_INT` / `TOK_FLOAT`: push value.
- `TOK_OP`: pop two, apply operator, push result.
- `TOK_FUNC`: pop arity-many args, call function, push result.
- `TOK_VAR`: look up `x` (or `y`) in the input/output history buffer at the
  appropriate `history_index` / `vector_index`.
- `TOK_TOFLOAT`: coerce top of stack to float.
- `TOK_TOINT32`: coerce top of stack to int.

### Build & test
- `Makefile`: `gcc -fprofile-arcs -ftest-coverage -g`, links `-lm`.
- `test.c`: two tests, returns 0 on success.
- No external dependencies beyond libc + libm.

## 2. Third-party dependency analysis

The C project has **zero third-party dependencies** — only libc and libm.

| C dependency | Rust counterpart | Notes |
|---|---|---|
| libc (stdio, stdlib, string, ctype) | `std` (prelude, `std::fmt`, `std::str`) | Built-in |
| libm (math.h) | `std::f32` / `std::f64` inherent methods | Built-in; all 29 functions map directly |
| gcov (coverage) | `cargo-llvm-cov` (dev tool) | Optional; not a runtime dep |

**No external crates are required.** The translation is pure `std`.

## 3. Target project design (Rust)

### 3.1 Crate structure
```
mapper-expr/
├── Cargo.toml
├── src/
│   ├── lib.rs          # public API, re-exports
│   ├── token.rs        # Token enum, TokenType
│   ├── lexer.rs        # expr_lex → Lexer
│   ├── expr_node.rs    # ExprNode, ExprList (linked list → Vec)
│   ├── parser.rs       # stack-based state-machine parser
│   ├── evaluator.rs    # walk-and-evaluate
│   ├── functions.rs    # function table (29 functions)
│   └── error.rs        # ExprError enum
└── tests/
    └── integration.rs  # port of test.c (test1, test2)
```

### 3.2 Key design decisions

#### 3.2.1 Expression representation
The C code uses a **linked list** of `exprnode`. In Rust, use a **`Vec<ExprNode>`**
(flat list, same order). This is simpler, cache-friendly, and avoids manual
memory management. The "insert before trailing operator" logic in
`collapse_expr_to_left` becomes a `Vec::insert` / `Vec::splice` operation.

```rust
pub struct ExprNode {
    pub token: Token,
    pub is_float: bool,
    pub history_index: i32,
    pub vector_index: i32,
}
```

#### 3.2.2 Token
```rust
pub enum Token {
    Float(f32),
    Int(i32),
    Op(char),
    OpenParen,
    CloseParen,
    Var(char),
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Func(ExprFunc),
    Comma,
    End,
    ToFloat,
    ToInt32,
}
```

#### 3.2.3 Function table
```rust
pub enum ExprFunc {
    Pow, Sin, Cos, Tan, Abs, Sqrt, Log, Log10, Exp,
    Floor, Round, Ceil, Asin, Acos, Atan, Atan2,
    Sinh, Cosh, Tanh, Logb, Exp2, Log2, Hypot,
    Cbrt, Trunc, Min, Max, Pi,
}
```
Each variant maps to a `fn(f32) -> f32` or `fn(f32, f32) -> f32` closure.
`pi` is arity 0, `min`/`max`/`pow`/`atan2`/`hypot` are arity 2, rest are arity 1.

#### 3.2.4 Parser
Port the state-machine parser directly. The stack of `stack_obj_t` becomes:
```rust
enum StackObj {
    State(ExprState),
    Node(ExprNode),
}
```
The `PUSHSTATE`, `PUSHEXPR`, `POP`, `TOPSTATE_IS`, `APPEND_OP`, `SUCCESS`,
`FAIL` macros become helper methods on a `Parser` struct.

`collapse_expr_to_left` becomes a method that:
1. Walks the `Vec<ExprNode>` to find the insertion point.
2. Inserts coercion nodes if needed.
3. Splices the RHS list into the LHS list.
4. If constant folding is enabled and no `Var` tokens are present, evaluates
   immediately and replaces the list with a single literal node.

#### 3.2.5 Evaluator
Walk the `Vec<ExprNode>` with an operand stack (`Vec<SignalValue>`).
```rust
pub enum SignalValue {
    Float(f32),
    Int(i32),
}
```
- `Int` / `Float` → push.
- `Op` → pop two, apply, push.
- `Func` → pop arity-many, call, push.
- `Var` → look up in input/output history at `history_index` / `vector_index`.
- `ToFloat` / `ToInt32` → coerce top of stack.

#### 3.2.6 History buffer
```rust
pub struct MapperExpr {
    nodes: Vec<ExprNode>,
    vector_size: usize,
    history_size: usize,
    history_pos: usize,
    input_history: Vec<SignalValue>,
    output_history: Vec<SignalValue>,
}
```

#### 3.2.7 Public API (Rust)
```rust
pub struct MapperExpr { /* ... */ }

impl MapperExpr {
    pub fn new_from_string(
        s: &str,
        input_is_float: bool,
        output_is_float: bool,
        vector_size: usize,
    ) -> Result<Self, ExprError>;

    pub fn evaluate(
        &mut self,
        input: &SignalValue,
        output: &mut SignalValue,
    ) -> Result<(), ExprError>;
}
```
`MapperExpr` is `Drop`-safe (no manual free needed).

#### 3.2.8 Error handling
```rust
pub enum ExprError {
    LexError { position: usize, message: String },
    ParseError { message: String },
    EvalError { message: String },
    VectorIndexOutOfBounds { index: i32, size: usize },
    VectorIndexingNotImplemented,
}
```
Implements `std::fmt::Display` and `std::error::Error`.

### 3.3 Cargo.toml
```toml
[package]
name = "mapper-expr"
version = "0.1.0"
edition = "2021"
license = "BSD-3-Clause"

[dependencies]
# none — pure std

[dev-dependencies]
# none
```

### 3.4 Test port
`tests/integration.rs` ports `test.c`:
- **test1:** float mode, complex expression with `log10(pi)`, `pow`, `cos`,
  history index `x{-6*2+12}`.
- **test2:** int mode, simple arithmetic.

Both tests parse, evaluate with a known input, and compare against the
expected value computed in Rust.

### 3.5 Build & test
- `cargo build` — compiles the library.
- `cargo test` — runs integration tests.
- `cargo clippy` — lint (optional).
- `cargo-llvm-cov` — coverage (optional, replaces gcov).

## 4. Risks & mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Float precision differences** between C `float` (f32) and Rust `f32` | Low | Use `f32` throughout to match C semantics. Compare with tolerance in tests. |
| **Constant folding** produces slightly different results due to evaluation order | Medium | Port the folding logic exactly; test with the same expressions. |
| **History buffer sizing** — C allocates based on oldest index seen during parse | Medium | Track `oldest_samps` during parse; allocate `Vec` of that size. |
| **Vector indexing** is partially implemented in C (rejects N>0) | Low | Port the same restriction; document as unimplemented. |
| **State-machine parser** is complex and hard to verify | High | Port state-by-state; add unit tests for each state transition. |
| **`y=` prefix** — C parser expects `y=` at the start | Low | Port the `YEQUAL_Y` → `YEQUAL_EQ` → `EXPR` state sequence. |
| **Type coercion** — C inserts `TOK_TOFLOAT` nodes; Rust must do the same | Medium | Port the coercion logic in `collapse_expr_to_left`. |
| **No external deps** — easy to port, but also no ecosystem help | Low | Pure `std` is sufficient; all math functions are in `std::f32`. |

## 5. Translation checklist

- [ ] `Cargo.toml` with package metadata
- [ ] `src/error.rs` — `ExprError` enum
- [ ] `src/token.rs` — `Token` enum
- [ ] `src/functions.rs` — `ExprFunc` enum + function table
- [ ] `src/expr_node.rs` — `ExprNode` struct
- [ ] `src/lexer.rs` — `Lexer` (port of `expr_lex`)
- [ ] `src/parser.rs` — `Parser` (port of state-machine parser)
- [ ] `src/evaluator.rs` — `evaluate` (port of `mapper_expr_evaluate`)
- [ ] `src/lib.rs` — `MapperExpr` public API
- [ ] `tests/integration.rs` — port of `test.c`
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean (optional)
