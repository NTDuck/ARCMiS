//! Laya typed decisions in Rust.
//!
//! An independent Rust port of the Laya decision models: a bidirectional
//! encoder answers constrained questions — a choice, a rubric score, or the
//! probability of a proposition — in one forward pass, with no token-by-token
//! decoding and no generated JSON.
//!
//! ```no_run
//! use laya::Agent;
//! use serde_json::json;
//!
//! let agent = Agent::from_pretrained("aac6fef/laya-mlx")?;
//! let result = agent.predict(
//!     &json!("I was billed twice. Please refund the duplicate."),
//!     &json!({
//!         "department": {
//!             "type": "choice",
//!             "instructions": "Who should handle this?",
//!             "criteria": ["billing", "technical", "sales"]
//!         }
//!     }),
//! )?;
//! println!("{}", result.answers["department"]);
//! # Ok::<(), laya::Error>(())
//! ```
//!
//! Unlike the MLX package this is ported from, inference runs on CPU, CUDA and
//! Metal. Build with `--features metal` or `--features cuda` for GPU support.

pub mod agent;
pub mod config;
pub mod convert;
pub mod model;
pub mod router;
pub mod weights;

#[cfg(feature = "hub")]
pub mod hub;

pub use agent::{Agent, AgentBuilder, Answer, Prediction};
pub use config::{AgentConfig, EncoderConfig};
pub use model::{DecisionModel, ModernBert};
pub use router::{ModelName, RouteDecision, Router};

pub use laya_core::{calibrate, email, lang, presets, prompt, question};
pub use laya_core::{Criteria, Question, QuestionKind, Tokenizer};

/// Errors raised by the runtime.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] laya_core::Error),

    #[error(transparent)]
    Candle(#[from] candle_core::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Not a complete Laya checkpoint: {0} is missing")]
    IncompleteCheckpoint(std::path::PathBuf),

    #[error("{0}")]
    Config(String),

    #[error("Non-finite model outputs; retry with dtype=\"float32\"")]
    NonFinite,

    #[error("{0}")]
    Hub(String),
}

pub type Result<T> = std::result::Result<T, Error>;
