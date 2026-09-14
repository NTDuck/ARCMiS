---
description: Rust style. Use fully qualified paths and derives. Chain with tap instead of nesting calls. Use functional style inside functions.
---

# Rust Style

This rule applies to all Rust code in this repository.

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

## 3. Functional Style Inside Functions

- Prefer iterator adapters (`map`, `filter`, `fold`, `try_fold`, `collect`) over manual loops and `Vec::push` accumulation.
- Avoid `mut` accumulators and reassignments inside a function. Build new values. Do not mutate in place.
- Return early for guard clauses. Keep the happy path as one tail expression where practical.
- This rule does not apply at module or architecture level. It governs the inside of a function body only. Keep type-first design, explicit boundaries, and exhaustive error handling as they are.

## 4. Fully Qualified Derives

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
