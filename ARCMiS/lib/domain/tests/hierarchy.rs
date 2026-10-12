//! Integration tests for the hierarchy module.

use mas_domain::Role;
use mas_domain::TeamRegistry;
use run_config::TeamConfig;

fn team(lead: &str, members: &[&str]) -> TeamConfig {
    TeamConfig {
        lead: lead.to_owned(),
        members: members.iter().map(|name| name.to_string()).collect(),
        turns: 3,
        stagnation_rounds: 2,
        strategy: None,
    }
}

#[test]
#[allow(deprecated)]
fn role_name_round_trips() {
    // WHY exclude deprecated roles: the alias table maps old names to
    // their new role, so a deprecated role's name resolves to the new
    // role, not the deprecated one.
    let deprecated_names = ["analyst", "architect", "planner", "translator", "validator", "tester", "critic"];
    for role in Role::ALL {
        let name = role.name();
        if deprecated_names.contains(&name) {
            continue;
        }
        assert_eq!(Role::from_name(name), Some(role));
    }
}

#[test]
fn alias_table_resolves_old_names_to_new_roles() {
    assert_eq!(Role::from_name("analyst"), Some(Role::LegacyAnalyst));
    assert_eq!(Role::from_name("architect"), Some(Role::TopologyAnalyst));
    assert_eq!(Role::from_name("planner"), Some(Role::StrategySelector));
    assert_eq!(Role::from_name("translator"), Some(Role::UpliftWorker));
    assert_eq!(Role::from_name("validator"), Some(Role::IndependentReviewer));
    assert_eq!(Role::from_name("tester"), Some(Role::DifferentialTester));
    assert_eq!(Role::from_name("critic"), Some(Role::ArchitectureCritic));
}

#[test]
fn direct_registry_has_no_teams() {
    let registry = TeamRegistry::direct();
    assert!(registry.is_direct());
    assert!(registry.teams().is_empty());
}

#[test]
fn compile_resolves_team_members() {
    let configs = vec![team("planner-lead", &["strategy-selector", "test-engineer"])];
    let registry = TeamRegistry::compile(true, &configs);
    assert!(!registry.is_direct());
    assert_eq!(registry.teams().len(), 1);
    assert_eq!(registry.lead_of(Role::StrategySelector), Some("planner-lead"));
    assert_eq!(registry.lead_of(Role::TestEngineer), Some("planner-lead"));
    assert_eq!(registry.lead_of(Role::FailureAnalyst), None);
}

#[test]
fn disabled_hierarchy_is_direct() {
    let configs = vec![team("planner-lead", &["strategy-selector"])];
    let registry = TeamRegistry::compile(false, &configs);
    assert!(registry.is_direct());
}

#[test]
fn permits_checks_lead_membership() {
    let configs = vec![team("planner-lead", &["strategy-selector"])];
    let registry = TeamRegistry::compile(true, &configs);
    assert!(registry.permits("planner-lead", Role::StrategySelector));
    assert!(!registry.permits("other-lead", Role::StrategySelector));
}

#[test]
fn validate_unique_memberships_rejects_duplicates() {
    let configs = vec![team("lead-a", &["strategy-selector"]), team("lead-b", &["strategy-selector"])];
    let registry = TeamRegistry::compile(true, &configs);
    let duplicates = registry.validate_unique_memberships().expect_err("duplicate membership");
    assert!(duplicates.contains(&"strategy-selector"));
}
