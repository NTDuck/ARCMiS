//! Preflight validation. Rejects mandatory-field omissions and hierarchy
//! ambiguities before any agent launches. Every finding names the rejected
//! field and says what to fix (ADR 0032).

use crate::errors::ValidationError;
use crate::hierarchy::TeamConfig;
use crate::schema::SectionConfig;
use std::collections::HashMap;

/// Runs every preflight check. Collects every finding so that the operator
/// fixes the whole file in one pass.
pub fn expect_valid(config: &SectionConfig) -> Vec<ValidationError> {
    reject_blank_strings(config)
        .into_iter()
        .chain(reject_unknown_members(config))
        .chain(reject_ambiguous_members(config))
        .collect()
}

/// Rejects a blank mandatory field.
#[must_use]
fn reject_blank_strings(config: &SectionConfig) -> Vec<ValidationError> {
    [
        (&config.run.name, "run.name"),
        (&config.paths.input_dir, "paths.input_dir"),
        (&config.paths.output_dir, "paths.output_dir"),
        (&config.languages.pair.source, "languages.pair.source"),
        (&config.languages.pair.target, "languages.pair.target"),
        (&config.languages.test_command, "languages.test_command"),
    ]
    .iter()
    .filter(|&&(value, _field)| value.trim().is_empty())
    .map(|&(_value, field)| blank_field(field))
    .collect()
}

#[must_use]
fn blank_field(field: &'static str) -> ValidationError {
    ValidationError {
        field,
        reason: String::from("must not be empty or whitespace"),
    }
}

/// Rejects a member name that does not resolve to any role. The alias
/// table covers old names; anything else is a typo (ADR 0038).
#[must_use]
fn reject_unknown_members(config: &SectionConfig) -> Vec<ValidationError> {
    config.hierarchy.teams.iter().flat_map(|team| reject_unknown_in_team(team)).collect()
}

#[must_use]
fn reject_unknown_in_team(team: &TeamConfig) -> Vec<ValidationError> {
    team.members
        .iter()
        .filter(|member| !is_known_member(member))
        .map(|member| ValidationError {
            field: "hierarchy.teams.members",
            reason: format!("{member} is not a known role"),
        })
        .collect()
}

/// Whether `name` resolves to a role through the alias table.
fn is_known_member(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "analyst"
            | "architect"
            | "planner"
            | "translator"
            | "validator"
            | "tester"
            | "failure-analyst"
            | "repairer"
            | "critic"
            | "fleet-analyst"
            | "legacy-analyst"
            | "topology-analyst"
            | "business-rules-extractor"
            | "citation-auditor"
            | "strategy-selector"
            | "architecture-critic"
            | "test-engineer"
            | "module-scheduler"
            | "uplift-worker"
            | "transform-worker"
            | "reimagine-worker"
            | "independent-reviewer"
            | "differential-tester"
            | "proof-builder"
            | "security-auditor"
            | "patch-reviewer"
    )
}

/// A member that sits in two teams makes routing ambiguous. Preflight
/// rejects it. Every member needs exactly one owning lead (ADR 0026).
#[must_use]
fn reject_ambiguous_members(config: &SectionConfig) -> Vec<ValidationError> {
    let mut owning_lead: HashMap<String, String> = HashMap::new();

    config.hierarchy.teams.iter().flat_map(|team| reject_duplicate_members(team, &mut owning_lead)).collect()
}

#[must_use]
fn reject_duplicate_members(team: &TeamConfig, owning_lead: &mut HashMap<String, String>) -> Vec<ValidationError> {
    team.members.iter().filter_map(|member| check_claim(&team.lead, member, owning_lead)).collect()
}

#[must_use]
fn check_claim(lead: &str, member: &str, owning_lead: &mut HashMap<String, String>) -> Option<ValidationError> {
    owning_lead
        .get(member)
        .map(|first_lead| ValidationError {
            field: "hierarchy.teams.members",
            reason: format!("{member} already sits under {first_lead}"),
        })
        .or_else(|| {
            owning_lead.insert(member.to_owned(), lead.to_owned());
            None
        })
}
