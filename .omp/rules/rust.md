---
description: Rust style. Use fully qualified paths and derives. Chain with tap instead of nesting calls. Use functional style inside functions.
---

# Rust Style

This rule applies to all Rust code in this repository. `python3 .omp/scripts/lint-rules.py` enforces sections 1, 4, and 9 mechanically. Review enforces the other sections. Build and test gates live in `.omp/rules/build-and-gates.md`.

## 1. Fully Qualified Paths

Always use fully qualified paths for crates, macros, and traits. Write a leading `::`. Write `::foo::bar`, not `foo::bar`.

- Paths in expressions, types, `use` items, and attributes all get the leading `::`.
- Traits from built-in crates carry their real module path: `::core::fmt::Debug`, `::core::clone::Clone`, `::core::ops::Deref`. Do not guess. Check rustdoc.
- Exception: items of the current crate stay bare inside it (`crate::module::Item` or plain `Item` in-module). This rule targets external and prelude names.

```rust
// Good
use ::core::ops::Add;
let doubled = ::itertools::iproduct!(xs, ys).count();
let f = ::std::collections::HashMap::<::std::string::String, u32>::new();

// Bad
use core::ops::Add;
let f = HashMap::<String, u32>::new();
```

## 2. Tap Chaining, Not Nesting

Write `foo(bar(baz()))` as a chain of `.tap()` calls (from the [`tap`](https://docs.rs/tap) crate) or an equivalent. A pipeline reads top to bottom. A nest reads inside out.

```rust
// Good
use ::tap::Tap;

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

Order functions so a reader sees the high level first. If `pub fn foo()` calls private `bar()` and `baz()`, then `foo()` comes first. `bar()` and `baz()` come immediately after it, before any other `pub fn`. Types follow the same rule: the type the reader meets first comes first in the file. Example: declare `Registry` before `Agent` when readers need the registry concept first.

## 7. Builders via bon

Use the [`bon`](https://docs.rs/bon) crate for constructible types and multi-parameter functions. Derive `::bon::Builder` on the struct. Use `#[::bon::builder]` on the function. Construct with named setters at the call site. Skip the builder in two cases. A type with one required field and no options keeps a plain constructor (`Foo::new`). A call site that would read as `Foo::builder().build()` with nothing set must not use a builder.

## 8. Fully Qualified Derives

Fully qualify derive macro paths. Write `#[derive(Debug)]` with the crate-qualified path:

```rust
#[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::default::Default)]
struct Point {
    x: f64,
    y: f64,
}
```

- `Debug` lives in `::core::fmt`. `Clone` and `Copy` live in `::core::clone` and `::core::marker`. Comparison traits live in `::core::cmp`. `Default` lives in `::core::default`.
- Note: the clippy config sets `absolute-paths-max-segments = 0`. This lint is off by default. If you enable it later, raise the cap in `.clippy.toml` or scope the lint to allow derives.
