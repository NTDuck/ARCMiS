//! Schema for a test report artifact.

use crate::artifact_content::ArtifactContent;
use crate::schema::Schema;

/// Schema for a test report artifact. Accepts non-empty text.
pub struct TestReportSchema;

impl Schema for TestReportSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            _ => Err("test report must be non-empty text".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "test-report"
    }
}
