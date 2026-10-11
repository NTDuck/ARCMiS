//! `conductor` runs one migration end to end: phases, dispatches, budgets,
//! and the final verdict (ADR 0031).
//!
//! The conductor owns the phase machine, the task graph, and the
//! transport. It asks the gateway before any tool call leaves the run.
//! Every phase advance is evidence-gated. Every failure burns repair
//! budget before it escalates.

use std::time::Duration;

use crate::context::ContextStore;
use crate::error::ConductorError;
use crate::frame_sink;
use crate::gateway::Gateway;
use crate::gateway::ToolCall;
use crate::gateway::ToolVerdict;
use crate::hierarchy::TeamRegistry;
use crate::phase::Evidence;
use crate::phase::Phase;
use crate::phase::PhaseMachine;
use crate::router::Router;
use crate::taskgraph::GraphBudgets;
use crate::taskgraph::TaskGraph;
use crate::taskgraph::WorkItem;
use crate::transport::Transport;
use crate::transport::TransportError;

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every phase gate held through FinalValidation.
    Success,
    /// The run ended early. Carries the failure class.
    Failed {
        reason: String,
    },
}

/// End-of-run summary.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    /// Final phase the machine reached.
    pub final_phase: Phase,
    /// How the run ended.
    pub verdict: Verdict,
    /// Completed dispatch count.
    pub dispatches: usize,
    /// Compaction passes over the run.
    pub compactions: usize,
}

/// One round's fresh activity, for stagnation accounting. Counts hold
/// round deltas, never cumulative totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundOutcome {
    /// Work items dispatched this round.
    pub dispatched: usize,
    /// Work items completed this round.
    pub completed: usize,
}

/// Budgets and policy knobs for one run. Every value bubbles from the
/// call site (config rule): the conductor never reads the environment.
#[derive(Debug, Clone)]
pub struct RunLimits {
    /// Graph budgets: fanout, repairs, stagnation.
    pub graph: GraphBudgets,
    /// Projected-token ceiling that triggers compaction.
    pub context_budget: usize,
    /// Per-event wait on the transport.
    pub event_timeout: Duration,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            graph: GraphBudgets::default(),
            context_budget: 100_000,
            event_timeout: Duration::from_secs(600),
        }
    }
}

/// One conductor run.
pub struct Conductor<T: Transport> {
    transport: T,
    registry: TeamRegistry,
    router: Router,
    gateway: Gateway,
    phases: PhaseMachine,
    graph: TaskGraph,
    limits: RunLimits,
    context: ContextStore,
    dispatches: usize,
}

impl<T: Transport> Conductor<T> {
    /// Assemble one run over `transport`.
    pub fn new(transport: T, registry: TeamRegistry, router: Router, gateway: Gateway, limits: RunLimits) -> Self {
        let graph = TaskGraph::new(limits.graph);
        let context = ContextStore::new(limits.context_budget);
        Self {
            transport,
            registry,
            router,
            gateway,
            phases: PhaseMachine::new(),
            graph,
            limits,
            context,
            dispatches: 0,
        }
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phases.phase()
    }

    /// The routing layer for failure triage during the run.
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Judge one tool call through the gateway before it leaves the run.
    pub fn judge_tool_call(&self, call: &ToolCall) -> ToolVerdict {
        self.gateway.judge(call)
    }

    /// Add one work item to the graph.
    pub fn add_work(&mut self, item: WorkItem, dependencies: Vec<usize>) -> usize {
        self.graph.add(item, dependencies)
    }

    /// Publish one context frame.
    pub fn publish_context(&mut self, source: &str, text: &str) {
        self.context.publish(source, text);
    }

    /// Run one dispatch round: admit ready work, drive the transport, and
    /// drain frames. Returns what the round accomplished.
    pub async fn round(&mut self) -> Result<RoundOutcome, ConductorError> {
        let done_before = self.graph.status_counts().done;
        let ready = self.graph.ready();
        let dispatched = ready.len();
        for id in ready {
            self.graph.admit(id);
            self.graph.start(id);
            let instruction = self.instruction_for(id);
            let request_id = self.transport.send_prompt(&instruction).await.map_err(Self::transport_error)?;
            tracing::debug!(id, request_id, "dispatched work item");
            self.dispatches += 1;
        }
        self.drain_events().await?;
        self.context.settle();
        Ok(RoundOutcome {
            dispatched,
            completed: self.graph.status_counts().done - done_before,
        })
    }

    /// Run the whole phase walk until `Done`, budgets exhaust, or the
    /// transport closes.
    pub async fn run(&mut self) -> Result<RunSummary, ConductorError> {
        loop {
            if self.phases.phase() == Phase::Done {
                return Ok(self.summarize(Verdict::Success));
            }
            let outcome = self.round().await?;
            if self.graph.status_counts().failed > 0 {
                return Ok(self.summarize(Verdict::Failed {
                    reason: "a work item exhausted its repair budget".to_owned(),
                }));
            }
            let settled = self.graph.is_settled();
            if settled {
                self.advance_phase()?;
            }
            // WHY round outcomes: a settled graph reports no fresh
            // completions while the phase walk advances, so stagnation
            // counts only rounds with no dispatch, no completion, and no
            // phase advance.
            if outcome.dispatched > 0 || outcome.completed > 0 || settled {
                self.graph.record_progress();
            } else if self.graph.record_stagnation() {
                return Ok(self.summarize(Verdict::Failed {
                    reason: "the graph stalled past the stagnation ceiling".to_owned(),
                }));
            }
        }
    }

    /// Try one forward phase advance under the collected evidence.
    fn advance_phase(&mut self) -> Result<(), ConductorError> {
        let next = self.phases.phase().forward_next();
        let evidence = self.evidence_for(next);
        self.phases
            .try_advance(next, "gate met", Some(evidence))
            .map_err(ConductorError::RegressionExhausted)
            .map(|_| ())
    }

    /// The evidence one phase exit measured.
    fn evidence_for(&self, phase: Phase) -> Evidence {
        match phase {
            Phase::FinalValidation => Evidence::TestCommandGreen,
            Phase::Pilot | Phase::Migration | Phase::Integration => {
                let counts = self.graph.status_counts();
                Evidence::Deliverables {
                    published: counts.done,
                    minimum: 1,
                }
            },
            _ => Evidence::CompletedDispatch,
        }
    }

    /// The instruction text for one node, addressed to its role.
    fn instruction_for(&mut self, id: usize) -> String {
        let item = self.graph.item(id);
        let lead = self.registry.lead_of(item.role);
        lead.map_or_else(
            || format!("{}: {}", item.role.name(), item.instruction),
            |lead| format!("{} (via {}): {}", item.role.name(), lead, item.instruction),
        )
    }

    /// Drain transport events until the wait times out. Frames append to
    /// the context.
    async fn drain_events(&mut self) -> Result<(), ConductorError> {
        loop {
            match self.transport.recv_within(self.limits.event_timeout).await {
                Ok(event) => frame_sink::absorb(&mut self.graph, &mut self.context, event),
                Err(TransportError::Timeout) => return Ok(()),
                Err(TransportError::Closed) => return Err(ConductorError::Transport("transport closed".to_owned())),
                Err(other) => return Err(ConductorError::Transport(other.to_string())),
            }
        }
    }

    /// Snapshot the run.
    fn summarize(&self, verdict: Verdict) -> RunSummary {
        RunSummary {
            final_phase: self.phases.phase(),
            verdict,
            dispatches: self.dispatches,
            compactions: self.phases.history.len(),
        }
    }

    /// Map transport failures onto run failure classes.
    fn transport_error(error: TransportError) -> ConductorError {
        match error {
            TransportError::Closed => ConductorError::Transport("closed".to_owned()),
            other => ConductorError::Transport(other.to_string()),
        }
    }
}
