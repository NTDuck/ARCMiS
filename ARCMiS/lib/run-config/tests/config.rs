//! Behavior tests against the example config and rejection cases. Lives in
//! `tests/` per `.omp/rules/README.md` and ADR 0006.

use std::path::PathBuf;

use run_config::parse_and_validate;
use run_config::ValidationError;

/// Location of the checked-in example config, resolved relative to this
/// test file (`<crate>/tests/config.rs` -> `<repo root>/configs/example.yml`).
fn example_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../configs/example.yml")
        .canonicalize()
        .expect("configs/example.yml must exist next to mise.toml")
}

fn validate_example() -> Vec<ValidationError> {
    let yaml_text = std::fs::read_to_string(example_config_path()).expect("example config reads");
    let (_, findings) = parse_and_validate(&yaml_text).unwrap_or_else(|error| {
        panic!("deserialization failed: {}", error);
    });
    findings
}

fn minimal_yaml() -> String {
    format!(
        r#"
run:
  name: behavior-test-run
paths:
  input_dir: {input_dir}
  output_dir: {output_dir}
languages:
  pair:
    source: {source}
    target: {target}
  test_command: {test_command}
"#,
        input_dir = "some/input",
        output_dir = "some/output",
        source = "c",
        target = "rust",
        test_command = "cargo test",
    )
}

#[test]
fn example_config_deserializes_and_passes_preflight() {
    let findings = validate_example();
    assert!(findings.is_empty(), "{:?}", findings);
}

#[test]
fn example_config_declares_expected_sections() {
    let yaml_text = std::fs::read_to_string(example_config_path().with_file_name("example.yml")).expect("reads");
    assert!(yaml_text.contains("input_dir"), "expected `paths.input_dir`");
    assert!(yaml_text.contains("output_dir"), "expected `paths.output_dir`");
    assert!(yaml_text.contains("languages:"), "expected `languages` section");
    assert!(yaml_text.contains("model:"), "expected `model` section");
    assert!(yaml_text.contains("hierarchy:"), "expected `hierarchy` section");
    assert!(yaml_text.contains("laya:"), "expected `laya` section");
}

#[test]
fn missing_run_name_reports_actionable_error() {
    let yaml_text = minimal_yaml().replace("behavior-test-run", " ");
    let findings = validate_yaml(yaml_text);

    assert!(
        findings.iter().any(|finding| finding.field == "run.name"),
        "expected run.name to be flagged: {:?}",
        findings
    );
}

#[test]
fn blank_paths_are_rejected() {
    let yaml_text = minimal_yaml().replace("some/input", "");

    let findings = validate_yaml(yaml_text);

    assert!(
        findings.iter().any(|finding| finding.field == "paths.input_dir"),
        "expected paths.input_dir to be flagged"
    );
}

#[test]
fn valid_minimal_config_has_no_findings() {
    let findings = validate_yaml(minimal_yaml());

    assert!(findings.is_empty(), "{:?}", findings);
}

#[test]
fn member_in_two_teams_is_an_error() {
    let yaml_text = format!(
        r#"
run:
  name: team-split-test
paths:
  input_dir: {input_dir}
  output_dir: {output_dir}
languages:
  pair:
    source: {source}
    target: {target}
  test_command: {test_command}
hierarchy:
  teams:
    - lead: alpha-lead
      members: [uplift-worker, transform-worker]
      turns: 4
      stagnation_rounds: 2
    - lead: beta-lead
      members: [reimagine-worker, uplift-worker]
      turns: 4
      stagnation_rounds: 2
"#,
        input_dir = "input",
        output_dir = "output",
        source = "java",
        target = "rust",
        test_command = "cargo test",
    );

    let findings = validate_yaml(yaml_text);

    assert_eq!(
        findings.iter().filter(|f| f.field == "hierarchy.teams.members").count(),
        1,
        "expected exactly one hierarchy finding, got {:?}",
        findings
    );
    assert!(findings[0].reason.contains("uplift-worker"), "expected shared member name in reason");
}

#[test]
fn member_under_one_lead_has_no_hierarchy_findings() {
    let yaml_text = r#"
run:
  name: team-split-ok
paths:
  input_dir: input
  output_dir: output
languages:
  pair:
    source: c
    target: rust
  test_command: cargo test
hierarchy:
  teams:
    - lead: alpha-lead
      members: [analyst, architect]
      turns: 4
      stagnation_rounds: 2
    - lead: beta-lead
      members: [translator, validator]
      turns: 4
      stagnation_rounds: 2
"#
    .to_string();

    let findings = validate_yaml(yaml_text);

    assert!(findings.is_empty(), "{:?}", findings);
}

fn validate_yaml(yaml_text: String) -> Vec<ValidationError> {
    parse_and_validate(&yaml_text).map(|(_config, findings)| findings).unwrap_or_default()
}

#[test]
fn unknown_top_level_section_is_a_deserialization_error() {
    let yaml_text = minimal_yaml() + "\ncommand_center:\n  enabled: true\n";

    assert!(parse_and_validate(&yaml_text).is_err(), "expected unknown top-level section to fail deserialization");
}

#[test]
fn unknown_nested_key_is_a_deserialization_error() {
    let yaml_text = minimal_yaml() + "\nlaya:\n  checkpointdir: assets/models\n";

    assert!(parse_and_validate(&yaml_text).is_err(), "expected unknown nested key to fail deserialization");
}

#[test]
fn omitted_optional_sections_take_defaults_with_zero_findings() {
    let findings = validate_yaml(minimal_yaml());

    assert!(findings.is_empty(), "{:?}", findings);
}
