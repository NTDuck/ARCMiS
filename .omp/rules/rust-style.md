---
description: Rust style — fully qualified paths and derives, tap-chained instead of nested calls, functional in-function style.
---

# Rust Style

Applies to all Rust code in this repository.

## 1. Fully Qualified Paths

Always use fully qualified paths — leading `::` — for crates, macros, and traits. Instead of `foo::bar`, write `::foo::bar`.

- Paths in expressions, types, `use` items, and attributes all get the leading `::`.
- Traits from built-in crates carry their real module path: `::core::fmt::Debug`, `::core::clone::Clone`, `::core::ops::Deref`. Do not guess; verify against rustdoc.
- Exception: inside the crate itself, items of the current crate stay bare (`crate::module::Item` or plain `Item` in-module). The rule targets external and prelude names.

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

Instead of `foo(bar(baz()))`, chain with `.tap()` (from the [`tap`](https://docs.rs/tap) crate) or equivalent. Pipelining reads top-to-bottom; nesting reads inside-out.

```rust
// Good
use ::tap::Tap;

let result = baz()
    .tap(|v| bar(v))
    .tap(|v| foo(v));

// Bad
let result = foo(bar(baz()));
```

- Order the chain so data flows downward; each stage transforms or inspects.
- Prefer `.tap_mut`/`.tap_ok`/`.tap_err` variants when the intent is inspection vs. transformation.

## 3. Functional Style Within Functions

- Prefer iterator adapters (`map`, `filter`, `fold`, `try_fold`, `collect`) over manual loops and `Vec::push` accumulation.
- Avoid `mut` accumulators and reassignments inside a function; build new values instead of mutating in place.
- Early return for guard clauses; keep the happy path a single tail expression where practical.
- Does not apply at module/architecture level: type-first design, explicit boundaries, and exhaustively handled errors are unaffected. This governs *within* a function body.

## 4. Fully Qualified Derives

Fully qualify derive macro paths too. Instead of `#[derive(Debug)]`, write the attribute with crate-qualified macro paths:

```rust
#[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::default::Default)]
struct Point {
    x: f64,
    y: f64,
}
```

- `Debug` lives in `::core::fmt`, `Clone`/`Copy` in `::core::clone`/`::core::marker`, comparison traits in `::core::cmp`, `Default` in `::core::default`.
- Note: the harness clippy config sets `absolute-paths-max-segments = 0`; that lint is `allow`-by-default, but if it is later enabled, either raise the cap in `.clippy.toml` or scope the lint to allow derives.
