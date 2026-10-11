#!/usr/bin/env python3
"""Watchdog for the llama-server spec-decode spin (c8 incident class).

Symptom: llama-server burns one core at ~100% CPU while GPU utilization
sits at 0% for minutes - a wedged speculative-decoding loop. Every probe
to ollama times out; the sweep harness hangs until the daemon restarts.

This monitor detects the signature and restarts ollama.service through
the pty sudo recipe (no passwordless sudo on this host; the password is
supplied interactively, never stored). One incident per log line under
.artifacts/experiments/watchdog.log.

Usage:
    python3 scripts/ollama-watchdog.py [--cpu 95] [--gpu 5] [--minutes 3] \
        [--interval 20] [--once]

Defaults: CPU >= 95%, GPU <= 5%, for >= 3 minutes, checked every 20 s.
--once runs a single check (for cron/timer wiring); the default loop
runs until interrupted.
"""

import argparse
import datetime
import os
import pty
import select
import subprocess
import sys
import time

LOG = ".artifacts/experiments/watchdog.log"
SERVICE = "ollama.service"


def log(message: str) -> None:
    stamp = datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds")
    line = f"{stamp} {message}\n"
    os.makedirs(os.path.dirname(LOG), exist_ok=True)
    with open(LOG, "a") as handle:
        handle.write(line)
    print(line, end="")


def llama_server_pids() -> list[int]:
    out = subprocess.run(
        ["pgrep", "-x", "llama-server"], capture_output=True, text=True
    )
    return [int(pid) for pid in out.stdout.split()]


def cpu_percent(pid: int) -> float:
    out = subprocess.run(
        ["ps", "-o", "pcpu=", "-p", str(pid)], capture_output=True, text=True
    )
    try:
        return float(out.stdout.strip())
    except ValueError:
        return 0.0


def gpu_percent() -> float:
    out = subprocess.run(
        [
            "nvidia-smi",
            "--query-gpu=utilization.gpu",
            "--format=csv,noheader,nounits",
        ],
        capture_output=True,
        text=True,
    )
    try:
        return max(float(v) for v in out.stdout.split())
    except ValueError:
        return 100.0  # unknown: do not trip on missing nvidia-smi


def daemon_dead() -> bool:
    probe = subprocess.run(
        ["curl", "-s", "-m", "5", "-o", "/dev/null", "-w", "%{http_code}",
         "http://localhost:11434/api/version"],
        capture_output=True,
        text=True,
    )
    return probe.stdout.strip() != "200"


def sudo_restart_ollama() -> bool:
    """Restart ollama.service via the pty sudo recipe (interactive prompt)."""
    script = f"sudo systemctl restart {SERVICE} && echo RESTART-OK"
    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("bash", ["bash", "-c", script])
    # Read until the child exits or the marker appears.
    deadline = time.monotonic() + 180
    collected = b""
    while time.monotonic() < deadline:
        ready, _, _ = select.select([fd], [], [], 5)
        if ready:
            try:
                chunk = os.read(fd, 4096)
            except OSError:
                break
            if not chunk:
                break
            collected += chunk
            if b"RESTART-OK" in collected:
                os.close(fd)
                _, status = os.waitpid(pid, 0)
                return status == 0
    try:
        os.close(fd)
    except OSError:
        pass
    return False


def check(args) -> bool:
    pids = llama_server_pids()
    if not pids:
        return False  # nothing served; nothing to wedge
    hot = any(cpu_percent(pid) >= args.cpu for pid in pids)
    gpu_idle = gpu_percent() <= args.gpu
    if not (hot and gpu_idle):
        return False
    # Confirm the daemon is actually wedged (not just batching) before
    # restarting: a healthy daemon answers version probes instantly.
    if not daemon_dead():
        return False
    log(
        f"SPIN detected: llama-server pid(s) {pids} CPU>={args.cpu}% "
        f"with GPU<={args.gpu}% and dead API - restarting {SERVICE}"
    )
    if sudo_restart_ollama():
        log("RESTART ok")
    else:
        log("RESTART FAILED - manual intervention needed")
    return True


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cpu", type=float, default=95.0)
    parser.add_argument("--gpu", type=float, default=5.0)
    parser.add_argument("--minutes", type=float, default=3.0)
    parser.add_argument("--interval", type=float, default=20.0)
    parser.add_argument("--once", action="store_true")
    args = parser.parse_args()

    # The spin must persist `minutes` before tripping; track consecutive hits.
    hot_since: float | None = None
    while True:
        pids = llama_server_pids()
        hot = bool(pids) and any(cpu_percent(p) >= args.cpu for p in pids)
        if hot and gpu_percent() <= args.gpu and daemon_dead():
            hot_since = hot_since or time.monotonic()
            if time.monotonic() - hot_since >= args.minutes * 60:
                log(
                    f"SPIN confirmed ({args.minutes} min): restarting {SERVICE}"
                )
                if sudo_restart_ollama():
                    log("RESTART ok")
                else:
                    log("RESTART FAILED - manual intervention needed")
                hot_since = None
        else:
            hot_since = None
        if args.once:
            return
        time.sleep(args.interval)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(0)
