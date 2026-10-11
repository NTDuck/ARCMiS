//! Numerical parity for the candle model.
//!
//! `tests/fixtures/model_cases.json` holds reference outputs from a NumPy
//! transcription of upstream's `laya_mlx/model.py`, run against a synthetic
//! checkpoint in `tests/fixtures/tiny`. That lets the encoder, decision head,
//! scorer and action head be checked numerically on any machine, without Apple
//! hardware, MLX, or the real 421M-parameter weights.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use laya::config::{AgentConfig, EncoderConfig};
use laya::model::DecisionModel;
use laya::weights::SanitizedBackend;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::LazyLock;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

static CASES: LazyLock<Value> = LazyLock::new(|| {
    let path = fixtures().join("model_cases.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "missing {}: {error}. Run tools/gen_model_fixture.py.",
            path.display()
        )
    });
    serde_json::from_str(&text).expect("valid fixture JSON")
});

fn load_model(dir: &str) -> (DecisionModel, Device) {
    let device = Device::Cpu;
    let root = fixtures().join(dir);

    let encoder_raw: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("encoder/config.json")).unwrap())
            .unwrap();
    let agent_raw: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("rl_agent_config.json")).unwrap())
            .unwrap();
    let encoder_cfg = EncoderConfig::from_value(&encoder_raw).expect("encoder config");
    let agent_cfg =
        AgentConfig::from_value(&agent_raw, encoder_cfg.max_position_embeddings).expect("config");

    let mmaped = unsafe {
        candle_core::safetensors::MmapedSafetensors::new(root.join("model.safetensors")).unwrap()
    };
    let names: Vec<String> = mmaped.tensors().into_iter().map(|(name, _)| name).collect();
    let backend = SanitizedBackend::new(Box::new(mmaped), names);
    let vb = VarBuilder::from_backend(Box::new(backend), DType::F32, device.clone());

    let model = DecisionModel::load(vb, &encoder_cfg, &agent_cfg, DType::F32, &device)
        .expect("model loads");
    (model, device)
}

fn rows_u32(case: &Value, key: &str) -> (Vec<u32>, usize, usize) {
    let rows = case[key].as_array().unwrap();
    let cols = rows[0].as_array().unwrap().len();
    let flat = rows
        .iter()
        .flat_map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as u32)
        })
        .collect();
    (flat, rows.len(), cols)
}

fn rows_i64(case: &Value, key: &str) -> (Vec<i64>, usize, usize) {
    let rows = case[key].as_array().unwrap();
    let cols = rows[0].as_array().unwrap().len();
    let flat = rows
        .iter()
        .flat_map(|row| row.as_array().unwrap().iter().map(|v| v.as_i64().unwrap()))
        .collect();
    (flat, rows.len(), cols)
}

fn rows_f64(case: &Value, key: &str) -> Vec<f64> {
    case[key]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()))
        .collect()
}

/// Run every fixture case and compare against the NumPy reference.
fn check_against_reference(dir: &str) {
    let (model, device) = load_model(dir);

    for case in CASES["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();

        let (ids, rows, seq) = rows_u32(case, "input_ids");
        let input_ids = Tensor::from_vec(ids, (rows, seq), &device).unwrap();
        let (mask, _, _) = rows_u32(case, "attention_mask");
        let attention_mask = Tensor::from_vec(
            mask.iter().map(|v| *v as u8).collect::<Vec<u8>>(),
            (rows, seq),
            &device,
        )
        .unwrap();
        let (pos, _, slots) = rows_i64(case, "marker_pos");
        let marker_pos = Tensor::from_vec(pos, (rows, slots), &device).unwrap();
        let (mmask, _, _) = rows_u32(case, "marker_mask");
        let marker_mask = Tensor::from_vec(
            mmask.iter().map(|v| *v as u8).collect::<Vec<u8>>(),
            (rows, slots),
            &device,
        )
        .unwrap();
        let qtype: Vec<u32> = case["qtype"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let qtype = Tensor::from_vec(qtype, rows, &device).unwrap();

        let output = model
            .forward(
                &input_ids,
                &attention_mask,
                &marker_pos,
                &marker_mask,
                &qtype,
            )
            .unwrap_or_else(|error| panic!("{name}: forward failed: {error}"));

        let logits: Vec<f32> = output.logits.flatten_all().unwrap().to_vec1().unwrap();
        let action: Vec<f32> = output.action.flatten_all().unwrap().to_vec1().unwrap();
        let expected_logits = rows_f64(case, "logits");
        let expected_action = rows_f64(case, "action");

        assert_eq!(logits.len(), expected_logits.len(), "{name}: logit count");
        assert_eq!(action.len(), expected_action.len(), "{name}: action count");

        for (index, (actual, wanted)) in logits.iter().zip(&expected_logits).enumerate() {
            let actual = *actual as f64;
            // The masked slots sit at exactly -1e4 in both implementations.
            let tolerance = if wanted.abs() > 1e3 { 1e-3 } else { 2e-4 };
            assert!(
                (actual - wanted).abs() <= tolerance * wanted.abs().max(1.0),
                "{name}: logit[{index}] = {actual}, reference {wanted}"
            );
        }
        for (index, (actual, wanted)) in action.iter().zip(&expected_action).enumerate() {
            let actual = *actual as f64;
            assert!(
                (actual - wanted).abs() <= 2e-4 * wanted.abs().max(1.0),
                "{name}: action[{index}] = {actual}, reference {wanted}"
            );
        }
    }
}

#[test]
fn candle_model_matches_numpy_reference() {
    check_against_reference("tiny");
}

#[test]
fn upstream_parameter_names_load_identically() {
    // The legacy checkpoint holds the same weights under PyTorch's original
    // names (`in_proj_weight`, `scorer.0.*`), which the sanitizer must map.
    check_against_reference("tiny-legacy");
}
