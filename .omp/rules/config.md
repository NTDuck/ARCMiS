---
description: Do not hardcode configuration. Bubble every setting up to the call site (main or the test entry). Library code receives configuration as parameters.
---

# Configuration at the Call Site

Do not hardcode configuration. Every setting bubbles up to the call site: `main` or the test entry point.

## Rules

- Library and module code receives configuration as function parameters, builder methods, or a config struct. It never invents values itself.
- Do not put magic constants in library code. Move each setting to the call site and pass it down.
- Read environment variables and config files once, at the call site or in an explicitly passed loader. Library code does not read the environment directly.
- A timeout, URL, model name, batch size, threshold, path, or feature toggle inside library code is a violation. Move it to the call site. Thread it through as a parameter.
- Tests construct their own configuration. A test that depends on a hidden constant tests the constant, not the behavior.
- The only allowed literals are values with no caller-specific meaning: protocol constants, mathematical identities, type-level markers. When in doubt, bubble it up.

```rust
// Good: the call site owns the values
let client = rig::providers::openai::Client::from_env()?;
let agent = client
    .agent(model_name)             // model_name comes from main's config
    .preamble(&prompt)
    .temperature(temperature)      // passed in, not hardcoded
    .build();

// Bad: library code picks the values
fn summarize(text: &str) -> String {
    let client = rig::providers::openai::Client::from_env()?; // IO in a lib fn
    client.agent("gpt-5.2")        // hardcoded model
        .temperature(0.7)          // hardcoded tuning
        .build()
        .prompt(text)
        .await
}
```

## Errors

- Fallible functions return `anyhow::Result<T>`. Propagate with `?`. Do not resolve an error at its call site when the caller can react better.
- `main` is fallible: `async fn main() -> anyhow::Result<ExitCode>`.
- Add `.context(...)` at natural boundaries when the bare error loses the subject: a path, a step name.
- Library code returns a typed error only when a caller must match on the variants.
- Tool input validation returns `Result<_, String>`. The message is data for the model, not an error path.

## Serde Defaults Stay Inline

- Do not write one `const fn default_x() -> T` per defaulted field.
- Derive `Default` on the struct, give every defaulted field a `Default` impl, and write `#[serde(default)]` on the struct or field. The values sit inline as literals in the `Default` derive when the type allows, otherwise in a small manual `Default` impl next to the struct.

```rust
// Good: values inline, no helper functions
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Run {
    pub max_turns: usize,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            max_turns: 14,
        }
    }
}

// Bad: one helper function per field
#[serde(default = "default_max_turns")]
pub max_turns: usize,

const fn default_max_turns() -> usize {
    14
}
```

## Why

- Call-site configuration shows every dependency in one place. A reader of `main` sees the full behavior surface.
- Tests can vary each setting. That is how you test boundaries and invariants.
- Hardcoded values hide coupling. They make one call site the only correct caller.
