# Wave-4 teardown disposition (2026-10-08) — scratch, NOT .artifacts

## External teardown evidence
- 2026-10-08 10:33:38Z (17:33:38 +0700): `q27sweep-driver.service` and `q27sweep-watchdog.service` stopped; unit definitions subsequently REMOVED (systemctl: "Unit q27sweep-driver.service could not be found"). `q27sweep.slice` still exists.
- journalctl excerpt (only lines at that moment, no stop/kill signals):
  ```
  Oct 08 17:33:38 systemd[1230]: q27sweep-watchdog.service: Consumed 16.187s CPU time over 1d 19h 33.313s wall clock time, 10M memory peak, 1.4M memory swap peak.
  Oct 08 17:33:38 systemd[1230]: q27sweep-driver.service: Consumed 6min 9.846s CPU time over 1d 19h 32.512s wall clock time, 1.4G memory peak, 67.3M memory swap peak.
  ```
- Coincides with user SSH disconnects 17:33:09 / 17:33:33 / 17:33:42 +0700. Same recovery class as 2026-10-06 reboot (external session teardown, not harness fault).

## toml (20261007T164013Zv3s34-toml-sweep, driver idx 12) — died mid-Migration, NO terminal class
- harness PID 438319 dead ~17:27-17:33 +0700 (~10:27-10:33Z). stdout: 0 ERROR lines; launcher log: no death line. No done event, no result/, no ROUNDS.yaml.
- run/ snapshot INTACT for possible resume: state.json {phase: Migration, phase_delegations: 4, round: null} (mtime 16:42:48 +0700); tasks.json 9 tasks; traces/turns.jsonl last write 17:33:29 +0700; ledgers decisions/observations/failures present (decisions last 17:27:42 +0700; observations last 16:42:48 +0700; failures.jsonl EMPTY).
- Event timeline: t2 pass 19:56:08Z, Planning 19:58:26Z, t3 fail 22:42:47Z, t4 fail 02:02:08Z (08 Oct), Pilot 04:02:06Z, t6 pass 04:31:31Z, Migration 04:31:36Z, t7 fail 08:44:03Z, t8 fail 09:42:48Z.
- Stall snapshot at death: observations ledger = {delegation_pass 18, lead_batch_pass 4, round_progress 5, round_stalled 7, lead_batch_fail 4}.
- Full-window wall if scored from launch: 16:40:13Z 07 Oct → ~10:33Z 08 Oct teardown (no done event → no wall from harness; use teardown ts if a disposition row is ever written).

## totp (20261007T164413Zv3s34-totp-sweep, driver idx 13) — stagnation-stop TERMINAL (census complete)
- done event 01:24:06Z: stop_reason "3 consecutive rounds without a completed task; stopping", 3 rounds / 2 delegations / 0 tasks_done, final_phase Discovery, completed=false, duration 1732s, harness_id mas-20261008005513. result/aggregate.yml written; ROUNDS.yaml NOT written (driver-owned, wave never closed).
- Attempt table: a1 16:44:13Z→00:17:38Z fatal admission-503 (in-run 3h33m; last decision gate "1 blocked task(s) still in the run" 30s pre-death); a2 00:27:38Z→00:40:13Z fatal admission-503 (13m post snapshot-resume; Discovery re-entry confirmed, delegations reset); a3 ~00:55:13Z→01:24:06Z survived 503s, stagnation-stop.
- Census extras: in-run orchestrator replan walk-back to Discovery 21:54:51Z (same PID 3032114); gate-refusal positive control ~01:36Z ("done claimed with no completed delegation" — refusal path working as designed). Wave-4 fatal-launcher ladder 2/2 fired ~01:19Z, report+wait honored.

## Retry queue (user decision pending)
coroutine (PAIRS idx 9), mathgen (idx 10), totp (idx 13), toml (idx 12 — fresh-dir semantics required if relaunched; no terminal class to score).

## Surviving infrastructure (verified ~10:45Z)
- Engine ninfer :8081 UP (models probe 200). GPU0 2928MiB used / 97% util at probe; second probe 22482MiB / 0% — engine alive, idle-varying. Outside the user slice; survived.
- Harness processes: 0. Heartbeats frozen at teardown (sweep-hb last write 17:29:19 +0700).
- Unstarted: wave-5 (textrank idx 14, csyncmers idx 15) — no dirs created; go-edlib (idx 16) solo wave untouched.
- Scoreboard: 13/114 (frozen). ROUNDS.yaml at 65 rows (campaign root).
