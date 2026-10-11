//! Integration tests for the static hierarchy registry.

use mas_core::hierarchy::Role;
use mas_core::hierarchy::TeamRegistry;

#[test]
fn role_names_round_trip() {
    for role in Role::ALL {
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
    }
}

#[test]
fn compiled_registry_maps_members_to_leads() {
    let registry = TeamRegistry::compile(true, &[team("migration-lead", &["translator", "validator"])]);
    assert_eq!(registry.lead_of(Role::Translator), Some("migration-lead"));
    assert_eq!(registry.lead_of(Role::Validator), Some("migration-lead"));
    assert!(registry.lead_of(Role::Analyst).is_none());
    assert!(registry.permits("migration-lead", Role::Translator));
    assert!(!registry.permits("migration-lead", Role::Analyst));
}

#[test]
fn unknown_members_are_dropped_with_warning() {
    let registry = TeamRegistry::compile(true, &[team("migration-lead", &["ghost"])]);
    let only = &registry.teams()[0];
    assert!(only.members.is_empty());
}

#[test]
fn duplicate_memberships_are_rejected() {
    let registry = TeamRegistry::compile(true, &[team("lead-a", &["translator"]), team("lead-b", &["translator"])]);
    let duplicates = registry.validate_unique_memberships();
    assert!(duplicates.is_err());
    assert_eq!(duplicates.unwrap_err(), vec!["translator"]);
}
