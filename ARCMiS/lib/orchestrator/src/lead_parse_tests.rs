//! Tests for the lead decision parser (ADR 0026 acceptance b/c, extended
//! by ADR 0029 multi-delegate rounds). Kept beside `lead.rs` to respect
//! the file-size gate.

#[cfg(test)]
mod tests {
    use crate::lead::parse_lead_round;
    use crate::lead::LeadRound;

    fn team(members: &[&str]) -> agents::mas::leads::Team {
        agents::mas::leads::Team {
            lead: "migration-lead".into(),
            members: members.iter().filter_map(|name| agents::Role::from_name(name)).collect(),
            turns: 4,
            stagnation_rounds: Some(2),
        }
    }

    /// A lead dispatches only its own team's roles: a cross-team or
    /// lead-named delegation draws a refusal (acceptance b/c, ADR 0026).
    #[test]
    fn lead_round_accepts_member_and_refuses_cross_team() {
        let team = team(&["translator", "validator"]);
        let round = parse_lead_round("DECISION: delegate translator | port batch 1", &team);
        assert!(
            matches!(round, LeadRound::Dispatches(d) if d.len() == 1 && d[0].0 == agents::Role::Translator),
            "one member dispatch expected"
        );

        // Another team's member: never dispatched.
        assert!(
            matches!(parse_lead_round("DECISION: delegate analyst | explore", &team), LeadRound::Dispatches(d) if d.is_empty())
        );
        // Another lead (leads are not specialist roles, so they never resolve).
        assert!(
            matches!(parse_lead_round("DECISION: delegate migration-lead | sub-batch", &team), LeadRound::Dispatches(d) if d.is_empty())
        );
    }

    /// `done` ends the inner loop (acceptance: the lead returns one outcome).
    /// A done line wins over delegate lines in the same answer.
    #[test]
    fn lead_round_done_ends_batch_and_beats_delegates() {
        let team = team(&["translator"]);
        assert!(matches!(parse_lead_round("DECISION: done", &team), LeadRound::Done));
        let mixed = "DECISION: delegate translator | port batch 1\nDECISION: done";
        assert!(matches!(parse_lead_round(mixed, &team), LeadRound::Done));
    }

    /// The lead parser never sees tier-1 lead-named dispatches. This test
    /// pins the parser side of the depth cap: a lead name is not a
    /// specialist role, so a lead cannot sub-delegate to another lead.
    #[test]
    fn lead_named_dispatch_is_never_a_specialist() {
        let team = team(&["translator"]);
        assert!(
            matches!(parse_lead_round("DECISION: delegate discovery-lead | sub-batch", &team), LeadRound::Dispatches(d) if d.is_empty())
        );
    }

    /// ADR 0029: several valid delegate lines in one answer parse to
    /// several dispatches, in emission order.
    #[test]
    fn lead_round_parses_multiple_delegates_in_order() {
        let team = team(&["translator", "validator", "tester"]);
        let answer = "DECISION: delegate translator | port batch 1\nDECISION: delegate validator | check batch \
                      1\nDECISION: delegate tester | run tests for batch 1";
        let round = parse_lead_round(answer, &team);
        match round {
            LeadRound::Dispatches(d) => {
                let roles: Vec<agents::Role> = d.iter().map(|(role, _)| *role).collect();
                assert_eq!(roles, vec![agents::Role::Translator, agents::Role::Validator, agents::Role::Tester]);
                assert_eq!(d[0].1, "port batch 1");
                assert_eq!(d[2].1, "run tests for batch 1");
            },
            other => panic!("expected dispatches, got {other:?}"),
        }
    }

    /// A refused line between valid lines does not disturb the order of
    /// the accepted ones, and an all-refused answer reads as an empty
    /// round (the caller's stagnation path).
    #[test]
    fn lead_round_refusals_do_not_disturb_valid_order() {
        let team = team(&["translator", "tester"]);
        let answer = "DECISION: delegate tester | run tests\nDECISION: delegate analyst | explore\nDECISION: delegate \
                      translator | port batch 2";
        let round = parse_lead_round(answer, &team);
        match round {
            LeadRound::Dispatches(d) => {
                let roles: Vec<agents::Role> = d.iter().map(|(role, _)| *role).collect();
                assert_eq!(roles, vec![agents::Role::Tester, agents::Role::Translator]);
            },
            other => panic!("expected dispatches, got {other:?}"),
        }
        assert!(
            matches!(parse_lead_round("DECISION: delegate analyst | explore", &team), LeadRound::Dispatches(d) if d.is_empty())
        );
    }
}
