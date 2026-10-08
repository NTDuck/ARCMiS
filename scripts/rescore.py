#!/usr/bin/env python3
"""Toolchain rescore for one experiment directory.

The MAS v1 harness (post a2e41dd) writes manifest.json, traces/turns.jsonl,
and result/aggregate.yml, but no result/per_problem.json. The frontier
aggregator reads per_problem.json to compute pass rate and test counts.
This script runs the toolchain in the workspace and writes the missing
record. It is the scoring step the contract expects ("result/ after
scoring"); the harness exit code reports only the agents' build verdict.

Usage: rescore.py <experiment-dir>
"""
import json
import re
import subprocess
import sys
from pathlib import Path


def run(cmd, cwd, timeout=600):
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout + "\n" + r.stderr
    except subprocess.TimeoutExpired:
        return 124, "timeout"


def parse_test_counts(text):
    p = sum(int(m) for m in re.findall(r"(\d+) passed", text))
    f = sum(int(m) for m in re.findall(r"(\d+) failed", text))
    # Go test: `--- FAIL:` per failing test; `ok  ` lines count package-level
    # passes when no per-test counts exist.
    f += len(re.findall(r"^--- FAIL:", text, re.MULTILINE))
    if p == 0 and f == 0:
        p = len(re.findall(r"^ok  ", text, re.MULTILINE))
    # Maven surefire: `Tests run: X, Failures: Y` lines, summed.
    if not re.search(r"\d+ passed", text):
        for m in re.finditer(r"Tests run: (\d+), Failures: (\d+)", text):
            p += int(m.group(1)) - int(m.group(2))
            f += int(m.group(2))
    return p, f


def find_project_root(workspace: Path, target_language: str) -> Path | None:
    target = workspace / "target"
    if target.is_dir():
        if (target / "Cargo.toml").is_file():
            return target if target_language in ("rust", None, "") else None
        if target_language in ("python", None, "") and any(target.rglob("pyproject.toml")):
            return target
        if target_language in ("javascript", None, "") and any(target.rglob("package.json")):
            return target
        if target_language in ("go", None, "") and any(target.rglob("go.mod")):
            return target
        if target_language in ("java", None, "") and (any(target.rglob("pom.xml")) or any(target.rglob("build.gradle"))):
            return target
    # None, not workspace: without this the toolchain command runs from the
    # workspace and walks up into the ARCMiS repo itself, scoring the
    # repo's own suite instead of the translated code.
    return None


def rescore(exp_dir: Path) -> dict:
    manifest = json.loads((exp_dir / "manifest.json").read_text())
    target_language = manifest.get("target_language")
    workspace = exp_dir / "workspace"
    # A run without a translated tree must fail here, not fall through to
    # running the test command in the caller's own repo root (which once
    # scored the harness's own suite as the workspace's).
    if not (workspace / "target").is_dir():
        print(
            f"rescore: {exp_dir}: no workspace/target translated tree; nothing to score",
            file=sys.stderr,
        )
        sys.exit(1)
    if not workspace.is_dir():
        return {"problem": "workspace", "success": False, "stage": "setup",
                "tests_passed": 0, "tests_failed": 0, "detail": "workspace missing"}
    test_command = ""
    cfg_path = exp_dir / "config.yml"
    if cfg_path.is_file():
        for line in cfg_path.read_text().splitlines():
            m = re.match(r"^\s*test_command:\s*(.+?)\s*$", line)
            if m:
                test_command = m.group(1).strip()
                break

    project_root = find_project_root(workspace, target_language)
    record = {"problem": "workspace", "success": False, "stage": "evaluate",
              "tests_passed": 0, "tests_failed": 0, "detail": ""}
    # Wall-time metric: run wall from the trace span, per completed task.
    trace_path = exp_dir / "traces" / "turns.jsonl"
    if trace_path.is_file():
        try:
            stamps = [json.loads(line)["at"] for line in trace_path.read_text().splitlines() if line.strip()]
            if stamps:
                wall = stamps[-1] - stamps[0]
                record["wall_seconds"] = wall
                tasks_path = exp_dir / "run" / "tasks.json"
                if tasks_path.is_file():
                    tasks = json.loads(tasks_path.read_text())
                    done = sum(1 for task in tasks if task.get("status") == "done")
                    record["tasks_done"] = done
                    record["tasks_total"] = len(tasks)
                    if done:
                        record["wall_seconds_per_task"] = round(wall / done, 1)
        except (json.JSONDecodeError, KeyError, TypeError):
            pass
    if project_root is None:
        # No translated project under target/: the run never reached the
        # migration phase. Score as a translate-stage failure.
        record["stage"] = "translate"
        record["detail"] = "no translated project under workspace/target"
        return record

    if target_language is None:
        # Infer from the manifests the find actually matched, so a manifest
        # without target_language still scores the right toolchain.
        for marker, language in (("Cargo.toml", "rust"), ("go.mod", "go"), ("pom.xml", "java"),
                                 ("pyproject.toml", "python"), ("package.json", "javascript")):
            if (project_root / marker).is_file():
                target_language = language
                break
            if any(project_root.parent.rglob(marker)) or any(project_root.rglob(marker)):
                target_language = language
                break

    # Language -> (pre-build command, fallback test command). None test
    # fallback means test_command is required.
    toolchains = {
        "rust": (["cargo", "build"], ["cargo", "test"]),
        "go": (["go", "build", "./..."], ["go", "test", "./..."]),
        "java": (["mvn", "-q", "-DskipTests", "compile"], None),
        "python": (None, None),
        "javascript": (None, None),
    }
    pre_build, fallback_test = toolchains.get(target_language, (None, None))
    if pre_build:
        rc, out = run(pre_build, cwd=project_root, timeout=900)
        if rc != 0:
            record["stage"] = "translate"
            record["detail"] = out[-400:]
            return record
    if test_command:
        rc, out = run(["sh", "-c", test_command], cwd=project_root, timeout=900)
    elif fallback_test:
        rc, out = run(fallback_test, cwd=project_root, timeout=900)
    else:
        record["stage"] = "validate"
        record["detail"] = "no test_command configured"
        return record

    p, f = parse_test_counts(out)
    record["tests_passed"] = p
    record["tests_failed"] = f
    record["success"] = rc == 0 and f == 0
    if rc != 0:
        record["stage"] = "validate"
    record["detail"] = out[-400:]
    if rc == 0 and p == 0 and f == 0:
        record["detail"] = "package-level success; no per-test counts in output. " + record["detail"]
    return record


def main():
    exp_dir = Path(sys.argv[1])
    record = rescore(exp_dir)
    result_dir = exp_dir / "result"
    result_dir.mkdir(parents=True, exist_ok=True)
    (result_dir / "per_problem.json").write_text(json.dumps([record], indent=2) + "\n")
    print(json.dumps(record))


if __name__ == "__main__":
    main()
