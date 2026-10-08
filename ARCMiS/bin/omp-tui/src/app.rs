//! `app` runs the select loop over RPC frames, keys, and redraw ticks.

use crate::config::RunConfig;
use crate::event::apply_frame;
use crate::input::{handle_event, InputAction};
use crate::store::Store;
use crate::ui::draw;
use crossterm::event::EventStream;
use futures::StreamExt;
use omprpc::client::Client;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

/// Terminal session over one RPC client.
pub struct App {
    store: Store,
    client: Client,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl App {
    /// Wire the store, client, and terminal together.
    pub fn new(store: Store, client: Client, terminal: Terminal<CrosstermBackend<Stdout>>) -> Self {
        Self {
            store,
            client,
            terminal,
        }
    }

    /// Run until quit. Every frame folds into the store before a redraw.
    pub async fn run(mut self, config: &RunConfig) -> anyhow::Result<()> {
        let Self {
            store,
            client,
            terminal,
        } = &mut self;
        let mut events = client.transport.take_events();
        let rpc_events = futures::stream::unfold(&mut events, |events| async move {
            let frame = events.recv().await?;
            Some((frame, events))
        });
        tokio::pin!(rpc_events);
        let mut keys = EventStream::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(120));
        if !config.seed_prompt.is_empty() {
            store.busy = true;
            client.request(serde_json::json!({ "type": "prompt", "message": config.seed_prompt })).await?;
        }
        loop {
            tokio::select! {
                maybe_frame = rpc_events.next() => {
                    let Some(frame) = maybe_frame else {
                        store.orchestrator.push_line("[host] child stream ended".to_string());
                        terminal.draw(|frame| draw(frame, store))?;
                        anyhow::bail!("omp child exited");
                    };
                    let redraw = apply_frame(store, &frame);
                    client.feed_frame(frame);
                    if redraw {
                        terminal.draw(|frame| draw(frame, store))?;
                    }
                }
                maybe_key = keys.next() => {
                    let Some(result) = maybe_key else { continue };
                    let event = result?;
                    match handle_event(store, &event) {
                        InputAction::None => {}
                        InputAction::Redraw => {
                            terminal.draw(|frame| draw(frame, store))?;
                        }
                        InputAction::Abort => {
                            store.orchestrator.push_line("[host] abort requested".to_string());
                            client
                                .request(serde_json::json!({ "type": "abort" }))
                                .await?;
                            store.busy = false;
                        }
                        InputAction::Quit => break,
                        InputAction::Submit { message, streaming } => {
                            store.busy = true;
                            let mut params =
                                serde_json::json!({ "type": "prompt", "message": message });
                            if streaming {
                                params["streamingBehavior"] = serde_json::json!("steer");
                            }
                            client.request(params).await?;
                        }
                    }
                    terminal.draw(|frame| draw(frame, store))?;
                }
                _ = ticker.tick() => {
                    terminal.draw(|frame| draw(frame, store))?;
                }
            }
        }
        terminal.draw(|frame| draw(frame, store))?;
        Ok(())
    }
}
