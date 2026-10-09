#!/usr/bin/env python3
"""Demo `messaging`: two omp subagents message each other over one RPC host.

The orchestrator spawns two task agents. Alpha broadcasts `PING from alpha`
over agent://all. Beta waits for that message, then broadcasts `PONG from
beta: received PING from alpha`. Alpha waits again and quotes Beta's PONG.
The demo prints the transcript and exits 0 when the round trip closes.

Usage: python3 scripts/demo-messaging.py [--host-cwd PATH] [--timeout SECS]
"""

import argparse
import json
import select
import subprocess
import sys
import time

PING_BODY = "PING from alpha"
PONG_BODY = "PONG from beta: received PING from alpha"


def spawn(host_cwd: str) -> subprocess.Popen:
    """Start one omp RPC child with yolo approvals."""
    return subprocess.Popen(
        ["omp", "--mode", "rpc", "--approval-mode=yolo"],
        cwd=host_cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )


def send(p: subprocess.Popen, payload: dict) -> None:
    """Write one JSONL frame and flush."""
    assert p.stdin is not None
    p.stdin.write((json.dumps(payload) + "\n").encode())
    p.stdin.flush()


def pump(p: subprocess.Popen, seconds: int, handler) -> None:
    """Read stdout frames until the deadline, feeding each to the handler."""
    assert p.stdout is not None
    end = time.time() + seconds
    buffer = b""
    while time.time() < end:
        ready, _, _ = select.select([p.stdout], [], [], 1)
        if not ready:
            continue
        chunk = p.stdout.read1(65536)
        if not chunk:
            break
        buffer += chunk
        while b"\n" in buffer:
            line, buffer = buffer.split(b"\n", 1)
            text = line.decode(errors="ignore").strip()
            if not text:
                continue
            try:
                handler(json.loads(text))
            except json.JSONDecodeError:
                continue


def run_demo(host_cwd: str, timeout: int) -> int:
    """Drive the two-agent exchange. Returns the exit code."""
    p = spawn(host_cwd)
    seed = (
        "Do exactly this:\n"
        '- task tool, agent "task", name "Alpha": your ONLY job is to send ONE '
        'message and finish. Use write to send "PING from alpha" to agent://all. '
        'Then report "alpha sent PING" and finish immediately.\n'
        '- Then task tool, agent "task", name "Beta": your ONLY job is to receive '
        "Alpha's message and reply. First wait for an incoming IRC message "
        'containing "PING from alpha". Then use write to send '
        f'"{PONG_BODY}" to agent://all. Report "beta sent PONG" and finish immediately.'
    )
    irc_seen = {"alpha": False, "beta": False}
    final_text = ""

    def handle(frame: dict) -> None:
        nonlocal final_text
        ftype = frame.get("type", "")
        if ftype == "ready":
            send(p, {"id": "1", "type": "negotiate_protocol", "protocolVersion": 2})
        elif ftype == "response" and frame.get("id") == "1":
            send(p, {"id": "2", "type": "set_subagent_subscription", "level": "events"})
        elif ftype == "response" and frame.get("id") == "2":
            send(p, {"id": "3", "type": "prompt", "message": seed})
        elif ftype == "subagent_lifecycle":
            payload = frame.get("payload", {})
            print(f"[lifecycle] {payload.get('id')} {payload.get('status')}", flush=True)
        elif ftype == "irc_message":
            content = (frame.get("message", {}) or {}).get("content", "")
            body = content.replace("\n", " ")
            print(f"[irc] {body[:170]}", flush=True)
            if PONG_BODY in content:
                irc_seen["beta"] = True
            elif PING_BODY in content:
                irc_seen["alpha"] = True
        elif ftype == "tool_execution_update" and frame.get("toolName") == "write":
            args = frame.get("args") or {}
            body = str(args.get("content", ""))
            if body == PONG_BODY:
                irc_seen["beta"] = True
            elif body == PING_BODY:
                irc_seen["alpha"] = True
        elif ftype == "subagent_event":
            # Peer-to-peer writes surface here, nested in the subagent's
            # own event stream, not as host irc_message frames.
            event = (frame.get("payload") or {}).get("event") or {}
            if event.get("type") == "tool_execution_update" and event.get("toolName") == "write":
                args = event.get("args") or {}
                body = str(args.get("content", ""))
                if body == PONG_BODY:
                    irc_seen["beta"] = True
                    print(f"[pong] {PONG_BODY}", flush=True)
                elif body == PING_BODY:
                    irc_seen["alpha"] = True
        elif ftype == "message_end":
            message = frame.get("message") or {}
            content = message.get("content")
            if isinstance(content, list):
                for part in content:
                    if isinstance(part, dict) and part.get("type") == "text":
                        final_text = part.get("text", "")
        elif ftype == "prompt_result":
            print(f"[prompt_result] status={frame.get('status')}", flush=True)
            raise SystemExit

    try:
        pump(p, timeout, handle)
    except SystemExit:
        pass
    finally:
        p.kill()

    print(
        f"--- alpha_ping={irc_seen['alpha']} beta_pong={irc_seen['beta']} "
        f"final_report={'PONG from beta' in final_text}"
    )
    ok = irc_seen["alpha"] and irc_seen["beta"]
    print("DEMO PASS" if ok else "DEMO FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host-cwd", default=".", help="omp working directory")
    parser.add_argument("--timeout", type=int, default=420, help="seconds before giving up")
    args = parser.parse_args()
    return run_demo(args.host_cwd, args.timeout)


if __name__ == "__main__":
    sys.exit(main())
