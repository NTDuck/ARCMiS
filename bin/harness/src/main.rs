//! Binary entry point for ARCMiS. Drives the migration agent over a run
//! config, prints reasoning and tool-call logs, and reports measurement.

use ::arcmis_agents::config::Config;
use ::arcmis_agents::{measure, migration};
use ::rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction};

/// The run log. Tool calls surface through the rig hook; the model's final
/// text and the measurement land in the printed report and the output dir.
#[derive(Clone, Default)]
struct RunLog;

impl AgentHook for RunLog {
    async fn on_completion_call(&self, ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        ::std::println!("[turn {}] model call", ctx.turn());
        CompletionCallAction::Continue
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        ::std::println!("[turn {}] tool call: {}({})", ctx.turn(), event.tool_name, event.args);
        ToolCallAction::Run
    }
}

#[tokio::main]
async fn main() -> ::std::process::ExitCode {
    let args: Vec<_> = ::std::env::args().collect();
    let config_path = args
        .get(1)
        .map(::std::path::PathBuf::from)
        .unwrap_or_else(|| ::std::path::PathBuf::from("assets/configs/GildedRose-Refactoring-Kata/config.yml"));

    let started = ::std::time::Instant::now();
    let config = match Config::load(&config_path) {
        ::core::result::Result::Ok(c) => c,
        ::core::result::Result::Err(e) => {
            ::std::eprintln!("config error: {e}");
            return ::std::process::ExitCode::FAILURE;
        },
    };

    // The output dir must exist before the agent writes into it.
    if let ::core::result::Result::Err(e) = ::std::fs::create_dir_all(&config.output.dir) {
        ::std::eprintln!("output dir error: {e}");
        return ::std::process::ExitCode::FAILURE;
    }

    // Package skeleton from config: manifest setup is toolchain work, the
    // agent owns the translations.
    for (path, content) in &config.output.scaffold_files {
        let full = config.output.dir.join(path);
        if let ::core::result::Result::Err(e) = ::std::fs::create_dir_all(full.parent().expect("scaffold parent")) {
            ::std::eprintln!("scaffold dir error for {path}: {e}");
            return ::std::process::ExitCode::FAILURE;
        }
        if let ::core::result::Result::Err(e) = ::std::fs::write(&full, content) {
            ::std::eprintln!("scaffold write error for {path}: {e}");
            return ::std::process::ExitCode::FAILURE;
        }
    }

    ::std::println!(
        "=== migration run: model={}, max_turns={}, num_ctx={}",
        config.run.model,
        config.run.max_turns,
        config.run.num_ctx
    );

    // A 2B model is flaky: a turn can truncate, wander, or exhaust its turn
    // budget. Each attempt ends in a measurement of the output dir; the run
    // stops at the first compiling attempt (speed) and otherwise keeps the
    // best measurement. A budget exhaustion is a run end, not a fatal error:
    // whatever the agent wrote still gets measured.
    let mut best: ::core::option::Option<arcmis_agents::measure::Measurement> = ::core::option::Option::None;
    for attempt in 1..=config.run.max_retries + 1 {
        let final_text = match migration::run(&config, RunLog).await {
            ::core::result::Result::Ok(text) => text,
            ::core::result::Result::Err(
                e @ ::rig::completion::PromptError::MaxTurnsError {
                    ..
                },
            ) => {
                ::std::format!("run ended at the turn budget: {e}")
            },
            ::core::result::Result::Err(e) => {
                ::std::eprintln!("[attempt {attempt}] agent run failed: {e}");
                continue;
            },
        };
        ::std::println!("=== agent final output:\n{final_text}");

        let m = measure::measure(&config.output.dir, &config.source.target.test_command).await;
        ::std::println!(
            "=== attempt {attempt} measurement: compile={} test_pass_rate={:?}",
            m.compile,
            m.test_pass_rate
        );
        let done = m.compile == "pass";
        best = ::core::option::Option::Some(match best {
            ::core::option::Option::Some(b) if b.compile == "pass" => b,
            _ => m,
        });
        if done {
            break;
        }
    }
    let m = match best {
        ::core::option::Option::Some(m) => m,
        ::core::option::Option::None => {
            ::std::eprintln!("all attempts failed before measurement");
            return ::std::process::ExitCode::FAILURE;
        },
    };
    let record = ::std::format!(
        "# measurement\ncompile: {}\ntest_pass_rate: {:?}\nelapsed: {:?}\n",
        m.compile,
        m.test_pass_rate,
        started.elapsed()
    );
    let report_path = config.output.dir.join("run-report.md");
    let _ = ::std::fs::write(&report_path, &record);
    ::std::println!(
        "=== measurement: compile={} test_pass_rate={:?} elapsed={:?} report={}",
        m.compile,
        m.test_pass_rate,
        started.elapsed(),
        report_path.display()
    );

    match m.compile {
        "pass" => ::std::process::ExitCode::SUCCESS,
        _ => ::std::process::ExitCode::FAILURE,
    }
}
