//! Model offload for the ollama daemon. The survey switches models
//! between runs; a loaded model pins its VRAM until its keep-alive
//! expires, and two large models cannot be resident at once. One
//! `/api/generate` request with `keep_alive: 0` unloads a model
//! immediately. Verified against the local daemon: a loaded model
//! disappears from `ollama ps` right after the request.

use anyhow::{Context, Result};
use std::time::Duration;

/// Unload every ollama model, then confirm the daemon holds nothing.
/// `base_url` comes from the provider variant at the call site.
/// `loading` names a model allowed to stay resident: before a run it is
/// the model about to run, which ollama may already be preloading; after
/// a run it is `None`, so the wait demands a truly empty daemon.
pub async fn unload_all(base_url: Option<&str>, loading: Option<&str>) -> Result<()> {
    let base = base_url.unwrap_or("http://localhost:11434").trim_end_matches('/').to_owned();
    for model in loaded_models(&base).await? {
        tracing::info!(model = %model, "offloading model");
        unload_one(&base, &model).await?;
    }
    wait_until_empty(&base, loading).await
}

/// Unload one model through the generate endpoint with a zero keep-alive.
async fn unload_one(base: &str, model: &str) -> Result<()> {
    let url = format!("{base}/api/generate");
    let body = serde_json::json!({ "model": model, "keep_alive": 0 });
    let response =
        reqwest::Client::new().post(&url).json(&body).send().await.context("ollama unload request failed")?;
    if !response.status().is_success() {
        anyhow::bail!("ollama unload failed for {model}: HTTP {}", response.status());
    }
    Ok(())
}

/// The resident model names from one `/api/ps` response. The daemon
/// wraps the array in an object: `{"models": [...]}`.
async fn loaded_models(base: &str) -> Result<Vec<String>> {
    let url = format!("{base}/api/ps");
    let response = reqwest::get(&url).await.context("ollama ps request failed")?;
    let text = response.text().await.context("ollama ps body read failed")?;
    let names = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| value.get("models").and_then(|models| models.as_array()).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| entry.get("name").and_then(|name| name.as_str()).map(str::to_owned))
        .collect();
    Ok(names)
}

/// Poll `/api/ps` until the daemon reports no resident model outside the
/// `loading` exception. A fresh load of the next model must not race the
/// unload of the last one.
async fn wait_until_empty(base: &str, loading: Option<&str>) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let resident = loaded_models(base).await?;
        let blocking: Vec<&String> = resident.iter().filter(|name| Some(name.as_str()) != loading).collect();
        if blocking.is_empty() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "ollama models still resident after 30s: {}",
                blocking.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ")
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
