//! `laya`: command-line prediction and checkpoint conversion.

use candle_core::{DType, Device};
use clap::{Parser, Subcommand};
use laya::agent::parse_dtype;
use laya::AgentBuilder;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "laya", version, about = "Laya typed decisions")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Answer typed questions about a state.
    Predict {
        #[arg(long, default_value = "convaiinnovations/laya")]
        model: String,
        #[arg(long)]
        subfolder: Option<String>,
        #[arg(long)]
        revision: Option<String>,
        #[arg(long, default_value = "float16")]
        dtype: String,
        /// Plain text input.
        #[arg(long, conflicts_with = "state_file")]
        state: Option<String>,
        /// JSON state file.
        #[arg(long, conflicts_with = "state")]
        state_file: Option<PathBuf>,
        /// JSON question definitions.
        #[arg(long)]
        questions: PathBuf,
        #[arg(long, default_value_t = 16)]
        batch_size: usize,
        /// Force CPU even when a GPU backend is compiled in.
        #[arg(long)]
        cpu: bool,
        /// Report per-sequence token counts and whether any state was cut to
        /// `max_len`. Off by default so the output matches upstream's.
        #[arg(long)]
        usage_detail: bool,
    },
    /// Re-export a checkpoint with canonical names and a chosen dtype.
    Convert {
        #[arg(long, default_value = "convaiinnovations/laya")]
        model: String,
        #[arg(long)]
        subfolder: Option<String>,
        #[arg(long)]
        revision: Option<String>,
        #[arg(long, default_value = "float16")]
        dtype: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Show which checkpoint a state would be routed to, without loading one.
    Route {
        #[arg(long, conflicts_with = "state_file")]
        state: Option<String>,
        #[arg(long, conflicts_with = "state")]
        state_file: Option<PathBuf>,
        #[arg(long)]
        questions: Option<PathBuf>,
    },
}

fn select_device(force_cpu: bool) -> Device {
    if force_cpu {
        return Device::Cpu;
    }
    #[cfg(feature = "metal")]
    if let Ok(device) = Device::new_metal(0) {
        return device;
    }
    #[cfg(feature = "cuda")]
    if let Ok(device) = Device::new_cuda(0) {
        return device;
    }
    Device::Cpu
}

fn read_state(state: Option<String>, state_file: Option<PathBuf>) -> Result<Value, String> {
    match (state, state_file) {
        (Some(text), _) => Ok(Value::String(text)),
        (None, Some(path)) => {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
        }
        (None, None) => Err("one of --state or --state-file is required".into()),
    }
}

fn resolve(
    model: &str,
    subfolder: Option<&str>,
    revision: Option<&str>,
) -> Result<PathBuf, String> {
    let local = PathBuf::from(model);
    if local.is_dir() {
        return Ok(match subfolder {
            Some(sub) => local.join(sub),
            None => local,
        });
    }
    #[cfg(feature = "hub")]
    {
        laya::hub::resolve_model(model, subfolder, revision, None).map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "hub"))]
    {
        let _ = revision;
        Err(format!(
            "{model} is not a local directory, and this build has no Hub support"
        ))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Predict {
            model,
            subfolder,
            revision,
            dtype,
            state,
            state_file,
            questions,
            batch_size,
            cpu,
            usage_detail,
        } => {
            let state = read_state(state, state_file)?;
            let questions: Value = serde_json::from_str(&std::fs::read_to_string(&questions)?)?;
            let dir = resolve(&model, subfolder.as_deref(), revision.as_deref())?;
            let agent = AgentBuilder::new()
                .dtype(parse_dtype(&dtype)?)
                .device(select_device(cpu))
                .batch_size(batch_size)
                .build(dir)?;
            let result = agent.predict(&state, &questions)?;
            let rendered = if usage_detail {
                result.to_value_detailed()
            } else {
                result.to_value()
            };
            println!("{}", serde_json::to_string_pretty(&rendered)?);
        }
        Command::Convert {
            model,
            subfolder,
            revision,
            dtype,
            output,
        } => {
            let dir = resolve(&model, subfolder.as_deref(), revision.as_deref())?;
            let dtype = parse_dtype(&dtype)?;
            let written = laya::convert::convert(&dir, &output, dtype)?;
            println!(
                "{}",
                serde_json::json!({"output": written.display().to_string()})
            );
        }
        Command::Route {
            state,
            state_file,
            questions,
        } => {
            let state = read_state(state, state_file)?;
            let questions: Option<Value> = match questions {
                Some(path) => Some(serde_json::from_str(&std::fs::read_to_string(path)?)?),
                None => None,
            };
            let router = laya::Router::new();
            let decision = router.route(
                &state,
                questions.as_ref().and_then(Value::as_object),
                None,
                None,
                None,
            )?;
            println!("{}", serde_json::to_string_pretty(&decision.to_value())?);
        }
    }
    let _: Option<DType> = None;
    Ok(())
}
