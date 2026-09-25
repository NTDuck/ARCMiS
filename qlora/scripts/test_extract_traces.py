#!/usr/bin/env python3
"""Throwaway validation for extract_traces.py against a synthetic session.

Builds a minimal results/trajectories tree in Claude Code session-jsonl
format (user/assistant rows, tool_use/tool_result blocks, two tool rounds),
runs the extractor over a zipped and an unpacked tree, and asserts rows.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

HERE = Path(__file__).parent


def session_lines() -> str:
    rows = [
        {
            "type": "user",
            "message": {
                "role": "user",
                "content": "Translate the colorsys module from Python to JavaScript.",
            },
        },
        {
            "type": "assistant",
            "message": {
                "role": "assistant",
                "model": "claude-sonnet-4-5",
                "content": [{"type": "text", "text": "Reading the module."}],
            },
        },
        {
            "type": "assistant",
            "message": {
                "role": "assistant",
                "model": "claude-sonnet-4-5",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "tu1",
                        "name": "Bash",
                        "input": {"command": "cat source.py"},
                    }
                ],
            },
        },
        {
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "tu1",
                        "content": "def rgb_to_hsv(): ...",
                    }
                ],
            },
        },
        {
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "tu2",
                        "content": "tests pass",
                    }
                ],
            },
        },
        {
            "type": "assistant",
            "message": {
                "role": "assistant",
                "model": "claude-sonnet-4-5",
                "content": [
                    {"type": "text", "text": "Translation complete. All checks pass."}
                ],
            },
        },
    ]
    return "".join(json.dumps(row) + "\n" for row in rows)


def make_tree(tmp: Path) -> Path:
    run = "recodeagent.skel.colorsys.python.javascript"
    session = Path(tmp) / "results" / "trajectories" / run / "-workspace" / "abc.jsonl"
    session.parent.mkdir(parents=True)
    session.write_text(session_lines())
    return session


def run_extract(results: Path, out: Path) -> None:
    subprocess.run(
        [
            sys.executable,
            str(HERE / "extract_traces.py"),
            "--results-dir",
            str(results),
            "--out-dir",
            str(out),
        ],
        check=True,
        capture_output=True,
    )


def check(out: Path, source: str) -> None:
    messages = [json.loads(l) for l in (out / "messages.jsonl").read_text().splitlines()]
    sft = [json.loads(l) for l in (out / "sft.jsonl").read_text().splitlines()]

    assert len(messages) == 1, f"{source}: expected 1 transcript, got {len(messages)}"
    record = messages[0]
    assert record["agent"] == "recodeagent", record["agent"]
    assert record["project"].startswith("skel.colorsys"), record["project"]
    roles = [m["role"] for m in record["messages"]]
    assert roles == ["user", "assistant", "tool", "assistant"], roles
    assert "[TOOL_CALL Bash]" in record["messages"][1]["content"]
    assert "[TOOL_RESULT tu1]" in record["messages"][2]["content"]
    # The consecutive tool results collapsed into one tool turn.
    assert "tests pass" in record["messages"][2]["content"]

    assert len(sft) == 1, f"{source}: expected 1 sft pair, got {len(sft)}"
    assert sft[0]["output"] == "Translation complete. All checks pass."
    assert "Translate the colorsys module" in sft[0]["instruction"]
    assert "[TOOL_RESULT tu1]" in sft[0]["instruction"]
    print(f"{source} OK")


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_tree(tmp)

        # 1. Unpacked directory source.
        out = Path(tmp) / "out-dir"
        run_extract(tmp / "results", out)
        check(out, "dir")

        # 2. Zipped source (the shipped artifact form).
        zip_path = tmp / "results.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            for path in (tmp / "results").rglob("*"):
                if path.is_file():
                    zf.write(path, path.relative_to(tmp).as_posix())
        out2 = Path(tmp) / "out-zip"
        run_extract(zip_path, out2)
        check(out2, "zip")

    print("extract_traces validation OK")


if __name__ == "__main__":
    sys.exit(main())
