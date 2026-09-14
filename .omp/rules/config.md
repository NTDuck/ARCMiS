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
let client = ::rig::providers::openai::Client::from_env()?;
let agent = client
    .agent(model_name)             // model_name comes from main's config
    .preamble(&prompt)
    .temperature(temperature)      // passed in, not hardcoded
    .build();

// Bad: library code picks the values
fn summarize(text: &str) -> String {
    let client = ::rig::providers::openai::Client::from_env()?; // IO in a lib fn
    client.agent("gpt-5.2")        // hardcoded model
        .temperature(0.7)          // hardcoded tuning
        .build()
        .prompt(text)
        .await
}
```

## Why

- Call-site configuration shows every dependency in one place. A reader of `main` sees the full behavior surface.
- Tests can vary each setting. That is how you test boundaries and invariants.
- Hardcoded values hide coupling. They make one call site the only correct caller.
