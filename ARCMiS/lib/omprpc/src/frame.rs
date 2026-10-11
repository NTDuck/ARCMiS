//! `frame` decodes omp RPC stdout lines into typed [`Event`]s.

use crate::frame_payloads::{
    ExtensionUiRequestFrame, PromptResultFrame, ReadyFrame, ResponseFrame, SessionEvent, SubagentEventFrame,
    SubagentLifecycleFrame, SubagentProgressFrame,
};
use serde::Deserialize;

/// One decoded server frame. Unknown shapes stay typed as [`Event::Other`].
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Ready(ReadyFrame),
    Response(ResponseFrame),
    PromptResult(PromptResultFrame),
    SubagentLifecycle(SubagentLifecycleFrame),
    SubagentProgress(SubagentProgressFrame),
    SubagentEvent(SubagentEventFrame),
    ExtensionUiRequest(ExtensionUiRequestFrame),
    #[serde(untagged)]
    Other(SessionEvent),
}

/// Physical stdout line classification.
#[derive(Debug)]
pub enum RawLine {
    Event(Box<Event>),
    Skipped(String),
}

/// Decode one stdout line. Non-JSON banner text returns `Skipped`.
///
/// WHY banner skipping: omp prints non-JSON startup lines such as
/// `[pi-reasonix] Loaded. ...` to stdout before the first frame. The child
/// and the host share one stdout pipe, so the host must tolerate them
/// instead of failing the first read.
pub fn decode_line(line: &str) -> RawLine {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return RawLine::Skipped(String::new());
    }
    let value = match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => value,
        // Banner text is not JSON at all. Only such lines get skipped.
        Err(_) => return RawLine::Skipped(trimmed.to_string()),
    };
    let typed = serde_json::from_value::<Event>(value.clone());
    match typed {
        Ok(event) => RawLine::Event(Box::new(event)),
        Err(error) => {
            // WHY a catch-all: omp adds frame types between releases and the
            // serde enum tag match would reject them. Dropping them silently
            // hangs wait loops (verified in a prior integration), so unknown
            // frames stay reachable as the raw payload and log at debug.
            tracing::debug!(error = %error, line = %trimmed, "kept untyped frame");
            let event = serde_json::from_value::<SessionEvent>(value).unwrap_or_else(|_| SessionEvent::default());
            RawLine::Event(Box::new(Event::Other(event)))
        },
    }
}
