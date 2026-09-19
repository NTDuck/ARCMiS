//! Typed task plumbing shared by the agents. One helper owns the
//! serialize, prompt, and parse sequence. The agents stay thin.

use anyhow::Context as _;
use rig::agent::Agent;
use rig::completion::Prompt;

/// Run one agent over one typed task. The task serializes into the prompt
/// payload. The structured answer parses into `Response` through the agent's
/// output schema (`OutputMode::Tool`). `max_turns` bounds the model-call
/// budget.
pub async fn task<Response>(agent: &Agent, task: &impl serde::Serialize, max_turns: usize) -> anyhow::Result<Response>
where
    Response: serde::de::DeserializeOwned,
{
    let prompt = serde_json::to_string(task).context("task render failed")?;
    let raw = agent.prompt(prompt).max_turns(max_turns).await?;
    Ok(serde_json::from_str(&raw)?)
}
