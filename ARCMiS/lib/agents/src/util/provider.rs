//! Provider selection for the agent clients. Ollama serves the local
//! models over its native protocol; netmind serves remote models over an
//! OpenAI-compatible chat-completions endpoint. Both clients produce the
//! same rig `Agent` through the blanket `AgentClientExt`, so every agent
//! builder stays provider-agnostic and generic over the client type.

use crate::util::config::Run;
use anyhow::Context as _;
use rig::providers::ollama;
use rig::providers::openai;

/// Selected model provider.
#[derive(Debug, Clone)]
pub enum Provider {
    /// Local ollama daemon. `base_url` overrides the `OLLAMA_API_BASE_URL`
    /// environment default.
    Ollama {
        base_url: Option<String>,
    },
    /// Remote OpenAI-compatible gateway (netmind). `api_key` and the chat
    /// completions base URL both come from the call site, never from a
    /// hardcoded value.
    Netmind {
        api_key: String,
        base_url: String,
    },
}

impl Provider {
    /// Resolve the provider from the CLI flags. `netmind` requires the api
    /// key flag; ollama is the default when no name is given.
    pub fn from_cli(name: Option<String>, api_key: Option<String>, base_url: Option<String>) -> anyhow::Result<Self> {
        match name.as_deref() {
            None | Some("ollama") => Ok(Self::Ollama {
                base_url,
            }),
            Some("netmind") => {
                let key = api_key.context("netmind provider needs --api-key")?;
                Ok(Self::Netmind {
                    api_key: key,
                    base_url: base_url.unwrap_or_else(|| "https://netmind.viettel.vn/gateway/v1".to_owned()),
                })
            },
            Some(other) => Err(anyhow::anyhow!("unknown provider {other}; expected ollama or netmind")),
        }
    }

    /// Build the concrete client for this provider. The caller matches on
    /// the provider variant and passes the client to the generic run
    /// functions; there is no type erasure.
    pub fn client(&self) -> anyhow::Result<Clients> {
        match self {
            Self::Ollama {
                base_url,
            } => {
                // Ollama needs no key, but the builder's type state requires
                // one before build. An empty key maps to no auth header.
                let mut builder = ollama::Client::builder().api_key("");
                if let Some(url) = base_url {
                    builder = builder.base_url(url.clone());
                }
                Ok(Clients::Ollama(builder.build().context("ollama client build failed")?))
            },
            Self::Netmind {
                api_key,
                base_url,
            } => {
                // The gateway speaks the Chat Completions route. The
                // responses-api default would hit `/responses`, which the
                // gateway does not serve.
                let client = openai::Client::builder()
                    .api_key(api_key.clone())
                    .base_url(base_url.clone())
                    .build()
                    .context("netmind client build failed")?
                    .completions_api();
                Ok(Clients::Netmind(client))
            },
        }
    }

    /// Provider-specific model parameters for the agent builders. Ollama
    /// carries the context window and the think switch; the OpenAI
    ///-compatible wire has no counterpart, so netmind sends none.
    pub fn extra_params(&self, run: &Run) -> Option<serde_json::Value> {
        match self {
            Self::Ollama {
                ..
            } => Some(serde_json::json!({
                "num_ctx": run.num_ctx,
                "think": run.think,
            })),
            Self::Netmind {
                ..
            } => None,
        }
    }
}

/// The concrete client of one provider. The harness matches on this and
/// calls the monomorphic run path for the variant.
pub enum Clients {
    /// Native ollama client.
    Ollama(ollama::Client),
    /// OpenAI chat-completions client over the netmind gateway.
    Netmind(openai::CompletionsClient),
}
