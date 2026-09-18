//! `ask` presents structured questions to the harness user.
//!
//! This crate has no interactive UI. The tool validates the question set and
//! returns a rendered text form. The harness collects the answers and feeds
//! them back through its own channel.

/// `ask` validates and renders a question set for the harness.
pub struct Ask;

impl rig::tool::Tool for Ask {
    const NAME: &'static str = "ask";
    type Error = rig::tool::ToolExecutionError;
    type Args = AskArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> std::string::String {
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

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> core::result::Result<Self::Output, Self::Error> {
        validate(&args.questions)?;
        core::result::Result::Ok(rig::tool::ToolOutput::text(render(&args.questions)))
    }
}

/// Arguments for `ask`.
#[derive(Debug, serde::Deserialize)]
pub struct AskArgs {
    pub questions: std::vec::Vec<AskQuestion>,
}

/// One question with its choices.
#[derive(Debug, serde::Deserialize)]
pub struct AskQuestion {
    pub id: std::string::String,
    pub question: std::string::String,
    pub options: std::vec::Vec<AskOption>,
    #[serde(default, rename = "multi")]
    pub multi: core::option::Option<bool>,
    #[serde(default)]
    pub recommended: core::option::Option<std::string::String>,
}

/// One answer choice.
#[derive(Debug, serde::Deserialize)]
pub struct AskOption {
    pub label: std::string::String,
}

/// Reject empty ids, empty questions, missing options, and duplicate ids.
fn validate(questions: &[AskQuestion]) -> core::result::Result<(), rig::tool::ToolExecutionError> {
    if questions.is_empty() {
        return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(
            "questions must hold at least one entry",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for question in questions {
        if question.id.is_empty() {
            return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(
                "question id must not be empty",
            ));
        }
        if question.question.trim().is_empty() {
            return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(std::format!(
                "question \"{}\" needs non-empty text",
                question.id
            )));
        }
        if question.options.is_empty() {
            return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(std::format!(
                "question \"{}\" needs at least one option",
                question.id
            )));
        }
        if question.options.iter().any(|option| option.label.trim().is_empty()) {
            return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(std::format!(
                "question \"{}\" has an option with an empty label",
                question.id
            )));
        }
        if !seen.insert(question.id.clone()) {
            return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(std::format!(
                "duplicate question id \"{}\"",
                question.id
            )));
        }
    }
    core::result::Result::Ok(())
}

/// Render the question set as a numbered text form.
fn render(questions: &[AskQuestion]) -> std::string::String {
    let mut lines = std::vec![std::string::String::from(
        "Answer these questions through the harness. This tool collects no answers itself."
    ),];
    for question in questions {
        lines.push(std::string::String::new());
        lines.push(std::format!("? {} ({})", question.question, question.id));
        if question.multi.unwrap_or(false) {
            lines.push(std::string::String::from("  choose one or more:"));
        } else {
            lines.push(std::string::String::from("  choose one:"));
        }
        for option in &question.options {
            let mark = if option.label == question.recommended.as_deref().unwrap_or_default() {
                " *"
            } else {
                ""
            };
            lines.push(std::format!("  - {}{mark}", option.label));
        }
        if question.recommended.is_some() {
            lines.push(std::string::String::from("  * marks the recommended answer"));
        }
    }
    lines.join("\n")
}
