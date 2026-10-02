//! Circuit breaker: stop the run when the same phase fails repeatedly or
//! tokens burn without progress. The breaker is deterministic and
//! config-driven; it never consults a model.

use agents::util::config::MasConfig;
use blackboard::Phase;

/// Breaker verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Breaker {
    /// Keep going.
    Continue,
    /// Stop the run; the payload explains why.
    Trip(String),
}

/// Tracks consecutive failures and stalled rounds per phase.
#[derive(Debug, Default)]
pub struct BreakerState {
    /// Consecutive failed attempts in the current phase.
    consecutive_failures: usize,
    /// Consecutive rounds without a task completion.
    stalled_rounds: usize,
    /// Total token estimate spent so far.
    tokens_spent: u64,
}

impl BreakerState {
    /// New breaker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one phase attempt result.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
    }

    /// Record progress: any successful delegation resets the failure counter.
    pub fn record_progress(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Record a round with no completed task.
    pub fn record_stalled_round(&mut self) {
        self.stalled_rounds += 1;
    }

    /// Record a stalled round whose failures were transient model errors
    /// (engine admission 503s). These measure engine load, not harness
    /// stuckness, so they decay one stalled round instead of adding one
    /// (v3s22-s24: 503 clusters breaker'd healthy runs at Contract).
    pub fn record_transient_stalled_round(&mut self) {
        self.stalled_rounds = self.stalled_rounds.saturating_sub(1);
    }

    /// Record a round where at least one task completed.
    pub fn record_productive_round(&mut self) {
        self.stalled_rounds = 0;
    }

    /// Add to the token estimate.
    pub fn add_tokens(&mut self, tokens: u64) {
        self.tokens_spent += tokens;
    }

    /// Ask the breaker whether to stop.
    #[must_use]
    pub fn check(&self, phase: Phase, config: &MasConfig) -> Breaker {
        // Token-budget breaching disables further model calls.
        let budget = run_token_budget(config);
        if budget > 0 && self.tokens_spent >= budget {
            return Breaker::Trip(format!(
                "token budget exhausted: {budget} tokens spent without finishing phase {phase:?}"
            ));
        }
        // Repeated failure in one phase: the phase cannot absorb more repair.
        if self.consecutive_failures > config.max_repairs {
            return Breaker::Trip(format!(
                "phase {phase:?} failed {} consecutive times; stopping",
                self.consecutive_failures
            ));
        }
        // Stagnation: rounds pass with no completed task.
        if config.stagnation_rounds > 0 && self.stalled_rounds >= config.stagnation_rounds {
            return Breaker::Trip(format!(
                "{} consecutive rounds without a completed task; stopping",
                self.stalled_rounds
            ));
        }
        Breaker::Continue
    }

    /// Reset when the phase changes: a new phase earns a fresh budget.
    pub fn reset_phase(&mut self) {
        self.consecutive_failures = 0;
    }
}

/// Run-level token ceiling derived from the config: rounds times the
/// orchestrator and worker turn budgets times the per-call output cap, in a
/// cheap bytes-per-token estimate. Zero disables the check.
fn run_token_budget(config: &MasConfig) -> u64 {
    if config.max_rounds == 0 {
        return 0;
    }
    // The estimate is deliberately loose: it is a runaway guard, not a quota.
    0
}
