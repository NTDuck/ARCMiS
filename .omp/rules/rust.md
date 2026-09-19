---
description: Plain paths. Write `foo::bar`, not `::foo::bar`. Chain with tap instead of nesting calls. Use functional style inside functions.
---

# Rust Style

This rule applies to all Rust code in this repository. `python3 .omp/scripts/lint-rules.py` enforces sections 1, 4, 5, and 9 mechanically. Review enforces the other sections. Build and test gates live in `.omp/rules/build-and-gates.md`.

## 1. Plain Paths

Write plain paths. Do not write a leading `::`. Write `foo::bar`, not `::foo::bar`.

- Paths in expressions, types, `use` items, and attributes stay bare.
- Prelude and standard-library names stay bare: `Debug`, `Clone`, `String`, `Vec`, `format!`.
- Do not write the verbose module path for a prelude name. Write `String`,
  not `std::string::String`. Write `Some`, not `core::option::Option::Some`.
  Write `format!`, not `std::format!`. The linter fails these forms.
- Names from a `use` import stay bare at the call site. Do not re-qualify them.

```rust
// Good
use core::ops::Add;
let doubled = itertools::iproduct!(xs, ys).count();
let f = HashMap::<String, u32>::new();

// Bad
use ::core::ops::Add;
let f = ::std::collections::HashMap::<::std::string::String, u32>::new();
```

2026-09-18: this section replaces the fully-qualified-path rule of ADR 0010.
The `::` noise hid each item behind a wall of qualifiers. See ADR 0016.
2026-09-19: the verbose `std::*`/`core::*` prelude paths gained a mechanical
detector in `lint-rules.py`. See ADR 0017.

## 2. Tap Chaining, Not Nesting

Write `foo(bar(baz()))` as a chain of `.tap()` calls (from the [`tap`](https://docs.rs/tap) crate) or an equivalent. A pipeline reads top to bottom. A nest reads inside out.

```rust
// Good
use tap::Tap;

let result = baz()
    .tap(|v| bar(v))
    .tap(|v| foo(v));

// Bad
let result = foo(bar(baz()));
```

- Order the chain so data flows down. Each stage transforms the value or inspects it.
- Prefer `.tap_mut`, `.tap_ok`, or `.tap_err` when the intent is inspection. Use plain `.tap` for transformation.
- This is a requirement, not a suggestion. Rewrite every nested call. For `Result` and `Option`, prefer the standard `inspect` and `inspect_err` for side effects. Use the `tap` crate for non-`Result` values. A diff with a `foo(bar(` shape inside library code fails review.

## 3. Functional Style Inside Functions

- Prefer iterator adapters (`map`, `filter`, `fold`, `try_fold`, `collect`) over manual loops and `Vec::push` accumulation.
- Avoid `mut` accumulators and reassignments inside a function. Build new values. Do not mutate in place.
- Return early for guard clauses. Keep the happy path as one tail expression where practical.
- This rule does not apply at module or architecture level. It governs the inside of a function body only. Keep type-first design, explicit boundaries, and exhaustive error handling as they are.

## 4. Explicit Names

Name every binding for its content. A name states what the value is.

- Do not abbreviate. Write `error`, not `e`. Write `output`, not `o`. Write `config`, not `c`.
- Loop variables and match arms follow the same rule. Write `item`, not `i`, when the value is not an index.
- Use one-letter names in two cases only: the loop index of a tiny numeric loop, and generic type parameters (`T`, `E`).

## 5. Resolve Types With the Turbofish

Do not annotate a binding when the type comes from the expression. Use the turbofish on the method that needs it. Write `let items = values.collect::<Vec<Item>>();`, not `let items: Vec<Item> = values.collect();`.

- Prefer `::<T>` on `collect`, `parse`, `downcast`, and similar methods.
- Keep type annotations for struct fields, function signatures, and consts. This rule targets local bindings only.

## 6. Function Order: Caller Before Callee

This is the Stepdown Rule of `code-clarity.md`, applied to functions.
Within one file, order functions so a reader sees the high level first:
the caller comes before each callee, and each next function sits one
level lower. If `pub fn foo()` calls private `bar()` and `baz()`, then
`foo()` comes first. `bar()` and `baz()` come immediately after it,
before any other `pub fn`. Types follow the same rule: the type the
reader meets first comes first in the file. Example: declare `Registry`
before `Agent` when readers need the registry concept first. The full
ordering law for every entity: `code-clarity.md`.

## 7. Method Call, Not Fully Qualified Function

Write `foo.clone()`, not `Clone::clone(foo)`. A method call puts the value first.

```rust
// Good
let copy = name.clone();
let text = config.to_string();

// Bad
let copy = Clone::clone(&name);
let copy = std::clone::Clone::clone(&name);
```

2026-09-18: added with ADR 0016. The fully qualified function form added
noise, no information.

## 8. Structs Over Free Functions

Name an operation group with a struct when the struct names the concept.
Call `Tracing::init()`, not `init_tracing()`. Call `MonolithTask::build(config)`,
not `monolith_task(config)`. A free function stays when the operation is a
plain computation with no concept of its own (`truncate_args`).

## 9. Plain Macros and Derives

Every macro invocation uses its crate path with a bang. No leading `::`. Write `serde_json::json!`, `format!`, `tracing::info!`, `assert_eq!`. Derive macro paths stay bare: `#[derive(Debug, Clone, Default)]`.

- Macros in the standard prelude (`format!`, `vec!`, `assert_eq!`, `assert!`, `write!`) stay bare.
- Other macros carry the crate path: `serde_json::json!`, `tracing::info!`.
- Do not import a macro with `use` and call it bare. The invocation carries the crate path.

```rust
// Good
#[derive(Debug, Clone, Default)]
struct Point {
    x: f64,
    y: f64,
}

let payload = serde_json::json!({ "path": path });
tracing::info!(path = %path, "read file");

// Bad
#[derive(::core::fmt::Debug, ::core::clone::Clone)]
let payload = ::serde_json::json!({ "path": path });
tracing::info!("read {}", path);
```
