//! `todo` maintains a phased task list across tool calls.
//!
//! State is a vector of phases on the tool struct, each phase holding ordered
//! items. Ops run in order. Any failed op adds a line to the errors prefix of
//! the returned tree. `init` replaces the whole list. `start` marks one item
//! in progress and demotes every other in-progress item to pending.

/// `todo` applies list edits and returns the tree summary.
pub struct Todo {
    /// Current phases in list order.
    pub phases: ::std::sync::Arc<::std::sync::Mutex<::std::vec::Vec<TodoPhase>>>,
}

impl ::rig::tool::Tool for Todo {
    const NAME: &'static str = "todo";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = TodoArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Apply edits to the phased task list and return the tree summary.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
            "type": "object",
            "properties": {
                "ops": {
                    "type": "array",
                    "description": "Edits to apply in order",
                    "items": {
                        "type": "object",
                        "properties": {
                            "op": { "type": "string", "enum": ["init", "start", "done", "drop", "rm", "append", "note"], "description": "Operation name" },
                            "task": { "type": "string", "description": "Task content to target" },
                            "phase": { "type": "string", "description": "Phase name to target" },
                            "items": { "type": "array", "items": { "type": "string" }, "description": "Task list for init" },
                            "list": { "type": "boolean", "description": "Drop or remove the whole list" },
                            "text": { "type": "string", "description": "Note text for the note op" }
                        },
                        "required": ["op"]
                    }
                }
            },
            "required": ["ops"]
        })
    }

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        if args.ops.is_empty() {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(
                "ops must hold at least one entry",
            ));
        }
        let mut phases = lock_phases(&self.phases)?;
        let mut errors = ::std::vec::Vec::new();
        for step in &args.ops {
            apply_op(&mut phases, step, &mut errors);
        }
        let mut output = render_tree(&phases);
        if !errors.is_empty() {
            output.insert_str(0, &::std::format!("errors:\n{}\n\n", errors.join("\n")));
        }
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(output))
    }
}

/// Arguments for `todo`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct TodoArgs {
    pub ops: ::std::vec::Vec<TodoOp>,
}

/// One list edit.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct TodoOp {
    pub op: ::std::string::String,
    #[serde(default)]
    pub task: ::core::option::Option<::std::string::String>,
    #[serde(default)]
    pub phase: ::core::option::Option<::std::string::String>,
    #[serde(default)]
    pub items: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
    #[serde(default)]
    pub list: ::core::option::Option<bool>,
    #[serde(default)]
    pub text: ::core::option::Option<::std::string::String>,
}

/// One phase holding ordered items.
#[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::default::Default)]
pub struct TodoPhase {
    /// Phase name.
    pub name: ::std::string::String,
    /// Items in list order.
    pub tasks: ::std::vec::Vec<TodoItem>,
}

/// One task with its status and notes.
#[derive(::core::fmt::Debug, ::core::clone::Clone)]
pub struct TodoItem {
    /// Task content.
    pub content: ::std::string::String,
    /// pending, in_progress, completed, or abandoned.
    pub status: ::std::string::String,
    /// Notes attached to this task.
    pub notes: ::std::vec::Vec<::std::string::String>,
}

/// Apply one op. A failed op records its message and leaves state unchanged.
fn apply_op(
    phases: &mut ::std::vec::Vec<TodoPhase>,
    step: &TodoOp,
    errors: &mut ::std::vec::Vec<::std::string::String>,
) {
    match step.op.as_str() {
        "init" => op_init(phases, step, errors),
        "start" => op_start(phases, step, errors),
        "done" | "drop" | "rm" => op_settle(phases, step, errors),
        "append" => op_append(phases, step, errors),
        "note" => op_note(phases, step, errors),
        other => errors.push(::std::format!("unknown op \"{other}\"")),
    }
}

/// Replace the whole list with the given items as one pending phase.
fn op_init(
    phases: &mut ::std::vec::Vec<TodoPhase>,
    step: &TodoOp,
    errors: &mut ::std::vec::Vec<::std::string::String>,
) {
    let items = step.items.as_deref().unwrap_or_default();
    if items.is_empty() {
        errors.push(::std::string::String::from("init needs a non-empty \"items\" list"));
        return;
    }
    let phase_name = step.phase.clone().unwrap_or_else(|| ::std::string::String::from("default"));
    *phases = ::std::vec![TodoPhase {
        tasks: items
            .iter()
            .map(|content| TodoItem {
                content: ::std::clone::Clone::clone(content),
                status: ::std::string::String::from("pending"),
                notes: ::std::vec::Vec::new(),
            })
            .collect::<::std::vec::Vec<_>>(),
        name: phase_name,
    }];
}

/// Mark one task in progress and demote other in-progress tasks to pending.
fn op_start(
    phases: &mut ::std::vec::Vec<TodoPhase>,
    step: &TodoOp,
    errors: &mut ::std::vec::Vec<::std::string::String>,
) {
    let target = step.task.as_deref().unwrap_or_default();
    match find_mut(phases, target) {
        ::core::option::Option::None => errors.push(::std::format!("task \"{target}\" not found")),
        ::core::option::Option::Some(_) => {
            for phase in phases.iter_mut() {
                for item in phase.tasks.iter_mut() {
                    if item.content == target {
                        item.status = ::std::string::String::from("in_progress");
                    } else if item.status == "in_progress" {
                        item.status = ::std::string::String::from("pending");
                    }
                }
            }
        },
    }
}

/// Mark tasks done, dropped, or removed.
fn op_settle(
    phases: &mut ::std::vec::Vec<TodoPhase>,
    step: &TodoOp,
    errors: &mut ::std::vec::Vec<::std::string::String>,
) {
    let new_status = match step.op.as_str() {
        "done" => "completed",
        "drop" => "abandoned",
        _ => "",
    };
    let whole_list = step.list.unwrap_or(false);
    if whole_list {
        if step.op == "rm" {
            phases.clear();
        } else {
            for phase in phases.iter_mut() {
                for item in phase.tasks.iter_mut() {
                    item.status = ::std::string::String::from(new_status);
                }
            }
        }
        return;
    }
    let target = step.task.as_deref().unwrap_or_default();
    if step.phase.is_none() && target.is_empty() {
        errors.push(::std::format!("{} needs a \"task\" or \"list\": true", step.op));
        return;
    }
    let phase_name = step.phase.as_deref();
    match find_mut_scoped(phases, phase_name, target) {
        ::core::option::Option::None => errors.push(::std::format!("task \"{target}\" not found")),
        ::core::option::Option::Some(_) => {
            for phase in phases.iter_mut() {
                if let ::core::option::Option::Some(name) = phase_name {
                    if phase.name != name {
                        continue;
                    }
                }
                phase.tasks.retain_mut(|item| {
                    let hit = item.content == target || target.is_empty();
                    if !hit {
                        return true;
                    }
                    if step.op == "rm" {
                        return false;
                    }
                    item.status = ::std::string::String::from(new_status);
                    true
                });
            }
        },
    }
}

/// Append one task. The tool creates the phase when it misses it. The tool rejects duplicates.
fn op_append(
    phases: &mut ::std::vec::Vec<TodoPhase>,
    step: &TodoOp,
    errors: &mut ::std::vec::Vec<::std::string::String>,
) {
    let target = step.task.as_deref().unwrap_or_default();
    if target.is_empty() {
        errors.push(::std::string::String::from("append needs a \"task\""));
        return;
    }
    let phase_name = step.phase.clone().unwrap_or_else(|| ::std::string::String::from("default"));
    if find_mut(phases, target).is_some() {
        errors.push(::std::format!("Task \"{target}\" already exists"));
        return;
    }
    let phase = phases.iter_mut().find(|phase| phase.name == phase_name);
    match phase {
        ::core::option::Option::Some(phase) => phase.tasks.push(new_item(target)),
        ::core::option::Option::None => phases.push(TodoPhase {
            name: phase_name,
            tasks: ::std::vec![new_item(target)],
        }),
    }
}

/// Append one note to one task.
fn op_note(
    phases: &mut ::std::vec::Vec<TodoPhase>,
    step: &TodoOp,
    errors: &mut ::std::vec::Vec<::std::string::String>,
) {
    let target = step.task.as_deref().unwrap_or_default();
    let note_text = step.text.as_deref().unwrap_or_default();
    if target.is_empty() || note_text.is_empty() {
        errors.push(::std::string::String::from("note needs a \"task\" and a \"text\""));
        return;
    }
    match find_mut(phases, target) {
        ::core::option::Option::None => errors.push(::std::format!("task \"{target}\" not found")),
        ::core::option::Option::Some(item) => item.notes.push(::std::string::String::from(note_text)),
    }
}

/// Find one item by content across every phase.
fn find_mut<'list>(phases: &'list mut [TodoPhase], target: &str) -> ::core::option::Option<&'list mut TodoItem> {
    phases.iter_mut().flat_map(|phase| phase.tasks.iter_mut()).find(|item| item.content == target)
}

/// Find one item. When the caller passes a phase name, the tool searches only that phase.
fn find_mut_scoped<'list>(
    phases: &'list mut [TodoPhase],
    phase_name: ::core::option::Option<&str>,
    target: &str,
) -> ::core::option::Option<&'list mut TodoItem> {
    phases
        .iter_mut()
        .filter(|phase| phase_name.is_none_or(|name| phase.name == name))
        .flat_map(|phase| phase.tasks.iter_mut())
        .find(|item| item.content == target)
}

/// Build one pending item.
fn new_item(content: &str) -> TodoItem {
    TodoItem {
        content: ::std::string::String::from(content),
        status: ::std::string::String::from("pending"),
        notes: ::std::vec::Vec::new(),
    }
}

/// Render phases and items as an indented tree with status marks.
fn render_tree(phases: &[TodoPhase]) -> ::std::string::String {
    if phases.is_empty() {
        return ::std::string::String::from("todo list is empty");
    }
    let mut lines = ::std::vec::Vec::new();
    for phase in phases {
        lines.push(::std::format!("## {}", phase.name));
        for item in &phase.tasks {
            let mark = match item.status.as_str() {
                "in_progress" => "▶",
                "completed" => "✓",
                "abandoned" => "✗",
                _ => "·",
            };
            lines.push(::std::format!("  {mark} {}", item.content));
            for note in &item.notes {
                lines.push(::std::format!("      note: {note}"));
            }
        }
    }
    lines.join("\n")
}

/// Lock the phase list. A poisoned lock returns an execution error.
fn lock_phases(
    phases: &::std::sync::Mutex<::std::vec::Vec<TodoPhase>>,
) -> ::core::result::Result<::std::sync::MutexGuard<'_, ::std::vec::Vec<TodoPhase>>, ::rig::tool::ToolExecutionError> {
    phases
        .lock()
        .map_err(|error| ::rig::tool::ToolExecutionError::other(::std::format!("todo state lock failed: {error}")))
}
