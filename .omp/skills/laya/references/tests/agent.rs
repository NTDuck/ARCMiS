//! End-to-end runtime behaviour, mirroring upstream's `tests/test_runtime.py`.

use candle_core::DType;
use laya::{Agent, AgentBuilder};
use serde_json::{json, Value};
use std::path::PathBuf;

fn checkpoint() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny")
}

fn agent() -> Agent {
    AgentBuilder::new()
        .dtype(DType::F32)
        .build(checkpoint())
        .expect("tiny checkpoint loads")
}

fn questions() -> Value {
    json!({
        "topic": {"type": "choice", "instructions": "choose", "criteria": ["a", "b", "c"]},
        "level": {"type": "score", "instructions": "level", "criteria": ["low", "high"]},
        "yes": {"type": "noul", "instructions": "is this true"}
    })
}

#[test]
fn answers_all_three_question_types() {
    let result = agent()
        .predict(&json!({"text": "hello world"}), &questions())
        .expect("prediction");

    assert_eq!(result.answers.len(), 3);
    assert_eq!(result.output_tokens, 0);
    assert!(result.input_tokens > 0);

    let topic = result.answer("topic").unwrap();
    assert_eq!(topic.kind, laya::QuestionKind::Choice);
    let choice = topic.choice.clone().unwrap();
    assert!(["a", "b", "c"].contains(&choice.as_str()));
    let total: f64 = topic.probabilities.iter().sum();
    assert!((total - 1.0).abs() < 1e-3, "probabilities sum to {total}");

    let level = result.answer("level").unwrap();
    let score = level.score.unwrap();
    assert!((0.0..=1.0).contains(&score), "score {score} out of range");

    let yes = result.answer("yes").unwrap();
    let noul = yes.noul.unwrap();
    assert!((0.0..=1.0).contains(&noul), "noul {noul} out of range");
    // Upstream overwrites the entropy confidence for noul.
    assert!((yes.confidence - noul.max(1.0 - noul)).abs() < 1e-9);
}

#[test]
fn batching_does_not_change_results() {
    let state = json!({"text": "hello world"});
    let together = AgentBuilder::new()
        .dtype(DType::F32)
        .batch_size(16)
        .build(checkpoint())
        .unwrap()
        .predict(&state, &questions())
        .unwrap();
    let separate = AgentBuilder::new()
        .dtype(DType::F32)
        .batch_size(1)
        .build(checkpoint())
        .unwrap()
        .predict(&state, &questions())
        .unwrap();

    assert_eq!(
        together.to_value(),
        separate.to_value(),
        "batch size changed the answers"
    );
}

#[test]
fn padding_to_a_multiple_does_not_change_results() {
    let state = json!("hello world hello");
    let plain = agent().predict(&state, &questions()).unwrap();
    let padded = AgentBuilder::new()
        .dtype(DType::F32)
        .pad_to_multiple(Some(16))
        .build(checkpoint())
        .unwrap()
        .predict(&state, &questions())
        .unwrap();

    // Padded positions are masked out, so answers must be bit-identical after
    // the four-decimal rounding upstream applies.
    assert_eq!(plain.to_value(), padded.to_value());
}

#[test]
fn prefix_cache_does_not_change_results() {
    let state = json!("hello world");
    let plain = agent().predict(&state, &questions()).unwrap();
    let cached_agent = AgentBuilder::new()
        .dtype(DType::F32)
        .cache_prompts(true)
        .build(checkpoint())
        .unwrap();

    // Run twice: the second call is served from the cache.
    let first = cached_agent.predict(&state, &questions()).unwrap();
    let second = cached_agent.predict(&state, &questions()).unwrap();

    assert_eq!(plain.to_value(), first.to_value());
    assert_eq!(first.to_value(), second.to_value());
}

#[test]
fn empty_request_is_handled() {
    let result = agent().predict(&json!(""), &json!({})).unwrap();
    assert!(result.answers.is_empty());
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
}

#[test]
fn single_option_choice_is_certain() {
    let result = agent()
        .predict(
            &json!("hello"),
            &json!({"one": {"type": "choice", "instructions": "choose", "criteria": ["a"]}}),
        )
        .unwrap();
    let answer = result.answer("one").unwrap();
    assert_eq!(answer.probabilities, vec![1.0]);
    assert_eq!(answer.choice.as_deref(), Some("a"));
    assert_eq!(answer.confidence, 1.0);
}

#[test]
fn long_states_are_truncated_to_max_len() {
    let long = "hello ".repeat(1000);
    let result = agent()
        .predict(
            &json!(long),
            &json!({"one": {"type": "choice", "instructions": "choose", "criteria": ["a"]}}),
        )
        .unwrap();
    // max_len in the tiny config is 128.
    assert_eq!(result.input_tokens, 128);

    // input_tokens alone cannot say whether the state was cut, so the
    // prediction reports it directly.
    assert!(
        result.truncated,
        "a 1000-word state cannot fit in 128 tokens"
    );
    assert_eq!(result.sequence_lengths, vec![128]);
}

#[test]
fn short_states_are_not_flagged_as_truncated() {
    let result = agent()
        .predict(&json!({"text": "hello world"}), &questions())
        .unwrap();

    assert!(!result.truncated);
    assert_eq!(result.sequence_lengths.len(), 3);
    // Several short sequences summing above max_len must not look truncated.
    assert_eq!(
        result.sequence_lengths.iter().sum::<usize>(),
        result.input_tokens
    );
    assert!(result.sequence_lengths.iter().all(|len| *len < 128));
}

#[test]
fn usage_detail_is_opt_in_and_upstream_shape_is_unchanged() {
    let result = agent()
        .predict(&json!({"text": "hello world"}), &questions())
        .unwrap();

    // Upstream emits exactly these two keys; adding to them here would be a
    // silent divergence, so the default rendering is pinned.
    let usage = result.to_value();
    let keys: Vec<&str> = usage["usage"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["input_tokens", "output_tokens"]);

    let detailed = result.to_value_detailed();
    assert_eq!(detailed["usage"]["truncated"], json!(false));
    assert_eq!(
        detailed["usage"]["sequence_lengths"],
        json!(result.sequence_lengths)
    );
    // Everything else is identical to the default rendering.
    assert_eq!(detailed["answers"], usage["answers"]);
    assert_eq!(
        detailed["usage"]["input_tokens"],
        usage["usage"]["input_tokens"]
    );
}

#[test]
fn result_matches_upstream_json_shape() {
    let result = agent().predict(&json!("hello"), &questions()).unwrap();
    let value = result.to_value();

    assert_eq!(value["model"], json!("laya-rl-agent"));
    assert_eq!(value["usage"]["output_tokens"], json!(0));

    // Key order matters: upstream emits type, confidence, action, then the
    // type-specific fields.
    let topic: Vec<&str> = value["answers"]["topic"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        topic,
        ["type", "confidence", "action", "choice", "probabilities"]
    );

    let level: Vec<&str> = value["answers"]["level"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        level,
        [
            "type",
            "confidence",
            "action",
            "score",
            "legend",
            "probabilities"
        ]
    );

    let yes: Vec<&str> = value["answers"]["yes"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(yes, ["type", "confidence", "action", "noul"]);

    assert!(value["answers"]["topic"]["action"]["act_probability"].is_number());
    assert_eq!(
        value["answers"]["level"]["legend"],
        json!({"0": "low", "1": "high"})
    );
}

#[test]
fn question_order_is_preserved() {
    let ordered = json!({
        "zebra": {"type": "noul", "instructions": "a"},
        "alpha": {"type": "noul", "instructions": "b"},
        "middle": {"type": "noul", "instructions": "c"}
    });
    let result = agent().predict(&json!("hello"), &ordered).unwrap();
    let keys: Vec<&str> = result.answers.keys().map(String::as_str).collect();
    assert_eq!(keys, ["zebra", "alpha", "middle"]);
}

#[test]
fn invalid_questions_are_rejected() {
    let agent = agent();
    for bad in [
        json!({"q": {"type": "nope", "instructions": "x"}}),
        json!({"q": {"type": "choice", "instructions": "x"}}),
        json!({"q": {"type": "choice", "instructions": "x", "criteria": ["a", "a"]}}),
        json!({"q": {"type": "score", "instructions": "x", "criteria": []}}),
        json!({"q": {"type": "noul"}}),
    ] {
        assert!(
            agent.predict(&json!("hello"), &bad).is_err(),
            "expected rejection for {bad}"
        );
    }
}

#[test]
fn mask_tokens_in_user_text_do_not_create_markers() {
    // A literal mask token in the state must not be mistaken for an option
    // marker; upstream replaces it with a space.
    let result = agent()
        .predict(
            &json!("hello [MASK] world"),
            &json!({"one": {"type": "choice", "instructions": "choose", "criteria": ["a", "b"]}}),
        )
        .unwrap();
    assert_eq!(result.answer("one").unwrap().probabilities.len(), 2);
}

#[test]
fn converted_checkpoints_round_trip() {
    let before = agent().predict(&json!("hello"), &questions()).unwrap();

    let output = std::env::temp_dir().join("laya-rs-convert-round-trip");
    let _ = std::fs::remove_dir_all(&output);
    laya::convert::convert(&checkpoint(), &output, DType::F32).expect("convert");

    let after = AgentBuilder::new()
        .dtype(DType::F32)
        .build(&output)
        .expect("converted checkpoint loads")
        .predict(&json!("hello"), &questions())
        .unwrap();
    assert_eq!(before.to_value(), after.to_value());

    // A second conversion must refuse rather than overwrite.
    assert!(laya::convert::convert(&checkpoint(), &output, DType::F32).is_err());
    let _ = std::fs::remove_dir_all(&output);
}

#[test]
fn incomplete_checkpoints_are_rejected() {
    let dir = std::env::temp_dir().join("laya-rs-missing-checkpoint");
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Agent::from_dir(&dir).is_err());
}
