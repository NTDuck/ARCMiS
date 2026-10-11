//! Export a standalone checkpoint with canonical parameter names and a dtype.

use candle_core::{DType, Device, Tensor};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::weights::sanitize_name;
use crate::{Error, Result};

/// Convert a checkpoint to canonical parameter names and the requested dtype.
///
/// The destination must not already exist; a partial directory is removed if
/// the export fails, so an existing checkpoint is never left half-overwritten.
pub fn convert(source: &Path, output: &Path, dtype: DType) -> Result<PathBuf> {
    if output.exists() {
        return Err(Error::Config(format!(
            "Output already exists: {}",
            output.display()
        )));
    }
    for name in [
        "model.safetensors",
        "rl_agent_config.json",
        "encoder/config.json",
    ] {
        let path = source.join(name);
        if !path.is_file() {
            return Err(Error::IncompleteCheckpoint(path));
        }
    }

    let result = convert_inner(source, output, dtype);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(output);
    }
    result
}

fn convert_inner(source: &Path, output: &Path, dtype: DType) -> Result<PathBuf> {
    std::fs::create_dir_all(output.join("encoder"))?;
    std::fs::create_dir_all(output.join("tokenizer"))?;

    for name in [
        "tokenizer/tokenizer.json",
        "tokenizer/tokenizer_config.json",
    ] {
        let from = source.join(name);
        if from.is_file() {
            std::fs::copy(from, output.join(name))?;
        }
    }
    for name in ["encoder/config.json", "rl_agent_config.json"] {
        std::fs::copy(source.join(name), output.join(name))?;
    }

    let device = Device::Cpu;
    let mmaped = unsafe {
        candle_core::safetensors::MmapedSafetensors::new(source.join("model.safetensors"))?
    };
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    for (name, _) in mmaped.tensors() {
        let tensor = mmaped.load(&name, &device)?.to_dtype(dtype)?;
        let canonical = sanitize_name(&name);
        if tensors.insert(canonical.clone(), tensor).is_some() {
            return Err(Error::Config(format!(
                "Duplicate checkpoint parameter after conversion: {canonical}"
            )));
        }
    }
    candle_core::safetensors::save(&tensors, output.join("model.safetensors"))?;

    let metadata: Value = json!({
        "format": "laya-rs",
        "format_version": 1,
        "dtype": dtype_name(dtype),
        "source": source.display().to_string(),
    });
    std::fs::write(
        output.join("laya_config.json"),
        format!("{}\n", serde_json::to_string_pretty(&metadata)?),
    )?;

    Ok(output.to_path_buf())
}

fn dtype_name(dtype: DType) -> &'static str {
    match dtype {
        DType::F32 => "float32",
        DType::F16 => "float16",
        DType::BF16 => "bfloat16",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny")
    }

    #[test]
    fn refuses_to_overwrite() {
        let existing = fixture();
        assert!(convert(&existing, &existing, DType::F16).is_err());
    }

    #[test]
    fn missing_checkpoints_are_rejected() {
        let missing = std::env::temp_dir().join("laya-rs-convert-missing-source");
        std::fs::create_dir_all(&missing).unwrap();
        let out = std::env::temp_dir().join("laya-rs-convert-missing-out");
        let _ = std::fs::remove_dir_all(&out);
        assert!(convert(&missing, &out, DType::F16).is_err());
    }
}
