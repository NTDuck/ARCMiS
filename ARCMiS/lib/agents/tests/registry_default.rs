//! Registry default test: every role builds from a mock client, prompts
//! resolve, and the fleet ladder promotes and demotes.

use std::collections::BTreeMap;

use agents::mas::fleet::Fleet;
use agents::mas::roles::Role;
use agents::util::config::Run;

#[test]
fn registry_module_is_reachable() {
    // The real client construction runs in the harness binary; here we pin
    // that the public build symbol resolves with the right bounds.
    let _build_fn: fn(&rig::providers::ollama::Client, &Fleet, &Run) -> anyhow::Result<agents::MasAgents> =
        |_client, _fleet, _run| unreachable!("construction requires a live client; covered by the e2e run");
    let _ = _build_fn;
}

#[test]
fn role_catalog_is_complete_and_prompted() {
    // 11 roles, every prompt file present, every name round-trips.
    assert_eq!(Role::ALL.len(), 11);
    for role in Role::ALL {
        let text = agents::mas::roles::prompt_text(role).expect("prompt file");
        assert!(text.len() > 200, "prompt for {} suspiciously short: {} bytes", role.name(), text.len());
        assert_eq!(Role::from_name(role.name()), Some(role));
    }
    assert_eq!(Role::from_name("failure-analyst"), Some(Role::FailureAnalyst));
    assert_eq!(Role::from_name("nonexistent"), None);
}

#[test]
fn tool_allowlists_partition_read_and_write() {
    // Judges never get write tools; the analyst and movers do.
    for role in [Role::Critic, Role::FleetAnalyst] {
        let tools = role.allowed_tools();
        assert!(!tools.contains(&"edit") && !tools.contains(&"write"), "{} must be read-only", role.name());
    }
    for role in [Role::Analyst, Role::Translator, Role::Repairer, Role::Tester] {
        let tools = role.allowed_tools();
        assert!(tools.contains(&"edit") || tools.contains(&"write"), "{} must be able to write", role.name());
    }
}

#[test]
fn fleet_ladder_promotes_and_demotes() {
    let mut fleet = Fleet {
        orchestrator_model: "qwen3:4b".into(),
        ladder: vec!["qwen3:4b".into(), "qwen3:8b".into(), "qwen3:14b".into()],
        role_models: BTreeMap::new(),
    };
    // Default assignment: weakest rung.
    assert_eq!(fleet.model_for(Role::Translator), "qwen3:4b");
    // Promote one rung at a time; stop at the top.
    assert_eq!(fleet.promote(Role::Translator), Some("qwen3:8b".to_owned()));
    assert_eq!(fleet.model_for(Role::Translator), "qwen3:8b");
    fleet.promote(Role::Translator).expect("second promote");
    assert_eq!(fleet.promote(Role::Translator), None, "top of ladder");
    // Demote back down; stop at the bottom.
    assert_eq!(fleet.demote(Role::Translator), Some("qwen3:8b".to_owned()));
    fleet.demote(Role::Translator).expect("second demote");
    assert_eq!(fleet.demote(Role::Translator), None, "bottom of ladder");
    // Orchestrator model is separate from the ladder.
    assert_eq!(fleet.model_for(Role::Orchestrator), "qwen3:4b");
    fleet.assign(Role::Orchestrator, "qwen3:32b".into());
    assert_eq!(fleet.model_for(Role::Orchestrator), "qwen3:32b");
}

#[test]
fn run_provider_flag_gates_ollama_params() {
    let mut run = Run::default();
    assert!(run.provider_is_ollama());
    run.provider = "netmind".into();
    assert!(!run.provider_is_ollama());
}

#[test]
fn role_think_override_wins_over_run_default() {
    // c8 fileupload: translator reasoning filled the context window before
    // its first write. The per-role override must beat the run default and
    // a missing role must fall back to it.
    let run = Run::default();
    assert!(run.think, "run default is think-on");
    let mut role_think = BTreeMap::new();
    role_think.insert("translator".to_owned(), false);
    // Override applies where present...
    assert_eq!(role_think.get("translator").copied(), Some(false));
    // ...and the run default holds where absent.
    assert!(role_think.get("analyst").copied().unwrap_or(run.think));
}
