//! `ask` presents structured questions to the harness user.
//!
//! This crate has no interactive UI. The tool validates the question set and
//! returns a rendered text form. The harness collects the answers and feeds
//! them back through its own channel.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use serde::Deserialize;
use std::collections::BTreeSet;

/// `ask` validates and renders a question set for the harness.
pub struct Ask;

impl Tool for Ask {
    const NAME: &'static str = "ask";
    type Error = ToolExecutionError;
    type Args = AskArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Validate a question set and render it as a text form for the harness.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "Questions to present",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Unique question id" },
                            "question": { "type": "string", "description": "Question text" },
                            "options": {
                                "type": "array",
                                "description": "Answer choices",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string", "description": "Choice label" }
                                    },
                                    "required": ["label"]
                                }
                            },
                            "multi": { "type": "boolean", "description": "Allow several answers" },
                            "recommended": { "type": "string", "description": "Recommended answer label" }
                        },
                        "required": ["id", "question", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate(&args.questions)?;
        Ok(ToolOutput::text(render(&args.questions)))
    }
}

/// Arguments for `ask`.
#[derive(Debug, Deserialize)]
pub struct AskArgs {
    pub questions: Vec<AskQuestion>,
}

/// One question with its choices.
#[derive(Debug, Deserialize)]
pub struct AskQuestion {
    pub id: String,
    pub question: String,
    pub options: Vec<AskOption>,
    #[serde(default, rename = "multi")]
    pub multi: Option<bool>,
    #[serde(default)]
    pub recommended: Option<String>,
}

/// One answer choice.
#[derive(Debug, Deserialize)]
pub struct AskOption {
    pub label: String,
}

/// Reject empty ids, empty questions, missing options, and duplicate ids.
fn validate(questions: &[AskQuestion]) -> Result<(), ToolExecutionError> {
    if questions.is_empty() {
        return Err(ToolExecutionError::invalid_args("questions must hold at least one entry"));
    }
    let mut seen = BTreeSet::new();
    for question in questions {
        if question.id.is_empty() {
            return Err(ToolExecutionError::invalid_args("question id must not be empty"));
        }
        if question.question.trim().is_empty() {
            return Err(ToolExecutionError::invalid_args(format!("question \"{}\" needs non-empty text", question.id)));
        }
        if question.options.is_empty() {
            return Err(ToolExecutionError::invalid_args(format!(
                "question \"{}\" needs at least one option",
                question.id
            )));
        }
        if question.options.iter().any(|option| option.label.trim().is_empty()) {
            return Err(ToolExecutionError::invalid_args(format!(
                "question \"{}\" has an option with an empty label",
                question.id
            )));
        }
        if !seen.insert(question.id.clone()) {
            return Err(ToolExecutionError::invalid_args(format!("duplicate question id \"{}\"", question.id)));
        }
    }
    Ok(())
}

/// Render the question set as a numbered text form.
fn render(questions: &[AskQuestion]) -> String {
    let mut lines =
        vec!["Answer these questions through the harness. This tool collects no answers itself.".to_owned()];
    for question in questions {
        lines.push(String::new());
        lines.push(format!("? {} ({})", question.question, question.id));
        if question.multi.unwrap_or(false) {
            lines.push("  choose one or more:".to_owned());
        } else {
            lines.push("  choose one:".to_owned());
        }
        for option in &question.options {
            let mark = if option.label == question.recommended.as_deref().unwrap_or_default() {
                " *"
            } else {
                ""
            };
            lines.push(format!("  - {}{mark}", option.label));
        }
        if question.recommended.is_some() {
            lines.push("  * marks the recommended answer".to_owned());
        }
    }
    lines.join("\n")
}
