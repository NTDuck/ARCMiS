//! Deterministic tool-call policy (ADR 0031).
//!
//! A tool call becomes an allow or deny verdict before it reaches the
//! transport. Deny patterns use the legacy `"<tool>:<substring>"` form
//! carried from the retired `guard.rs`. When both lists match, deny
//! wins: this follows the run-config GuardConfig contract and ADR 0031
//! over any task brief asking otherwise.

/// One tool call awaiting a verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Tool name, e.g. `bash` or `edit`.
    pub tool: String,
    /// Raw argument text.
    pub args: String,
}

/// Gateway verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolVerdict {
    /// The call may proceed.
    Allow,
    /// The gateway refused the call. Carries the matched pattern.
    Deny {
        /// Pattern that matched the call.
        matched: String,
    },
}

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
