---
description: Order and format dependency declarations. Pin full versions. Prefer workspace inheritance.
---

# Cargo Manifests

## 1. Declaration Order

Order the entries in every `[dependencies]` and `[dev-dependencies]` table in three groups. Sort alphabetically inside each group.

1. `ARCMiS-*` crates.
2. Shared workspace crates (declared in `[workspace.dependencies]`).
3. All other crates.

## 2. Version Pins

Pin every workspace dependency to its latest stable version with all three parts: `x.y.z`, not `x.y`.

## 3. Workspace Inheritance

Prefer workspace inheritance:

```toml
foo.workspace = true
tokio = { workspace = true, features = ["full"] }
```

Add features inline next to `workspace = true`.

## 4. Inline Form

Collapse a dependency with exactly one attribute to dotted form when the attribute is `version`:

```toml
foo.version = "1.2.3"
```

Keep the table form for package/path aliases and for dependencies with two or more attributes:

```toml
foo = { version = "1.2.3", features = ["derive"] }
bar = { package = "ARCMiS-bar", path = "../bar" }
```

## 5. Workspace Table

Declare shared dependencies in `[workspace.dependencies]` in the root `Cargo.toml`. Members inherit from there.
