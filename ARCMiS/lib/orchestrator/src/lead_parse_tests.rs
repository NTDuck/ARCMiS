//! Tests for the lead decision parser (ADR 0026 acceptance b/c). Kept
//! beside `lead.rs` to respect the file-size gate.

#[cfg(test)]
mod tests {
    use crate::lead::parse_lead_decision;
    use crate::lead::LeadDecision;

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
    fn lead_decision_accepts_member_and_refuses_cross_team() {
        let team = team(&["translator", "validator"]);
        let dispatch = parse_lead_decision("DECISION: delegate translator | port batch 1", &team);
        assert!(matches!(dispatch, Some(LeadDecision::Delegate { role, .. }) if role == agents::Role::Translator));

        // Another team's member: never dispatched.
        assert!(parse_lead_decision("DECISION: delegate analyst | explore", &team).is_none());
        // Another lead (leads are not specialist roles, so they never resolve).
        assert!(parse_lead_decision("DECISION: delegate migration-lead | sub-batch", &team).is_none());
    }

    /// `done` ends the inner loop (acceptance: the lead returns one outcome).
    #[test]
    fn lead_decision_done_ends_batch() {
        let team = team(&["translator"]);
        assert!(matches!(parse_lead_decision("DECISION: done", &team), Some(LeadDecision::Done)));
    }

    /// The lead parser never sees tier-1 lead-named dispatches. This test
    /// pins the parser side of the depth cap: a lead name is not a
    /// specialist role, so a lead cannot sub-delegate to another lead.
    #[test]
    fn lead_named_dispatch_is_never_a_specialist() {
        let team = team(&["translator"]);
        assert!(parse_lead_decision("DECISION: delegate discovery-lead | sub-batch", &team).is_none());
    }
}
