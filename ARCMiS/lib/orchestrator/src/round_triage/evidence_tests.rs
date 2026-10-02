//! Evidence-reader tests over fixture ledgers (ADR 0028). Deterministic,
//! isolated, no checkpoint: the readers are pure file scans.

use std::path::Path;
use std::path::PathBuf;

use crate::round_triage::evidence_io::aggregate_fields;
use crate::round_triage::evidence_io::count_action;
use crate::round_triage::evidence_io::count_kind;
use crate::round_triage::evidence_io::read_failure_causes;
use crate::round_triage::evidence_io::AggregateFields;
use crate::round_triage::RoundEvidence;

/// A fixture round dir: `run/ledgers/*` plus `result/aggregate.yml`.
fn fixture_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("arcmis-round-triage-test-{}", std::process::id()));
    let ledgers = dir.join("run").join("ledgers");
    std::fs::create_dir_all(&ledgers).expect("mkdir ledgers");
    std::fs::create_dir_all(dir.join("result")).expect("mkdir result");

    std::fs::write(
        ledgers.join("failures.jsonl"),
        concat!(
            "{\"root_cause\":\"MaxTurnsError: reached max turns limit: 40\"}\n",
            "{\"root_cause\":\"CompletionError: finish_reason=Length; out of output budget\"}\n",
            "{\"root_cause\":\"ProviderError: context_length_exceeded\"}\n",
            "{\"root_cause\":\"validator produced no VALIDATION line\"}\n",
        ),
    )
    .expect("write failures");
    std::fs::write(
        ledgers.join("observations.jsonl"),
        concat!(
            "{\"kind\":\"round_stalled\",\"detail\":{}}\n",
            "{\"kind\":\"round_stalled\",\"detail\":{}}\n",
            "{\"kind\":\"round_progress\",\"detail\":{}}\n",
            "{\"kind\":\"jev_triage\",\"detail\":{\"action\":\"stop\"}}\n",
        ),
    )
    .expect("write observations");
    std::fs::write(
        ledgers.join("decisions.jsonl"),
        concat!(
            "{\"action\":\"delegate\",\"detail\":{\"role\":\"repairer\"}}\n",
            "{\"action\":\"escalate\",\"detail\":{}}\n",
            "{\"action\":\"delegate\",\"detail\":{\"role\":\"validator\"}}\n",
        ),
    )
    .expect("write decisions");
    std::fs::write(
        dir.join("result").join("aggregate.yml"),
        concat!(
            "final_phase: Integration\n",
            "completed: false\n",
            "rounds: 15\n",
            "delegations: 7\n",
            "tasks_done: 7\n",
            "tasks_total: 7\n",
            "stop_reason: 3 consecutive rounds without a completed task; stopping\n",
            "duration_seconds: 10290\n",
        ),
    )
    .expect("write aggregate");
    dir
}

#[test]
fn evidence_load_counts_fixture_ledger_rows() {
    let dir = fixture_dir();
    let evidence = RoundEvidence::load(&dir).expect("load");
    assert_eq!(evidence.max_turns_deaths, 1);
    assert_eq!(evidence.output_cap_deaths, 1);
    assert_eq!(evidence.context_length_events, 1);
    assert_eq!(evidence.failures, 4);
    assert_eq!(evidence.stalled_rounds, 2);
    assert_eq!(evidence.escalations, 1);
    assert!(!evidence.completed);
    assert_eq!(evidence.phase_reached, "Integration");
    assert_eq!(evidence.rounds, 15);
    assert_eq!(evidence.wall_seconds, Some(10290));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn evidence_load_errors_without_ledgers() {
    let dir = std::env::temp_dir().join(format!("arcmis-round-triage-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    assert!(RoundEvidence::load(&dir).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn aggregate_fields_parse_scalars() {
    let dir = fixture_dir();
    let fields: AggregateFields =
        aggregate_fields(&Path::new(&dir).join("result").join("aggregate.yml")).expect("parse");
    assert_eq!(fields.final_phase, "Integration");
    assert!(!fields.completed);
    assert_eq!(fields.stop_reason, "3 consecutive rounds without a completed task; stopping");
    assert_eq!(fields.rounds, 15);
    assert_eq!(fields.wall_seconds, Some(10290));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_aggregate_reports_in_flight_zeros() {
    let dir = fixture_dir();
    let fields = aggregate_fields(&Path::new(&dir).join("result").join("missing.yml")).expect("missing ok");
    assert!(!fields.completed);
    assert_eq!(fields.final_phase, String::new());
    assert_eq!(fields.wall_seconds, None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn counters_tolerate_missing_files() {
    let dir = std::env::temp_dir().join(format!("arcmis-round-triage-nofiles-{}", std::process::id()));
    let ledgers = dir.join("ledgers");
    std::fs::create_dir_all(&ledgers).expect("mkdir");
    assert_eq!(count_kind(&ledgers.join("observations.jsonl"), "round_stalled").expect("count"), 0);
    assert_eq!(count_action(&ledgers.join("decisions.jsonl"), "escalate").expect("count"), 0);
    assert!(read_failure_causes(&ledgers.join("failures.jsonl")).expect("read").is_empty());
    std::fs::remove_dir_all(&dir).ok();
}
