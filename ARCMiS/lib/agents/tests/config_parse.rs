//! Config parse test: the shipped MAS yml parses into the typed shape and
//! every section carries the values the run needs.

use agents::util::config::Config;

#[test]
fn mas_config_yml_parses() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../assets/configs/mas/config.yml"))
        .expect("config file");
    let config: Config = serde_yaml::from_str(&text).expect("parse");
    assert_eq!(config.run.provider, "ollama");
    assert_eq!(config.run.model, "recodeagent-sft");
    assert!(config.run.provider_is_ollama());
    assert_eq!(config.source.language, "c");
    assert_eq!(config.source.target.language, "rust");
    assert_eq!(config.mas.max_rounds, 60);
    assert_eq!(config.mas.max_repairs, 2);
    assert_eq!(config.mas.stagnation_rounds, 6);
    assert_eq!(config.mas.plan_cap, 4000);
    assert_eq!(config.mas.notes_cap, 8000);
    assert_eq!(config.mas.model_ladder.len(), 1);
    assert_eq!(config.mas.max_generated_tests_per_module, 4);
}

#[test]
fn mas_section_is_optional_for_other_methods() {
    // The existing per-problem configs carry no `mas:` and no `provider:`;
    // they must keep parsing.
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets/configs/GildedRose-Refactoring-Kata/config.yml"
    ))
    .expect("config file");
    let config: Config = serde_yaml::from_str(&text).expect("parse");
    assert_eq!(config.mas.max_rounds, 60, "defaults apply");
    assert_eq!(config.run.provider, "ollama", "default provider");
}
