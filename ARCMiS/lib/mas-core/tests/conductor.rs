//! Integration tests for the conductor over a scripted transport.

use std::time::Duration;

use mas_core::conductor::Conductor;
use mas_core::conductor::RunLimits;
use mas_core::conductor::Verdict;
use mas_core::error::ConductorError;
use mas_core::gateway::Gateway;
use mas_core::gateway::ToolCall;
use mas_core::hierarchy::TeamRegistry;
use mas_core::phase::Phase;
use mas_core::router::Router;
use mas_core::taskgraph::GraphBudgets;
use mas_core::taskgraph::WorkItem;
use mas_core::transport::Transport;
use mas_core::transport::TransportError;
use omprpc::frame::Event;

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
        Ok(Event::Response(omprpc::frame_payloads::ResponseFrame {
            id: Some("req-1".to_owned()),
            command: None,
            success: true,
            data: Some(serde_json::json!("done")),
            error: None,
        }))
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
