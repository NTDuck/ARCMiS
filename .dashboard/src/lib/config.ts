// Poll interval in milliseconds. Single knob, imported everywhere.
export const POLL_MS = 10_000;

export const PROBE_TIMEOUT_MS = 3_000;

// Heartbeat older than this is stale.
export const HEARTBEAT_STALE_MS = 120_000;

// A round dir with events.jsonl touched within this window is live.
export const LIVE_ROUND_WINDOW_MS = 15 * 60 * 1000;
