---
description: Workspace crates are ARCMiS-<name>. Aliases make imports short. Never write a lowercase arcmis- name.
---

# Crate and Import Naming

## 1. Package Names

Name every workspace crate `ARCMiS-<name>`. Preserve the case and use a hyphen as separator.

- Correct: `ARCMiS-agents`, `ARCMiS-tools`.
- Wrong: `arcmis-agents`, `Arcmis-agents`.

## 2. Dependency Aliases

A crate that depends on an `ARCMiS-<name>` crate must alias the dependency to the bare name:

```toml
foo = { package = "ARCMiS-foo", path = "..." }
agents = { package = "ARCMiS-agents", version = "0.1.0" }
```

## 3. Imports

Import and refer through the alias in Rust code:

```rust
use ::foo::Item;
use ::agents::Config;
use ::tools::Catalog;
```

Never write an import path like `arcmis_foo::Item`.

## 4. Integration Tests

Import the crate under test through the same alias. Set the lib name in the crate manifest and write the plain alias import in `tests/*.rs`:

```toml
[lib]
name = "foo"
```

```rust
use ::foo::Item;
```

Do not write a `::ARCMiS_<name>` import. The alias is one name for one thing, in dependencies and self tests alike.

## 5. Enforcement

Mechanical: `lint-rules.py` flags lowercase `arcmis-` package names and `arcmis_` imports.
