# Translation Design: mapper_expr (C) → Rust

## 1. Source Project Overview

`mapper_expr` is a small, self-contained expression parser/evaluator extracted from
`libmapper` (BSD-licensed, author Stephen Sinclair). It parses a string of the form
`y=<expression>` into a linked-list AST and evaluates it against a single input
variable `x`, producing a single output `y`. It supports integer and float
arithmetic, a table of ~28 math functions, and history indexing `x{N}` (negative
index = samples in the past).

### Files
- `mapper_expr.h` — public C API (3 functions).
- `mapper_expr.c` — lexer, recursive-descent-ish stack parser, AST, evaluator,
  constant folding, function table.
- `test.c` — two tests (float and int mode) with expected-value checks.
- `Makefile` — builds a `test` binary with gcov coverage flags.
- `COPYING` — BSD-3-Clause license.
- `README` — provenance note.

### Public API (C)
```c
mapper_expr mapper_expr_new_from_string(const char *str,
                                        int input_is_float,
                                        int output_is_float,
                                        int vector_size);
int mapper_expr_evaluate(mapper_expr expr, void* input_vector, void* output_vector);
void mapper_expr_free(mapper_expr expr);
```
`input_is_float`/`output_is_float` are 0/1 flags selecting int32 vs float32
semantics. `vector_size` bounds `x[N]` vector indexing (only `N==0` is implemented;
`N>0` fails).

### Key internal concepts
- **Token** (`token_t`): FLOAT, INT, OP, parens, VAR(x/y), square/curly brackets,
  FUNC, COMMA, END, TOFLOAT, TOINT32.
- **AST node** (`exprnode`): a token + `is_float` flag + `history_index` +
  `vector_index` + `next` pointer (singly-linked list, RPN-ish order).
- **Parser**: a hand-written state machine over a fixed-size stack
  (`STACK_SIZE 256`) of states and partial expressions; collapses sub-expressions
  left-associatively (`collapse_expr_to_left`) with constant folding.
- **Evaluator** (`mapper_expr_evaluate`): walks the node list, maintains an
  evaluation stack, resolves `x` from input history, applies functions.
- **History**: `input_history`/`output_history` arrays of `mapper_signal_value_t`
  (union of float/int32), `history_size`/`history_pos` track the ring.
- **Function table**: name + arity + function pointer; includes `min`, `max`, `pi`
  as custom C helpers.

### Build/test
- `make` builds `test`; `make coverage` runs it and generates gcov reports.
- Tests are plain C functions returning 0/1, aggregated in `main`.

## 2. Third-Party Library Analysis

The C project has **no third-party dependencies** — only the C standard library
(`ctype.h`, `math.h`, `stdio.h`, `stdlib.h`, `string.h`) and libm.

Mapping to Rust:
| C dependency | Rust counterpart | Notes |
|---|---|---|
| `math.h` (libm) | `std::f32`/`std::f64` inherent methods + `std::f32::consts::PI` | All needed functions exist as `f32` methods (verified: `sin, cos, tan, abs, sqrt, ln, log10, exp, floor, round, ceil, asin, acos, atan, atan2, sinh, cosh, tanh, exp2, log2, cbrt, trunc, hypot, powf`). `logb` → `log2().floor()`; `min`/`max` → `f32::min`/`f32::max`; `pi` → `std::f32::consts::PI`. |
| `ctype.h` | `char::is_ascii_digit`, `char::is_ascii_alphabetic`, `char::is_ascii_alphanumeric` | Direct equivalents. |
| `stdlib.h` (atoi/atof/malloc/free) | `str::parse`, `Vec`/`Box`/`String` | No manual memory management needed. |
| `stdio.h` (printf) | `println!` / `eprintln!` | Debug tracing only. |
| `string.h` (strncmp) | `str` slicing / `starts_with` | For function-name lookup. |

**Result: zero external crates required.** The translation is pure `std`. This is
the idiomatic choice — pulling in a parser combinator crate (e.g. `nom`) would be
over-engineering for a ~700-line hand-written parser and would change the
algorithm. We keep the same lexer/parser structure for fidelity.

## 3. Target Project Design (Rust)

### Crate layout
```
mapper_expr/
├── Cargo.toml
├── src/
│   ├── lib.rs          # public API + re-exports
│   ├── token.rs        # Token enum, TokenType
│   ├── lexer.rs        # expr_lex
│   ├── ast.rs          # ExprNode (AST), NodeKind
│   ├── parser.rs       # state-machine parser + collapse_expr_to_left
│   ├── evaluator.rs    # mapper_expr_evaluate + history
│   ├── functions.rs    # function table (name, arity, fn)
│   └── error.rs        # ExprError
└── tests/
    └── integration.rs  # test1, test2 (port of test.c)
```

### Public API (Rust)
```rust
pub struct MapperExpr { /* node list, vector_size, history, ... */ }

impl MapperExpr {
    pub fn new_from_string(
        expr: &str,
        input_is_float: bool,
        output_is_float: bool,
        vector_size: usize,
    ) -> Result<Self, ExprError>;

    /// Evaluate with a single input sample; returns the output sample.
    pub fn evaluate(&mut self, input: f32) -> f32;
    pub fn evaluate_i32(&mut self, input: i32) -> i32;
}
```
Design decisions:
- Replace the C `void*` in/out vectors with typed methods. The C API's
  `input_is_float`/`output_is_float` flags become two methods (`evaluate` for
  f32, `evaluate_i32` for i32) — more idiomatic than a `bool` + `void*`.
- Return `Result<Self, ExprError>` instead of null-pointer for parse failures.
- `MapperExpr` owns its AST and history; `Drop` replaces `mapper_expr_free`.

### AST representation
C uses a singly-linked list of `exprnode`. In Rust we use an owned linked list
via `Option<Box<ExprNode>>` to preserve the exact traversal/collapse semantics,
or (cleaner) a `Vec<ExprNode>` in the same RPN order. **Recommendation: `Vec`**
— the C code only ever appends and walks forward; a `Vec` is simpler, avoids
manual `Box` chains, and keeps the same logical order. `collapse_expr_to_left`
becomes a splice on the `Vec`.

### Token / NodeKind
```rust
pub enum TokenType {
    Float(f32), Int(i32), Op(char),
    OpenParen, CloseParen, Var(char),
    OpenSquare, CloseSquare, OpenCurly, CloseCurly,
    Func(usize), Comma, End, ToFloat, ToInt32,
}
```
`Func(usize)` indexes the function table (mirrors `expr_func_t`).

### Function table
```rust
pub struct FuncDef { name: &'static str, arity: usize, f: Func }
```
Use an enum of function variants + a dispatch `fn apply(f: FuncVariant, args: &[f32]) -> f32`
rather than raw function pointers (Rust closures with a fixed signature work, but
an enum dispatch is cleaner and avoids `dyn`/`Fn` trait-object overhead). `min`,
`max`, `pi` are trivial Rust closures.

### Parser
Port the state machine faithfully:
- `stack: Vec<StackObj>` where `StackObj = State(state_t) | Node(Vec<ExprNode>)`.
- Same `state_t` enum (YEQUAL_Y, YEQUAL_EQ, EXPR, EXPR_RIGHT, TERM, TERM_RIGHT,
  VALUE, NEGATE, VAR_RIGHT, VAR_VECTINDEX, VAR_HISTINDEX, CLOSE_VECTINDEX,
  CLOSE_HISTINDEX, OPEN_PAREN, CLOSE_PAREN, COMMA, END).
- `collapse_expr_to_left` → splice the right-hand `Vec` before the trailing op of
  the left-hand `Vec`, inserting a `ToFloat` coercion node when types disagree,
  and constant-folding when no variable is referenced.
- Errors: return `ExprError` with a message (replaces `FAIL(msg)` + null return).

### Evaluator
- Walk the node list with an evaluation stack (`Vec<Value>`).
- `Value = F32(f32) | I32(i32)` (mirrors `mapper_signal_value_t` union).
- Resolve `x` from `input_history` ring buffer using `history_index`.
- Apply functions via the dispatch.
- Maintain `input_history`/`output_history` as `Vec<Value>` with `history_pos`.

### Error handling
```rust
#[derive(Debug)]
pub enum ExprError {
    Lexical(String),
    Parse(String),
    VectorIndexOutOfRange { index: i32, size: usize },
    VectorIndexingNotImplemented,
    Evaluation(String),
}
```

### Testing
Port `test.c` into `tests/integration.rs`:
- `test1`: float mode, `y=26*2/2+log10(pi)+2.*pow(2,1*(3+7*.1)*1.1+x{-6*2+12})*3*4+cos(2.)`,
  x=3.0, compare against the C expected expression (use `assert!((out - expected).abs() < 1e-4)`).
- `test2`: int mode, `y=26*2/2+x*30/(20*1)`, x=3 and x=321, exact integer compare.
- Add unit tests for the lexer, function table, and edge cases (empty string,
  unknown function, out-of-range vector index).

### Cargo.toml
```toml
[package]
name = "mapper_expr"
version = "0.1.0"
edition = "2021"
license = "BSD-3-Clause"

[lib]
name = "mapper_expr"
path = "src/lib.rs"

# No external dependencies.
```

## 4. Translation Risks

1. **Float precision / rounding**: C uses `float` (f32) throughout; Rust `f32`
   matches. But C's `atoi`/`atof` parsing and Rust's `str::parse` may differ on
   edge cases (e.g. `2.` → 2.0). The lexer's special handling of `2.` (digit then
   `.`) must be reproduced exactly. Tests use a 1e-4 tolerance to absorb minor
   libm differences.
2. **`logb`/`log2`/`exp2` availability**: C's `logbf`/`log2f`/`exp2f` map to
   `f32::log2`, `f32::exp2`. `logb` (exponent of the floating-point
   representation) has no direct `f32` method — implement as
   `x.log2().floor()` (verified equivalent for normal numbers).
3. **Constant folding correctness**: `collapse_expr_to_left` evaluates a
   sub-expression immediately when it contains no variables. The Rust port must
   preserve the exact insertion/coercion logic or results will diverge.
4. **History indexing (`x{N}`)**: negative indices reference past samples. The
   ring-buffer semantics (`history_pos`, `history_size`) must be ported exactly;
   off-by-one here breaks `test1` (which uses `x{-6*2+12}` = `x{0}`).
5. **State-machine parser fidelity**: the C parser is a hand-written stack machine
   with subtle `top`/`top-1`/`top-2` indexing. A faithful port is mechanical but
   error-prone; the `die_unless` assertions become `debug_assert!` or `Result`
   errors.
6. **`void*` API → typed API**: changing the public signature is a deliberate
   idiomatic improvement but breaks ABI compatibility with the C API. This is
   acceptable since the target is a Rust crate, not a C FFI shim. If FFI
   compatibility is ever needed, a separate `#[no_mangle] extern "C"` wrapper can
   be added later.
7. **Vector indexing (`x[N]`)**: only `N==0` is implemented in C; `N>0` returns
   an error. Preserve this limitation and the error message.
8. **`DEBUG`/`TRACING` macros**: C uses `#ifdef DEBUG` for tracing. In Rust, use
   `log` crate or `eprintln!` behind a feature flag (`#[cfg(feature = "debug")]`).
   For the initial port, keep tracing minimal (`eprintln!` in a `debug_assert!`
   block) to avoid adding a dependency.

## 5. Verification Plan

1. `cargo build` — compiles cleanly with no warnings.
2. `cargo test` — all ported tests pass (test1 float, test2 int).
3. `cargo clippy` — no lints.
4. Manual spot-check: evaluate `y=x+1` with x=5 → 6 (both int and float modes).
5. Edge cases: empty string → error; `y=pi` → 3.14159...; `y=min(3,5)` → 3.
