#!/usr/bin/env python3
"""Extract ReCodeAgent tool-call traces into QLoRA chat datasets.

Source: the ReCodeAgent paper's published experiment artifacts (Zenodo DOI
10.5281/zenodo.21842351, `results.zip`). The zip ships Claude Code session
JSONL under `results/trajectories/`:

    results/trajectories/{agent}.{tool}.{project}.{src}.{dst}/-workspace/{uuid}.jsonl
    results/trajectories/.../-workspace/{uuid}/subagents/agent-{id}.jsonl

Each line is a Claude Code session record:
{"type": "user"|"assistant"|"progress"|..., "message": {"role": ..., 
"content": <string | [blocks]>}, ...} with content blocks of type text,
tool_use, and tool_result. This script converts every session into one chat
transcript and emits two views:

- `messages.jsonl`: {"id", "agent", "project", "messages": [...]} with
  roles system/user/assistant/tool, ready for chat-template packing.
- `sft.jsonl`: {"id", "agent", "instruction", "output"} flattened pairs.

Tool calls become `[TOOL_CALL name] {args}` text inside the assistant turn;
tool results become `[TOOL_RESULT id] text` user turns, so any chat
template can render the data without provider-specific tool roles.

Default extraction covers the canonical `recodeagent.*` trajectories; pass
--agent to widen (recodeagent, noanalyzer, noplanning, novalidator,
baseagent-concat, baseagent-condensed).

Usage:
    python3 qlora/scripts/extract_traces.py \
        --results-dir assets/ReCodeAgent/data/results.zip \
        --out-dir qlora/datasets/recodeagent
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import zipfile
from collections import Counter
from pathlib import Path

# Session file path: any jsonl under results/trajectories/.
SESSION_RE = re.compile(r"^results/trajectories/.+?\.jsonl$")


def blocks_to_parts(content) -> tuple[str, list[dict], list[dict]]:
    """Split one message content into (text, tool_calls, tool_results)."""
    if isinstance(content, str):
        return content, [], []
    texts, calls, results = [], [], []
    for block in content or []:
        btype = block.get("type")
        if btype == "text":
            texts.append(block.get("text", ""))
        elif btype == "tool_use":
            calls.append(
                {
                    "id": block.get("id"),
                    "name": block.get("name"),
                    "arguments": block.get("input", {}),
                }
            )
        elif btype == "tool_result":
            inner = block.get("content")
            if isinstance(inner, list):
                inner = "\n".join(
                    part.get("text", "") for part in inner if isinstance(part, dict) and part.get("type") == "text"
                )
            results.append(
                {
                    "id": block.get("tool_use_id"),
                    "content": inner if isinstance(inner, str) else "" if inner is None else str(inner),
                }
            )
    return "\n".join(t for t in texts if t), calls, results


def session_to_messages(path: str, lines: list[str]) -> dict | None:
    """Convert one session jsonl into a chat transcript record."""
    messages: list[dict] = []
    for line in lines:
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = row.get("type")
        if kind not in ("user", "assistant"):
            continue
        message = row.get("message") or {}
        content = message.get("content")
        if content is None:
            continue
        text, calls, results = blocks_to_parts(content)
        if kind == "assistant":
            for call in calls:
                text += f"\n[TOOL_CALL {call['name']}] " + json.dumps(
                    call["arguments"], ensure_ascii=False
                )
            text = text.strip()
            if not text:
                continue
            if messages and messages[-1]["role"] == "assistant":
                messages[-1]["content"] += "\n" + text
            else:
                messages.append({"role": "assistant", "content": text})
        else:
            for result in results:
                entry = f"[TOOL_RESULT {result['id']}] {result['content']}".strip()
                if messages and messages[-1]["role"] == "tool":
                    messages[-1]["content"] += "\n" + entry
                else:
                    messages.append({"role": "tool", "content": entry})
            if not results and text.strip():
                messages.append({"role": "user", "content": text.strip()})
    # Collapse consecutive tool turns into one (chat templates prefer single
    # tool-result blocks).
    collapsed: list[dict] = []
    for message in messages:
        if message["role"] == "tool" and collapsed and collapsed[-1]["role"] == "tool":
            collapsed[-1]["content"] += "\n" + message["content"]
        else:
            collapsed.append(message)
    if len(collapsed) < 2:
        return None
    run = run_of(path)
    base = {
        "id": hashlib.sha256(path.encode()).hexdigest()[:16],
        "agent": run.split(".")[0] if run else "unknown",
        "project": ".".join(run.split(".")[1:]) if "." in run else "unknown",
    }
    # The first user/system instruction: Claude CLI -p passes the prompt as
    # the first user message; it is text-only.
    return {**base, "messages": collapsed}


def run_of(rel: str) -> str:
    """Extract the `{agent}.{tool}.{project}.{src}.{dst}` run dir name from a
    trajectory-relative path (works with and without the results/ prefix)."""
    parts = rel.split("/")
    if parts and parts[0] == "results":
        parts = parts[1:]
    return parts[1] if len(parts) > 2 else ""


def iter_sessions(results: Path, agents: set[str]) -> tuple[str, list[str]]:
    """Yield (path, lines) for every session jsonl under trajectories/."""
    if results.is_dir():
        root = results / "trajectories"
        for path in sorted(root.rglob("*.jsonl")):
            rel = path.relative_to(results).as_posix()
            agent = run_of(rel).split(".")[0]
            if agents and agent not in agents:
                continue
            yield rel, path.read_text(errors="replace").splitlines()
    else:
        with zipfile.ZipFile(results) as zf:
            for info in zf.infolist():
                rel = info.filename
                if not SESSION_RE.match(rel):
                    continue
                agent = run_of(rel).split(".")[0]
                if agents and agent not in agents:
                    continue
                try:
                    text = zf.read(info).decode("utf-8", errors="replace")
                except Exception:
                    continue
                yield rel, text.splitlines()


def sft_pair(record: dict) -> dict | None:
    """Flatten one transcript into (instruction, output).

    Instruction: the opening user prompt plus tool traffic before the final
    assistant text. Output: the final nonempty assistant text.
    """
    final = None
    for message in reversed(record["messages"]):
        if message["role"] == "assistant" and message.get("content"):
            final = message["content"]
            break
    if not final:
        return None
    parts = [
        message["content"]
        for message in record["messages"][:-1]
        if message["role"] in ("user", "tool") and message.get("content")
    ]
    instruction = "\n\n".join(parts)
    if not instruction:
        return None
    return {
        "id": record["id"],
        "agent": record["agent"],
        "instruction": instruction[-240_000:],
        "output": final,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--results-dir",
        required=True,
        type=Path,
        help="results.zip or unpacked results/ directory",
    )
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument(
        "--agent",
        action="append",
        default=None,
        help="restrict to these trajectory agent variants (repeatable)",
    )
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    agents = {a for a in (args.agent or ["recodeagent"])}
    stats: Counter = Counter()
    messages_path = args.out_dir / "messages.jsonl"
    sft_path = args.out_dir / "sft.jsonl"

    with messages_path.open("w") as messages_out, sft_path.open("w") as sft_out:
        for path, lines in iter_sessions(args.results_dir, agents):
            stats["sessions_seen"] += 1
            record = session_to_messages(path, lines)
            if record is None:
                stats["sessions_dropped_short"] += 1
                continue
            messages_out.write(json.dumps(record) + "\n")
            stats["conversations_emitted"] += 1
            pair = sft_pair(record)
            if pair:
                sft_out.write(json.dumps(pair) + "\n")
                stats["sft_emitted"] += 1
            else:
                stats["sft_dropped_no_pair"] += 1

    print(json.dumps(dict(stats), indent=2))
    print(f"messages: {messages_path}")
    print(f"sft:      {sft_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
