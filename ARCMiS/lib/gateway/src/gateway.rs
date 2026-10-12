//! Deterministic call gateway.

use crate::tool_call::ToolCall;
use crate::tool_verdict::ToolVerdict;

/// Deterministic call gateway.
#[derive(Debug, Default)]
pub struct Gateway {
    /// Patterns in `"<tool>:<substr>"` form. A match denies the call.
    deny: Vec<String>,
    /// Patterns in the same form. A match allows a call the deny list
    /// does not cover. Empty means allow everything not denied.
    allow: Vec<String>,
}

impl Gateway {
    /// A gateway from explicit pattern lists. Order inside each list is
    /// irrelevant: deny dominance is structural, not positional.
    pub fn new(deny: Vec<String>, allow: Vec<String>) -> Self {
        Self {
            deny,
            allow,
        }
    }

    /// A gateway that denies nothing.
    pub fn permissive() -> Self {
        Self::new(Vec::new(), Vec::new())
    }

    /// Parse one `"<tool>:<substr>"` pattern into its parts.
    fn split(pattern: &str) -> Option<(&str, &str)> {
        let (tool, substr) = pattern.split_once(':')?;
        Some((tool.trim(), substr))
    }

    /// Whether one pattern list covers the call.
    fn covers(patterns: &[String], call: &ToolCall) -> Option<String> {
        patterns.iter().find_map(|pattern| {
            Self::split(pattern)
                .and_then(|(tool, needle)| (tool == call.tool && call.args.contains(needle)).then(|| pattern.clone()))
        })
    }

    /// Judge one call. Deny beats allow when both lists match.
    pub fn judge(&self, call: &ToolCall) -> ToolVerdict {
        if let Some(matched) = Self::covers(&self.deny, call) {
            return ToolVerdict::Deny {
                matched,
            };
        }
        if self.allow.is_empty() || Self::covers(&self.allow, call).is_some() {
            ToolVerdict::Allow
        } else {
            ToolVerdict::Deny {
                matched: "no allow pattern covers this call".to_owned(),
            }
        }
    }
}
