# laya-rs

**Laya typed decisions, in Rust.** A port of
[laya-mlx](https://github.com/mizorewww/laya-mlx) from MLX/Python to
[candle](https://github.com/huggingface/candle).

A bidirectional encoder answers constrained questions — a choice, a rubric score, or the
probability of a proposition — in one forward pass. No token-by-token decoding, no
generated JSON, **0 output tokens**.

> The package is `laya-rs` because `laya` was taken on crates.io by an unrelated project.
> The library target is still named `laya`, so imports read `use laya::…`.

```toml
[dependencies]
laya-rs = "0.1"
```

```rust
use laya::Agent;
use serde_json::json;

let agent = Agent::from_pretrained("aac6fef/laya-mlx")?;
let result = agent.predict(
    &json!("I was billed twice. Please refund the duplicate."),
    &json!({
        "department": {
            "type": "choice",
            "instructions": "Who should handle this?",
            "criteria": ["billing", "technical", "sales"]
        }
    }),
)?;

println!("{}", result.answer("department").unwrap().choice.as_ref().unwrap());
```

## What this adds over the Python package

The MLX package is macOS + Apple Silicon only. This runs on **CPU, CUDA and Metal**,
across Linux, macOS and Windows, as a single static binary with no Python runtime.

## CLI

The `cli` feature (on by default) builds a `laya-rs` binary:

```bash
laya-rs predict --model aac6fef/laya-mlx --state "I was billed twice" --questions questions.json
laya-rs route   --state "Мой счёт был дважды списан"
laya-rs convert --model convaiinnovations/laya --output ./models/laya --dtype float16
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `cli` | yes | the `laya-rs` binary |
| `hub` | yes | download checkpoints from the Hugging Face Hub |
| `metal` | no | Apple GPU acceleration |
| `cuda` | no | NVIDIA GPU acceleration |
| `mkl` / `accelerate` | no | faster CPU inference |

## License

Apache-2.0. See [NOTICE](https://github.com/bob-rietveld/laya-rs/blob/main/NOTICE) for
attribution: upstream Laya is by Convai Innovations, and the MLX port this is based on is
`mizorewww/laya-mlx`. Neither endorses this project.
