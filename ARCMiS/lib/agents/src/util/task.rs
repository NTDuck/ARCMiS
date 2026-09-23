//! Typed task plumbing shared by the agents. One helper owns the
//! serialize, prompt, retry, and parse sequence. The agents stay thin.

use anyhow::Context as _;
use rig::agent::Agent;
use rig::completion::Prompt;

/// Run one agent over one typed task. The task serializes into the prompt
/// payload. The structured answer parses into `Response` through the agent's
/// output schema (`OutputMode::Tool`). `max_turns` bounds the model-call
/// budget. `max_retries` bounds whole-task retries after a failed attempt:
/// a transient provider failure costs one attempt, never the whole run.
pub async fn task<Response>(
    agent: &Agent,
    task: &impl serde::Serialize,
    max_turns: usize,
    max_retries: u32,
) -> anyhow::Result<Response>
where
    Response: serde::de::DeserializeOwned,
{
    let prompt = serde_json::to_string(task).context("task render failed")?;
    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            tracing::warn!(attempt, "task retry after failed attempt");
        }
        match agent.prompt(&prompt).max_turns(max_turns).max_invalid_tool_call_retries(2).await {
            Ok(raw) => {
                return serde_json::from_str(&raw).context("task response parse failed");
            },
            Err(error) => last_error = Some(error),
        }
    }
    Err(anyhow::anyhow!(last_error.expect("retry loop ran at least once")).context("task failed after retries"))
}
