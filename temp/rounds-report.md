# Autooptimise campaign rounds report

Ledger: 57 rows in ROUNDS.yaml (verdicts: SOLVED 21, NOT CLEARED 21, ABORTED 14, IN FLIGHT 1). Scoreboard: 13/114 cleared (per-family clears: crust 11, oxidizer 1, skel 1; chtrie is a ledger SOLVED the campaign counts VERIFIED-WORKSPACE, retry candidate, so the scoreboard stays 13/114). Campaign window 2026-09-26 to 2026-10-06; the ledger itself starts 2026-09-30 (v2r1), earlier v1 smoke rounds are not ledger rows. Engine: ninfer/qwen3.8-27b.

Rows are chronological by candidate_id. Time gives the round start (UTC) and wall-clock minutes; '-' means the wall time was not recorded. Metrics are compile status and tests passed/failed with the best pass rate of that round. LoC counts source lines by extension (.c/.h, .py, .go, .java) in the problem tree under assets/ReCodeAgent/data/tool_projects/.

| Round | Time | Problem | Metrics | Verdict | Notes |
|---|---|---|---|---|---|
| autoopt-v0.2.1 (20260930T081500Z) | 2026-09-30 08:15Z + - | 20260930T081500Zv2r1-v2r1-crust-leftpad-ninfer (non-sweep; no problem tree) | - - | ABORTED | (hypothesis unknown) |
| autoopt-v0.3.1 (20260930T092000Z) | 2026-09-30 09:20Z + 111 min | 20260930T092000Zv3r1-v3r1-crust-leftpad-ninfer (non-sweep; no problem tree) | ✓ 9/0 (100%) | SOLVED | Provider + model pinned: ninfer/qwen3.8-27b (ninfer-serve :8081 via the netmind ... |
| autoopt-v0.3.2 (20260930T112000Z) | 2026-09-30 11:20Z + 25 min | 20260930T112000Zv3r2-v3r2-crust-leftpad-confirm (non-sweep; no problem tree) | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.3 (20260930T132000Z) | 2026-09-30 13:20Z + 17 min | 20260930T132000Zv3r3-v3r3-crust-leftpad-repair16384 (non-sweep; no problem tree) | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.4 (20260930T153000Z) | 2026-09-30 15:30Z + 197 min | 20260930T153000Zv3r4-v3r4-crust-leftpad-replangate (non-sweep; no problem tree) | ✓ 22/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.5 (20260930T211500Z) | 2026-09-30 21:15Z + 99 min | 20260930T211500Zv3r5-v3r5-crust-leftpad-orchout16384 (non-sweep; no problem tree) | ✓ 20/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.6 (20260930T235900Z) | 2026-09-30 23:59Z + 114 min | 20260930T235900Zv3r6-v3r6-crust-leftpad-replicate (non-sweep; no problem tree) | ✓ 10/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s0 (20261001T000604Z) | 2026-10-01 00:06Z + 115 min | amp; crust C to Rust; LoC 188 | ✓ 18/0 (100%) | SOLVED | coverage round: crust/amp (untouched, 185 LOC), v0.3 sweep config family, no con... |
| autoopt-v0.3.sweep.s1 (20261001T002104Z) | 2026-10-01 00:21Z + 222 min | gonameparts; oxidizer Go to Rust; LoC 1059 | ✓ 27/0 (100%) | SOLVED | coverage round: oxidizer/gonameparts (untouched, 813 LOC go); single delta: mas.... [single-delta jev A/B round] |
| autoopt-v0.3.sweep.s2 (20261001T021636Z) | 2026-10-01 02:16Z + 140 min | ulidgen; crust C to Rust; LoC 187 | ✓ 13/0 (100%) | SOLVED | coverage round: crust/ulidgen, untouched project, v0.3 sweep config family (tran... |
| autoopt-v0.3.sweep.s3 (20261001T041913Z) | 2026-10-01 04:19Z + 41 min | fft; crust C to Rust; LoC 233 | ✗ - | NOT CLEARED | single delta: mas.jev_triage enabled, checkpoint assets/models/laya-typed-decisi... [single-delta jev A/B round] |
| autoopt-v0.3.sweep.s4 (20261001T044543Z) | 2026-10-01 04:45Z + - | edlib; oxidizer Go to Rust; LoC 2653 | - - | ABORTED | coverage round: oxidizer/go-edlib (untouched, 2412 LOC go); single delta: jev_tr... [single-delta jev A/B round] |
| autoopt-v0.3.sweep.s5 (20261001T051430Z) | 2026-10-01 05:14Z + 149 min | avalanche; crust C to Rust; LoC 247 | ✓ 11/0 (100%) | SOLVED | coverage round: crust/avalanche (untouched, 247 LOC); jev-off baseline for the 0... |
| autoopt-v0.3.sweep.s6 (20261001T075136Z) | 2026-10-01 07:51Z + 323 min | coroutine; crust C to Rust; LoC 256 | ✗ 2/0 (100%) | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s7 (20261001T081450Z) | 2026-10-01 08:14Z + 74 min | colorsys; skel Python to Rust; LoC 355 | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s8 (20261001T095303Z) | 2026-10-01 09:53Z + - | colorsys; skel Python to Rust; LoC 355 | ✓ 5/0 (100%) | SOLVED | coverage retry: skel/colorsys (untouched, 355 LOC python); v3s7 died on engine n... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s9 (20261001T110858Z) | 2026-10-01 11:08Z + - | morton; crust C to Rust; LoC 268 | - - | ABORTED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s10 (20261001T114151Z) | 2026-10-01 11:41Z + - | morton; crust C to Rust; LoC 268 | - - | ABORTED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s11 (20261001T114159Z) | 2026-10-01 11:41Z + - | libqueue; crust C to Rust; LoC 277 | - - | ABORTED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s12 (20261001T121012Z) | 2026-10-01 12:10Z + 172 min | morton; crust C to Rust; LoC 268 | ✓ 2/0 (100%) | SOLVED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s13 (20261001T125936Z) | 2026-10-01 12:59Z + - | libqueue; crust C to Rust; LoC 277 | - - | ABORTED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s14 (20261001T132415Z) | 2026-10-01 13:24Z + 124 min | libqueue; crust C to Rust; LoC 277 | ✓ 2/0 (100%) | SOLVED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.7 (20261001T143000Z) | 2026-10-01 14:30Z + 146 min | 20261001T143000Zv3r7-v3r7-crust-leftpad-blockedgate (non-sweep; no problem tree) | ✗ - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s13 (20261001T150642Z) | 2026-10-01 15:06Z + 200 min | libqueue; crust C to Rust; LoC 277 | - - | NOT CLEARED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s15 (20261001T150810Z) | 2026-10-01 15:08Z + 204 min | murmurhash; crust C to Rust; LoC 324 | ✓ 16/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s16 (20261001T153933Z) | 2026-10-01 15:39Z + 18 min | geofence; crust C to Rust; LoC 324 | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s17 (20261001T160913Z) | 2026-10-01 16:09Z + - | geofence; crust C to Rust; LoC 324 | ✓ 6/0 (100%) | SOLVED | coverage round: crust/geofence retry (v3s16 died on admission 503 x3 at Discover... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s18 (20261001T180718Z) | 2026-10-01 18:07Z + 35 min | gfc; crust C to Rust; LoC 259 | ✓ 6/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s19 (20261001T185700Z) | 2026-10-01 18:57Z + 206 min | chtrie; crust C to Rust; LoC 259 | ✗ 14/2 (88%) | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.8 (20261001T193000Z) | 2026-10-01 19:30Z + 146 min | 20261001T193000Zv3r8-v3r8-crust-leftpad-transnothink (non-sweep; no problem tree) | ✓ 18/0 (100%) | SOLVED | Translator turn-efficiency (max wall-share bottleneck, named from traces): trans... |
| autoopt-v0.3.sweep.s20 (20261001T212709Z) | 2026-10-01 21:27Z + 172 min | 2dpartint; crust C to Rust; LoC 855 | ✓ 27/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s21 (20261001T224920Z) | 2026-10-01 22:49Z + 8 min | csv; alphatrans Java to Rust; LoC 15481 | ✗ - | ABORTED | (hypothesis unknown) |
| autoopt-v0.3.9 (20261002T003000Z) | 2026-10-02 00:30Z + - | 20261002T003000Zv3r9-v3r9-crust-leftpad-nostdgate (non-sweep; no problem tree) | - - | ABORTED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s22 (20261002T004008Z) | 2026-10-02 00:40Z + 48 min | printf; crust C to Rust; LoC 314 | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s23 (20261002T014623Z) | 2026-10-02 01:46Z + 44 min | simd; crust C to Rust; LoC 474 | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.10 (20261002T023000Z) | 2026-10-02 02:30Z + 52 min | 20261002T023000Zv3r10-v3r10-crust-leftpad-orchthinkoff (non-sweep; no problem tree) | ✗ - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.11 (20261002T030000Z) | 2026-10-02 03:00Z + 172 min | 20261002T030000Zv3r11-v3r11-crust-leftpad-thinkwire (non-sweep; no problem tree) | ✓ 13/0 (100%) | SOLVED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s24 (20261002T030311Z) | 2026-10-02 03:03Z + 50 min | bhshell; crust C to Rust; LoC 645 | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s25 (20261002T044900Z) | 2026-10-02 04:49Z + - | bhshell; crust C to Rust; LoC 645 | ✓ 36/0 (100%) | SOLVED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s26 (20261002T052620Z) | 2026-10-02 05:26Z + 65 min | bigint; crust C to Rust; LoC 1178 | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s27 (20261002T064559Z) | 2026-10-02 06:45Z + - | csv; alphatrans Java to Rust; LoC 15481 | ✓ 5/0 (100%) | SOLVED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s28 (20261002T075338Z) | 2026-10-02 07:53Z + - | bigint; crust C to Rust; LoC 1178 | - - | ABORTED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s29 (20261002T094410Z) | 2026-10-02 09:44Z + - | bigint; crust C to Rust; LoC 1178 | - - | ABORTED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s30 (20261002T105943Z) | 2026-10-02 10:59Z + - | bigint; crust C to Rust; LoC 1178 | ✗ - | NOT CLEARED | (hypothesis unknown) [retry of earlier problem attempt] |
| autoopt-v0.3.12 (20261002T130000Z) | 2026-10-02 13:00Z + 11 min | 20261002T130000Zv3r12-v3r12-crust-leftpad-repnothink (non-sweep; no problem tree) | - - | NOT CLEARED | (hypothesis unknown) |
| autoopt-v0.3.sweep.s0 (20261004T081728Z) | 2026-10-04 08:17Z + - | remimu; crust C to Rust; LoC 1483 | ✗ - | ABORTED | ninfer engine reinstated (qwen3.8-27b); q27 excursion reverted; coverage continu... |
| autoopt-v0.3.sweep.s1 (20261004T082228Z) | 2026-10-04 08:22Z + 65 min | heapq; skel Python to Rust; LoC 497 | ✗ - | NOT CLEARED | ninfer engine reinstated (qwen3.8-27b); q27 excursion reverted; coverage continu... |
| autoopt-v0.3.sweep.s3 (20261004T160230Z) | 2026-10-04 16:02Z + - | bst; skel Python to Rust; LoC 808 | - - | ABORTED | ninfer engine reinstated (qwen3.8-27b); q27 excursion reverted; coverage continu... |
| autoopt-v0.3.sweep.s31 (20261005T071828Z) | 2026-10-05 07:18Z + 839 min | remimu; crust C to Rust; LoC 1483 | ✗ 0/2 (0%) | NOT CLEARED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s31 (20261005T072228Z) | 2026-10-05 07:22Z + 1 min | fft; crust C to Rust; LoC 233 | - - | NOT CLEARED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s31 (20261005T072628Z) | 2026-10-05 07:26Z + 21 min | bst; skel Python to Rust; LoC 808 | - - | NOT CLEARED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s31 (20261005T220632Z) | 2026-10-05 22:06Z + - | skp; crust C to Rust; LoC 2338 | - - | ABORTED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... |
| autoopt-v0.3.sweep.s31 (20261005T221032Z) | 2026-10-05 22:10Z + - | rbt; skel Python to Rust; LoC 906 | - - | ABORTED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... |
| autoopt-v0.3.sweep.s33 (20261005T222327Z) | 2026-10-05 22:23Z + 453 min | skp; crust C to Rust; LoC 2338 | ✓ - | SOLVED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s33 (20261005T222727Z) | 2026-10-05 22:27Z + 91 min | rbt; skel Python to Rust; LoC 906 | ✗ - | NOT CLEARED | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s33 (20261006T055729Z) | 2026-10-06 05:57Z + 321 min | chtrie; crust C to Rust; LoC 259 | ✓ 10/0 (100%) | SOLVED (ledger); campaign counts VERIFIED-WORKSPACE, retry candidate | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... [retry of earlier problem attempt] |
| autoopt-v0.3.sweep.s33 (20261006T060129Z) | 2026-10-06 06:01Z + in flight | strsim; skel Python to Rust; LoC 1594 | - - | IN FLIGHT | ninfer engine reinstated after GPU1 bus-drop outage 2026-10-04/05 and q27-serve ... |

generated from ROUNDS.yaml + problem trees; regenerable via python3 temp/fig-rounds-report.py
