#!/usr/bin/env python3
"""Build an axolotl-ready chat dataset from extracted ReCodeAgent traces.

Takes `messages.jsonl` from extract_traces.py and emits:

- `train.jsonl` / `val.jsonl` split 98/2 by session hash (deterministic).
- Dedup on normalized assistant output (identical boilerplate replies carry
  no gradient signal and bias the run).
- Length filter: drops conversations whose packed character count exceeds
  `--max-chars` (default 96k chars ~= 24k tokens for code-heavy text).

Output rows follow axolotl's `chat_template`-ready schema:

    {"messages": [{"role": "system"|"user"|"assistant", "content": ...}]}

Tool traffic is flattened into the user turn text with explicit
`[TOOL_CALL ...]` / `[TOOL_RESULT ...]` markers so any chat template can
represent it without provider-specific tool roles.

Usage:
    python3 qlora/scripts/build_dataset.py \
        --messages qlora/datasets/recodeagent/messages.jsonl \
        --out-dir qlora/datasets/recodeagent
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

SYSTEM_PROMPT = (
    "You are ReCodeAgent, a coding agent that translates and validates "
    "repositories across programming languages. Use the provided tools to "
    "read files, write translations, and run validation commands. Reason "
    "before acting, keep edits inside the workspace, and finish with the "
    "requested summary line."
)


def flatten_tool_traffic(messages: list[dict]) -> list[dict]:
    """Rewrite tool roles into user-turn text with explicit markers."""
    out: list[dict] = []
    for message in messages:
        role = message["role"]
        if role == "user":
            out.append({"role": "user", "content": message.get("content", "")})
        elif role == "assistant":
            content = message.get("content", "")
            for call in message.get("tool_calls", []) or []:
                content += (
                    f"\n[TOOL_CALL {call.get('name')}] "
                    + json.dumps(call.get("arguments", {}), ensure_ascii=False)
                )
            out.append({"role": "assistant", "content": content.strip()})
        elif role == "tool":
            parts = [
                f"[TOOL_RESULT {res.get('tool_call_id', '')}] {res.get('content', '')}"
                for res in message.get("tool_results", []) or []
            ]
            text = "\n".join(parts)
            if out and out[-1]["role"] == "user":
                out[-1]["content"] += "\n" + text
            else:
                out.append({"role": "user", "content": text})
    return [m for m in out if m["content"].strip()]


def row_chars(row: dict) -> int:
    return sum(len(m["content"]) for m in row["messages"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--messages", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--max-chars", type=int, default=96_000)
    parser.add_argument("--val-fraction", type=float, default=0.02)
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    stats: Counter = Counter()
    seen_outputs: set[str] = set()
    rows: list[dict] = []

    for line in args.messages.read_text().splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        stats["read"] += 1
        flat = flatten_tool_traffic(record["messages"])
        # Claude CLI -p transcripts open with the assistant's first message;
        # the instruction text lives in tool traffic and system context.
        if not flat or not any(m["role"] == "user" for m in flat):
            stats["dropped_no_user"] += 1
            continue
        final = next(
            (m["content"] for m in reversed(flat) if m["role"] == "assistant" and m["content"]),
            None,
        )
        if not final:
            stats["dropped_no_assistant"] += 1
            continue
        key = hashlib.sha256(" ".join(final.split()).encode()).hexdigest()
        if key in seen_outputs:
            stats["dropped_duplicate"] += 1
            continue
        seen_outputs.add(key)
        row = {"id": record["id"], "messages": [{"role": "system", "content": SYSTEM_PROMPT}, *flat]}
        if row_chars(row) > args.max_chars:
            stats["dropped_too_long"] += 1
            continue
        rows.append(row)
        stats["kept"] += 1

    # Deterministic session-hash split: same session never straddles splits.
    train, val = [], []
    for row in rows:
        bucket = int(hashlib.sha256(row["id"].encode()).hexdigest()[:16], 16) / 2**64
        (val if bucket < args.val_fraction else train).append(row)

    train_path = args.out_dir / "train.jsonl"
    val_path = args.out_dir / "val.jsonl"
    train_path.write_text("".join(json.dumps(r) + "\n" for r in train))
    val_path.write_text("".join(json.dumps(r) + "\n" for r in val))

    stats["train"] = len(train)
    stats["val"] = len(val)
    chars = sorted(row_chars(r) for r in rows)
    if chars:
        stats["chars_p50"] = chars[len(chars) // 2]
        stats["chars_p95"] = chars[int(len(chars) * 0.95)]
    print(json.dumps(dict(stats), indent=2))
    print(f"train: {train_path}")
    print(f"val:   {val_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
