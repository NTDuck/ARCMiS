//! `frame` decodes omp RPC wire frames from one stdout stream.
//!
//! The transport is newline-delimited JSON. Physical lines either carry one

//! reassembles v2 chunks in index order and skips banner lines that the
//! child prints to stdout.

use serde::Deserialize;

pub use crate::frame_payloads::*;

/// One decoded stdout frame. Unknown fields stay available through `extra`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    Ready(ReadyFrame),
    RpcChunk(RpcChunkFrame),
    Response(ResponseFrame),
    AgentStart(SessionEvent),
    AgentEnd(SessionEvent),
    TurnStart(SessionEvent),
    TurnEnd(SessionEvent),
    MessageStart(SessionEvent),
    MessageUpdate(SessionEvent),
    MessageEnd(SessionEvent),
    ToolExecutionStart(ToolExecutionStartFrame),
    ToolExecutionUpdate(SessionEvent),
    ToolExecutionEnd(SessionEvent),
    SubagentLifecycle(SubagentLifecycleFrame),
    SubagentProgress(SubagentProgressFrame),
    SubagentEvent(SubagentEventFrame),
    IrcMessage(SessionEvent),
    Notice(SessionEvent),
    QueueUpdate(SessionEvent),
    SessionSettled(SessionEvent),
    PromptResult(PromptResultFrame),
    ExtensionUiRequest(ExtensionUiRequestFrame),
    AvailableCommandsUpdate(SessionEvent),
    AdvisorYielded(SessionEvent),
}

/// Physical line classification after decode.
#[derive(Debug)]
pub enum DecodedLine {
    Frame(Box<ServerFrame>),
    ChunkPart,
    Skipped(String),
}

/// Chunk reassembly state. One interleaved sequence is active at a time.
#[derive(Debug, Default)]
pub struct Reassembler {
    pending: Option<PendingChunk>,
}

#[derive(Debug)]
struct PendingChunk {
    chunk_id: String,
    count: u32,
    parts: Vec<Option<String>>,
}

impl Reassembler {
    /// Feed one JSON object. Returns a logical frame when reassembly finishes.
    pub fn ingest(&mut self, value: serde_json::Value) -> anyhow::Result<Option<ServerFrame>> {
        let Some(frame_type) = value.get("type").and_then(serde_json::Value::as_str) else {
            return Ok(Some(tolerant_frame(value)?));
        };
        if frame_type != "rpc_chunk" {
            return Ok(Some(tolerant_frame(value)?));
        }
        let chunk: RpcChunkFrame =
            serde_json::from_value(value).map_err(|error| anyhow::anyhow!("bad rpc_chunk: {error}"))?;
        if chunk.count == 0 {
            anyhow::bail!("rpc_chunk count 0 for {}", chunk.chunk_id);
        }
        match &mut self.pending {
            Some(pending) if pending.chunk_id == chunk.chunk_id => {
                if chunk.index >= pending.count {
                    anyhow::bail!("chunk index {} beyond count {}", chunk.index, chunk.count);
                }
                pending.parts[chunk.index as usize] = Some(chunk.data);
            },
            Some(pending) => {
                anyhow::bail!("interleaved chunk {} while {} is open", chunk.chunk_id, pending.chunk_id);
            },
            None => {
                self.pending = Some(PendingChunk {
                    chunk_id: chunk.chunk_id,
                    count: chunk.count,
                    parts: (0..chunk.count).map(|_| None).collect(),
                });
                if let Some(pending) = self.pending.as_mut() {
                    pending.parts[chunk.index as usize] = Some(chunk.data);
                }
            },
        }
        let complete = self.pending.as_ref().is_some_and(|pending| pending.parts.iter().all(Option::is_some));
        if !complete {
            return Ok(None);
        }
        let pending = self.pending.take().expect("checked above");
        let joined =
            pending.parts.into_iter().map(|part| part.expect("checked complete")).collect::<Vec<String>>().join("");
        let value = serde_json::from_str::<serde_json::Value>(&joined)
            .map_err(|error| anyhow::anyhow!("reassembly of {} produced bad JSON: {error}", pending.chunk_id))?;
        Ok(Some(tolerant_frame(value)?))
    }
}

fn tolerant_frame(value: serde_json::Value) -> anyhow::Result<ServerFrame> {
    match serde_json::from_value::<ServerFrame>(value.clone()) {
        Ok(frame) => Ok(frame),
        Err(expected) => {
            let frame_type = value.get("type").and_then(serde_json::Value::as_str).map(str::to_string);
            if let Some(frame_type) = frame_type {
                tracing::debug!(frame_type = %frame_type, error = %expected, "kept untyped frame");
                serde_json::from_value::<SessionEvent>(value)
                    .map(ServerFrame::AdvisorYielded)
                    .map_err(|error| anyhow::anyhow!("untyped frame {frame_type}: {error}"))
            } else {
                Err(anyhow::anyhow!("frame without type: {expected}"))
            }
        },
    }
}

/// Decode one stdout line. Banner text returns `Skipped`.
pub fn decode_line(line: &str, reassembler: &mut Reassembler) -> anyhow::Result<DecodedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(DecodedLine::Skipped("empty".to_string()));
    }
    let value = match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => value,
        Err(_) => {
            tracing::debug!(line = %trimmed, "skipped non-JSON stdout line");
            return Ok(DecodedLine::Skipped(trimmed.to_string()));
        },
    };
    if value.get("type").and_then(serde_json::Value::as_str) == Some("rpc_chunk") {
        if reassembler.ingest(value)?.is_some() {
            anyhow::bail!("chunk completed without yielding a frame");
        }
        return Ok(DecodedLine::ChunkPart);
    }
    Ok(DecodedLine::Frame(Box::new(tolerant_frame(value)?)))
}
