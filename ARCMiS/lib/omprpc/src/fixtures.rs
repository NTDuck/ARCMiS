//! Real wire fixtures for the tests. Every line comes from the live
//! census probe against omp 18.6.1 (`--mode rpc --approval-mode=yolo`).
//! Subagent payload shapes come from the omp-subagent-frame-routing doc.

// --- Real wire fixtures (census, omp 18.6.1) ------------------------------

pub(crate) const BANNER: &str = "[pi-reasonix] Loaded. cache=on cost=on metrics=on";

pub(crate) const READY: &str = r#"{"maxFrameBytes": 1048576, "maxReassembledFrameBytes": 67108864, "protocolVersion": 1, "supportedProtocolVersions": [1, 2], "type": "ready"}"#;

pub(crate) const RESPONSE_NEGOTIATE: &str = r#"{"command": "negotiate_protocol", "data": {"protocolVersion": 2}, "id": "1", "success": true, "type": "response"}"#;

pub(crate) const RESPONSE_SUBSCRIPTION: &str = r#"{"command": "set_subagent_subscription", "data": {"level": "events"}, "id": "2", "success": true, "type": "response"}"#;

pub(crate) const RESPONSE_PROMPT: &str = r#"{"command": "prompt", "id": "3", "success": true, "type": "response"}"#;

pub(crate) const AGENT_START: &str = r#"{"type": "agent_start"}"#;

pub(crate) const MESSAGE_START: &str = r#"{"message": {"attribution": "user", "content": [{"text": "Reply with exactly: OK", "type": "text"}], "role": "user", "timestamp": 1791655290660}, "messageId": "msg-1", "type": "message_start"}"#;

pub(crate) const MESSAGE_END_ASSISTANT: &str = r#"{"message": {"api": "openai-completions", "content": [{"text": "OK", "type": "text"}], "duration": 798.7, "model": "zai-org/GLM-5.3-Flash", "provider": "codev", "responseId": "chatcmpl-832887caca6a015f", "role": "assistant", "stopReason": "stop", "timestamp": 1791655292070, "ttft": 797.5, "usage": {"cacheRead": 0, "cacheWrite": 0, "cost": {"cacheRead": 0, "cacheWrite": 0, "input": 0.0015576, "output": 1.5e-06, "total": 0.0015591}, "input": 10384, "output": 3, "totalTokens": 10387}}, "messageId": "msg-2", "type": "message_end"}"#;

pub(crate) const TURN_END: &str = r#"{"message": {"api": "openai-completions", "content": [{"text": "OK", "type": "text"}], "duration": 798.7, "model": "zai-org/GLM-5.3-Flash", "provider": "codev", "responseId": "chatcmpl-832887caca6a015f", "role": "assistant", "stopReason": "stop", "timestamp": 1791655292070, "ttft": 797.5, "usage": {"cacheRead": 0, "cacheWrite": 0, "input": 10384, "output": 3, "totalTokens": 10387}}, "toolResults": [], "type": "turn_end"}"#;

pub(crate) const AGENT_END: &str = r#"{"isTerminal": true, "messages": [{"attribution": "user", "content": [{"text": "Reply with exactly: OK", "type": "text"}], "role": "user", "timestamp": 1791655290660}, {"api": "openai-completions", "content": [{"text": "OK", "type": "text"}], "role": "assistant", "stopReason": "stop", "timestamp": 1791655292070}], "type": "agent_end", "yielded": true}"#;

pub(crate) const PROMPT_RESULT: &str =
    r#"{"agentInvoked": true, "id": "3", "sessionSettled": true, "status": "completed", "type": "prompt_result"}"#;

pub(crate) const EXTENSION_UI_REQUEST: &str =
    r#"{"id": "15a10bdda6d0ebb3", "method": "setWidget", "type": "extension_ui_request", "widgetKey": "autoresearch"}"#;

pub(crate) const ADVISOR_COST_CHANGED: &str = r#"{"type": "advisor_cost_changed"}"#;

// Subagent payload fixtures from the skill doc (omp 18.6.1 binary, src/
// modes/rpc/rpc-subagents.ts). Shapes verified there, not in this census.
pub(crate) const SUBAGENT_LIFECYCLE: &str = r#"{"type": "subagent_lifecycle", "payload": {"id": "FranticSole", "agent": "recodeagent-analyzer", "parentToolCallId": "tool-1", "detached": false, "agentSource": "project", "description": "Analyze the diff", "status": "running", "sessionFile": "/tmp/s.jsonl", "index": 0}}"#;

pub(crate) const SUBAGENT_PROGRESS: &str = r#"{"type": "subagent_progress", "id": "FranticSole", "agent": "recodeagent-analyzer", "index": 0, "task": "Analyze", "parentToolCallId": "tool-1", "detached": false, "assignment": "diff", "progress": {"status": "running", "lastIntent": "reading src", "currentTool": "read", "currentToolIntent": "inspect", "recentOutput": ["line one", "line two"], "tokens": 1200, "cost": 0.01, "durationMs": 4200, "contextTokens": 12000, "contextWindow": 128000, "toolCount": 3}, "sessionFile": "/tmp/s.jsonl"}"#;

pub(crate) const SUBAGENT_EVENT: &str = r#"{"type": "subagent_event", "payload": {"id": "FranticSole", "event": {"type": "message_update", "message": {"role": "assistant", "content": [{"type": "text", "text": "partial"}]}}}}"#;
