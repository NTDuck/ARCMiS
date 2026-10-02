//! Round-state mapping tests (ADR 0028). No checkpoint: the state builder
//! and evidence summary are pure.

#[cfg(test)]
mod tests {
    use super::super::RoundEvidence;

    fn evidence() -> RoundEvidence {
        RoundEvidence {
            phase_reached: "Integration".into(),
            completed: false,
            stop_reason: "3 consecutive rounds without a completed task; stopping".into(),
            stalled_rounds: 8,
            rounds: 15,
            delegations: 7,
            tasks_done: 7,
            tasks_total: 7,
            max_turns_deaths: 1,
            output_cap_deaths: 2,
            escalations: 3,
            context_length_events: 1,
            failures: 4,
            wall_seconds: Some(7465),
        }
    }

    #[test]
    fn summary_carries_the_round_counters() {
        let summary = evidence().summary();
        assert!(summary.contains("phase Integration"));
        assert!(summary.contains("tasks 7/7"));
        assert!(summary.contains("stalled 8"));
        assert!(summary.contains("stop: 3 consecutive rounds"));
        assert!(summary.contains("2 output-cap deaths"));
        assert!(summary.contains("wall 7465s"));
    }

    #[test]
    fn summary_omits_empty_counters() {
        let mut clean = evidence();
        clean.max_turns_deaths = 0;
        clean.output_cap_deaths = 0;
        clean.escalations = 0;
        clean.context_length_events = 0;
        clean.failures = 0;
        clean.stop_reason = String::new();
        clean.wall_seconds = None;
        let summary = clean.summary();
        assert!(!summary.contains("deaths"));
        assert!(!summary.contains("escalations"));
        assert!(!summary.contains("stop:"));
        assert!(!summary.contains("wall"));
    }

    #[test]
    fn round_state_maps_evidence_into_the_trained_fields() {
        let state = super::super::round_state(&evidence());
        assert_eq!(state["role"], serde_json::json!("migration-round"));
        assert_eq!(state["phase"], serde_json::json!("Integration"));
        assert_eq!(state["passed"], serde_json::json!(false));
        let output = state["output"].as_str().expect("output");
        assert!(output.contains("phase Integration"));
    }
}
