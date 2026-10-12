//! Integration tests for the conductor over a scripted transport.

use std::collections::VecDeque;
use std::time::Duration;

use mas_core::conductor::Conductor;
use mas_core::conductor::RunLimits;
use mas_core::conductor::Verdict;
use mas_core::error::ConductorError;
use mas_core::frames::Event;
use mas_core::frames::PromptResultFrame;
use mas_core::frames::ResponseFrame;
use mas_core::gateway::Gateway;
use mas_core::gateway::ToolCall;
use mas_core::hierarchy::TeamRegistry;
use mas_core::phase::Phase;
use mas_core::router::Router;
use mas_core::taskgraph::GraphBudgets;
use mas_core::taskgraph::WorkItem;
use mas_core::transport::Transport;
use mas_core::transport::TransportError;

/// A transport that answers each prompt with one success response frame,
/// then reports timeout so the drain loop ends.
#[derive(Default)]
struct ScriptedTransport {
    prompts: Vec<String>,
    /// Responses left to hand out before the stream times out.
    responses_left: usize,
}

impl ScriptedTransport {
    fn answering(responses: usize) -> Self {
        Self {
            prompts: Vec::new(),
            responses_left: responses,
        }
    }
}

impl Transport for ScriptedTransport {
    async fn send_prompt(&mut self, message: &str) -> Result<String, TransportError> {
        self.prompts.push(message.to_owned());
        Ok(format!("req-{}", self.prompts.len()))
    }

    async fn recv(&mut self) -> Result<Event, TransportError> {
        self.recv_within(Duration::ZERO).await
    }

    async fn recv_within(&mut self, _timeout: Duration) -> Result<Event, TransportError> {
        if self.responses_left == 0 {
            return Err(TransportError::Timeout);
        }
        self.responses_left -= 1;
        Ok(Event::Response(ResponseFrame {
            id: Some("req-1".to_owned()),
            command: None,
            success: true,
            data: Some(serde_json::json!("done")),
            error: None,
        }))
    }
}

/// A transport that replays one scripted frame sequence, then reports
/// timeout. It records every action so tests can assert ordering.
#[derive(Default)]
struct StreamTransport {
    prompts: Vec<String>,
    actions: Vec<String>,
    /// Frames left to hand out, oldest first.
    script: VecDeque<Result<Event, TransportError>>,
}

impl StreamTransport {
    fn replaying(script: Vec<Result<Event, TransportError>>) -> Self {
        Self {
            prompts: Vec::new(),
            actions: Vec::new(),
            script: script.into(),
        }
    }
}

impl Transport for StreamTransport {
    async fn send_prompt(&mut self, message: &str) -> Result<String, TransportError> {
        self.prompts.push(message.to_owned());
        self.actions.push(format!("send:{}", self.prompts.len()));
        Ok(format!("req-{}", self.prompts.len()))
    }

    async fn recv(&mut self) -> Result<Event, TransportError> {
        self.recv_within(Duration::ZERO).await
    }

    async fn recv_within(&mut self, _timeout: Duration) -> Result<Event, TransportError> {
        match self.script.pop_front() {
            Some(Ok(Event::SubagentLifecycle(frame))) => {
                self.actions.push(format!("lifecycle:{}:{}", frame.payload.agent, frame.payload.status));
                Ok(Event::SubagentLifecycle(frame))
            },
            Some(frame) => frame,
            None => Err(TransportError::Timeout),
        }
    }
}

/// One completed prompt result on the wire vocabulary.
fn completed_frame(id: &str) -> Event {
    Event::PromptResult(PromptResultFrame {
        id: Some(id.to_owned()),
        agent_invoked: Some(true),
        status: Some("completed".to_owned()),
        session_settled: Some(true),
    })
}

/// One failing prompt result on the wire vocabulary.
fn error_frame(id: &str) -> Event {
    Event::PromptResult(PromptResultFrame {
        id: Some(id.to_owned()),
        agent_invoked: Some(true),
        status: Some("error".to_owned()),
        session_settled: Some(true),
    })
}

/// One analyst work item with the given instruction.
fn analyst_item(instruction: &str) -> WorkItem {
    WorkItem {
        role: mas_core::hierarchy::Role::Analyst,
        kind: mas_core::taskgraph::Kind::Analyze,
        instruction: instruction.to_owned(),
    }
}

fn limits() -> RunLimits {
    RunLimits {
        graph: GraphBudgets {
            fanout: 2,
            max_repairs: 1,
            stagnation_rounds: 5,
        },
        context_budget: 10_000,
        event_timeout: Duration::from_millis(10),
    }
}

#[tokio::test]
async fn run_completes_through_the_phase_walk() {
    let transport = ScriptedTransport::answering(1);
    let mut conductor =
        Conductor::new(transport, TeamRegistry::direct(), Router::keyword_only(), Gateway::permissive(), limits());
    conductor.add_work(
        WorkItem {
            role: mas_core::hierarchy::Role::Analyst,
            kind: mas_core::taskgraph::Kind::Analyze,
            instruction: "map the source".to_owned(),
        },
        Vec::new(),
    );
    let summary = conductor.run().await.unwrap();
    assert_eq!(summary.verdict, Verdict::Success);
    assert_eq!(summary.final_phase, Phase::Done);
    assert!(summary.dispatches >= 1);
}

#[tokio::test]
async fn closed_transport_maps_to_transport_error() {
    struct ClosedTransport;

    impl Transport for ClosedTransport {
        async fn send_prompt(&mut self, _message: &str) -> Result<String, TransportError> {
            Err(TransportError::Closed)
        }

        async fn recv(&mut self) -> Result<Event, TransportError> {
            Err(TransportError::Closed)
        }

        async fn recv_within(&mut self, _timeout: Duration) -> Result<Event, TransportError> {
            Err(TransportError::Closed)
        }
    }

    let mut conductor = Conductor::new(
        ClosedTransport,
        TeamRegistry::direct(),
        Router::keyword_only(),
        Gateway::permissive(),
        limits(),
    );
    conductor.add_work(
        WorkItem {
            role: mas_core::hierarchy::Role::Analyst,
            kind: mas_core::taskgraph::Kind::Analyze,
            instruction: "map the source".to_owned(),
        },
        Vec::new(),
    );
    let error = conductor.run().await.unwrap_err();
    assert!(matches!(error, ConductorError::Transport(_)));
}

#[test]
fn gateway_sits_in_front_of_tool_calls() {
    let transport = ScriptedTransport::answering(0);
    let conductor = Conductor::new(
        transport,
        TeamRegistry::direct(),
        Router::keyword_only(),
        Gateway::new(vec!["bash:rm -rf".to_owned()], Vec::new()),
        limits(),
    );
    let verdict = conductor.judge_tool_call(&ToolCall {
        tool: "bash".to_owned(),
        args: "rm -rf /".to_owned(),
    });
    assert!(matches!(verdict, mas_core::gateway::ToolVerdict::Deny { .. }));
}

#[tokio::test]
async fn completed_result_finishes_the_item_without_repair_burn() {
    let transport = StreamTransport::replaying(vec![Ok(completed_frame("req-1"))]);
    let mut conductor =
        Conductor::new(transport, TeamRegistry::direct(), Router::keyword_only(), Gateway::permissive(), limits());
    conductor.add_work(analyst_item("map the source"), Vec::new());
    let summary = conductor.run().await.unwrap();
    assert_eq!(summary.verdict, Verdict::Success);
    assert_eq!(summary.final_phase, Phase::Done);
    // The wire-vocabulary result completes the item: one dispatch, no
    // re-dispatch, no repair burn.
    assert_eq!(summary.dispatches, 1);
}

#[tokio::test]
async fn error_result_burns_the_repair_budget() {
    let transport = StreamTransport::replaying(vec![Ok(error_frame("req-1"))]);
    let mut limits = limits();
    limits.graph.max_repairs = 0;
    let mut conductor =
        Conductor::new(transport, TeamRegistry::direct(), Router::keyword_only(), Gateway::permissive(), limits);
    conductor.add_work(analyst_item("map the source"), Vec::new());
    let summary = conductor.run().await.unwrap();
    assert_eq!(
        summary.verdict,
        Verdict::Failed {
            reason: "a work item exhausted its repair budget".to_owned()
        }
    );
}
