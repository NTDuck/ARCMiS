//! Integration tests for the static hierarchy registry.

#![allow(deprecated)]

use mas_core::hierarchy::Role;
use mas_core::hierarchy::TeamRegistry;

#[test]
fn role_names_round_trip() {
    // Deprecated variants' names resolve to their new role through the
    // alias table, so the round-trip only holds for non-deprecated
    // variants (ADR 0038).
    for role in Role::ALL {
        if matches!(
            role,
            Role::Analyst
                | Role::Architect
                | Role::Planner
                | Role::Translator
                | Role::Validator
                | Role::Tester
                | Role::Critic
        ) {
            continue;
        }
        assert_eq!(Role::from_name(role.name()), Some(role));
    }
}

#[test]
fn role_name_lookup_is_case_insensitive() {
    assert_eq!(Role::from_name("Failure-Analyst"), Some(Role::FailureAnalyst));
    assert_eq!(Role::from_name("unknown"), None);
}

#[test]
fn direct_registry_has_no_teams() {
    let registry = TeamRegistry::direct();
    assert!(registry.is_direct());
    assert!(registry.teams().is_empty());
    assert!(registry.lead_of(Role::Analyst).is_none());
}

#[test]
fn disabled_hierarchy_compiles_to_direct() {
    let registry = TeamRegistry::compile(false, &[]);
    assert!(registry.is_direct());
}

#[test]
fn permits_answers_lead_membership() {
    let registry = TeamRegistry::direct();
    assert!(!registry.permits("lead", Role::Analyst));
}

fn team(lead: &str, members: &[&str]) -> run_config::TeamConfig {
    run_config::TeamConfig {
        lead: lead.to_owned(),
        members: members.iter().map(|name| (*name).to_owned()).collect(),
        turns: 10,
        stagnation_rounds: 3,
        strategy: None,
    }
}

#[test]
fn compiled_registry_maps_members_to_leads() {
    let registry = TeamRegistry::compile(true, &[team("migration-lead", &["uplift-worker", "independent-reviewer"])]);
    assert_eq!(registry.lead_of(Role::UpliftWorker), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::IndependentReviewer), Some("migration-lead"));
    assert!(registry.lead_of(Role::LegacyAnalyst).is_none());
    assert!(registry.permits("migration-lead", Role::UpliftWorker));
    assert!(!registry.permits("migration-lead", Role::LegacyAnalyst));
}

#[test]
fn unknown_members_are_dropped_with_warning() {
    let registry = TeamRegistry::compile(true, &[team("migration-lead", &["ghost"])]);
    let only = &registry.teams()[0];
    assert!(only.members.is_empty());
}

#[test]
fn duplicate_memberships_are_rejected() {
    let registry =
        TeamRegistry::compile(true, &[team("lead-a", &["uplift-worker"]), team("lead-b", &["uplift-worker"])]);
    let duplicates = registry.validate_unique_memberships();
    assert!(duplicates.is_err());
    assert_eq!(duplicates.unwrap_err(), vec!["uplift-worker"]);
}

#[test]
fn from_name_resolves_every_old_and_new_name() {
    let cases: &[(&str, Role)] = &[
        // Old names resolve to new roles via the alias table.
        ("analyst", Role::LegacyAnalyst),
        ("architect", Role::TopologyAnalyst),
        ("planner", Role::StrategySelector),
        ("translator", Role::UpliftWorker),
        ("validator", Role::IndependentReviewer),
        ("tester", Role::DifferentialTester),
        ("failure-analyst", Role::FailureAnalyst),
        ("repairer", Role::Repairer),
        ("critic", Role::ArchitectureCritic),
        ("fleet-analyst", Role::FleetAnalyst),
        // New names resolve to themselves.
        ("legacy-analyst", Role::LegacyAnalyst),
        ("topology-analyst", Role::TopologyAnalyst),
        ("business-rules-extractor", Role::BusinessRulesExtractor),
        ("citation-auditor", Role::CitationAuditor),
        ("strategy-selector", Role::StrategySelector),
        ("architecture-critic", Role::ArchitectureCritic),
        ("test-engineer", Role::TestEngineer),
        ("module-scheduler", Role::ModuleScheduler),
        ("uplift-worker", Role::UpliftWorker),
        ("transform-worker", Role::TransformWorker),
        ("reimagine-worker", Role::ReimagineWorker),
        ("independent-reviewer", Role::IndependentReviewer),
        ("differential-tester", Role::DifferentialTester),
        ("proof-builder", Role::ProofBuilder),
        ("security-auditor", Role::SecurityAuditor),
        ("patch-reviewer", Role::PatchReviewer),
    ];
    for (name, expected) in cases {
        assert_eq!(Role::from_name(name), Some(*expected), "name={name}");
    }
}

#[test]
fn alias_table_maps_every_old_name_to_new_role() {
    let aliases: &[(&str, Role)] = &[
        ("analyst", Role::LegacyAnalyst),
        ("architect", Role::TopologyAnalyst),
        ("planner", Role::StrategySelector),
        ("translator", Role::UpliftWorker),
        ("validator", Role::IndependentReviewer),
        ("tester", Role::DifferentialTester),
        ("failure-analyst", Role::FailureAnalyst),
        ("repairer", Role::Repairer),
        ("critic", Role::ArchitectureCritic),
        ("fleet-analyst", Role::FleetAnalyst),
    ];
    for (old_name, new_role) in aliases {
        assert_eq!(Role::from_name_with_alias(old_name), Some(*new_role), "old_name={old_name}");
    }
}

#[test]
fn compile_accepts_example_config_with_old_names() {
    let configs = vec![
        team("discovery-lead", &["analyst", "architect"]),
        team("migration-lead", &["translator", "validator", "tester", "repairer"]),
        team("integration-lead", &["planner", "failure-analyst", "critic"]),
    ];
    let registry = TeamRegistry::compile(true, &configs);
    assert_eq!(registry.teams().len(), 3);
    assert_eq!(registry.lead_of(Role::LegacyAnalyst), Some("discovery-lead"));
    assert_eq!(registry.lead_of(Role::TopologyAnalyst), Some("discovery-lead"));
    assert_eq!(registry.lead_of(Role::UpliftWorker), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::IndependentReviewer), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::DifferentialTester), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::Repairer), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::StrategySelector), Some("integration-lead"));
    assert_eq!(registry.lead_of(Role::FailureAnalyst), Some("integration-lead"));
    assert_eq!(registry.lead_of(Role::ArchitectureCritic), Some("integration-lead"));
}

#[test]
fn compile_accepts_six_team_config_with_new_names() {
    let configs = vec![
        team("discovery-lead", &["legacy-analyst", "topology-analyst"]),
        team("semantics-lead", &["business-rules-extractor", "citation-auditor"]),
        team("planning-lead", &["strategy-selector", "architecture-critic", "test-engineer"]),
        team(
            "migration-lead",
            &[
                "module-scheduler",
                "uplift-worker",
                "transform-worker",
                "reimagine-worker",
                "test-engineer",
                "failure-analyst",
                "repairer",
            ],
        ),
        team("verification-lead", &["independent-reviewer", "differential-tester", "proof-builder"]),
        team("security-lead", &["security-auditor", "patch-reviewer"]),
    ];
    let registry = TeamRegistry::compile(true, &configs);
    assert_eq!(registry.teams().len(), 6);
    assert_eq!(registry.lead_of(Role::LegacyAnalyst), Some("discovery-lead"));
    assert_eq!(registry.lead_of(Role::BusinessRulesExtractor), Some("semantics-lead"));
    assert_eq!(registry.lead_of(Role::StrategySelector), Some("planning-lead"));
    assert_eq!(registry.lead_of(Role::ModuleScheduler), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::ProofBuilder), Some("verification-lead"));
    assert_eq!(registry.lead_of(Role::SecurityAuditor), Some("security-lead"));
}

#[test]
fn validate_unique_memberships_rejects_duplicates() {
    let configs = vec![team("lead-a", &["uplift-worker"]), team("lead-b", &["uplift-worker"])];
    let registry = TeamRegistry::compile(true, &configs);
    let result = registry.validate_unique_memberships();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), vec!["uplift-worker"]);
}

#[test]
fn validate_unique_memberships_accepts_partition() {
    let configs = vec![
        team("discovery-lead", &["legacy-analyst", "topology-analyst"]),
        team("semantics-lead", &["business-rules-extractor", "citation-auditor"]),
    ];
    let registry = TeamRegistry::compile(true, &configs);
    assert!(registry.validate_unique_memberships().is_ok());
}
