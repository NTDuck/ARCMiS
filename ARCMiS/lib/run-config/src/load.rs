//! Parse-then-validate bridge. One entry point couples YAML deserialization
//! to preflight validation, so callers cannot hold an unchecked config.

use crate::errors::ValidationError;
use crate::schema::SectionConfig;
use crate::validation::expect_valid;

/// Parses one YAML document and runs every preflight check on the result.
///
/// The returned pair keeps both halves visible. Deserialization errors say
/// where the YAML failed to bind. Findings list every rejected field. The
/// caller decides how to fail the run.
#[must_use = "validate the findings before starting a run"]
pub fn parse_and_validate(yaml_text: &str) -> anyhow::Result<(SectionConfig, Vec<ValidationError>)> {
    let config = serde_yaml::from_str::<SectionConfig>(yaml_text)?;
    let findings = expect_valid(&config);

    Ok((config, findings))
}
