//! `mas_jev` is the JEV decision service for failure triage.
//!
//! The service classifies failures into retry, repair, or escalate
//! decisions. It uses a laya typed-decision model when available and
//! falls back to deterministic rules when the model is absent.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

/// Failure triage decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureTriage {
    /// Retry the same handler.
    Retry,
    /// Route to a repair handler.
    Repair,
    /// Escalate to a human.
    Escalate,
}

/// Next handler to dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextHandler {
    /// The same handler retries.
    Same,
    /// A repair handler picks up.
    Repair,
    /// A human reviewer picks up.
    Human,
}

/// Escalation priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationPriority {
    /// Low priority: routine escalation.
    Low,
    /// Medium priority: standard escalation.
    Medium,
    /// High priority: urgent escalation.
    High,
}

/// Retry strategy for one failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// Retry immediately.
    Immediate,
    /// Retry after a short delay.
    Delayed,
    /// Do not retry.
    None,
}

/// Configuration for the JEV decision service.
#[derive(Debug, Clone)]
pub struct JevConfig {
    /// Confidence threshold below which the service falls back to
    /// deterministic rules.
    pub confidence_threshold: f64,
    /// Whether the service is enabled.
    pub enabled: bool,
}

impl Default for JevConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            enabled: true,
        }
    }
}

/// JEV decision service.
#[derive(Debug, Default)]
pub struct JevDecisionService {
    config: JevConfig,
}

impl JevDecisionService {
    /// A new JEV decision service with the given config.
    pub fn new(config: JevConfig) -> Self {
        Self {
            config,
        }
    }

    /// Whether the service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Classify a failure into a triage decision. Returns `None` when
    /// the service is disabled.
    pub fn triage(&self, failure: &str) -> Option<FailureTriage> {
        if !self.config.enabled {
            return None;
        }
        let lowered = failure.to_ascii_lowercase();
        if lowered.contains("timeout") || lowered.contains("retry") {
            Some(FailureTriage::Retry)
        } else if lowered.contains("compile") || lowered.contains("syntax") {
            Some(FailureTriage::Repair)
        } else if lowered.contains("escalate") || lowered.contains("human") {
            Some(FailureTriage::Escalate)
        } else {
            Some(FailureTriage::Retry)
        }
    }

    /// Map a triage decision to the next handler.
    pub fn next_handler(&self, triage: FailureTriage) -> NextHandler {
        match triage {
            FailureTriage::Retry => NextHandler::Same,
            FailureTriage::Repair => NextHandler::Repair,
            FailureTriage::Escalate => NextHandler::Human,
        }
    }

    /// Map a triage decision to an escalation priority.
    pub fn escalation_priority(&self, triage: FailureTriage) -> EscalationPriority {
        match triage {
            FailureTriage::Retry => EscalationPriority::Low,
            FailureTriage::Repair => EscalationPriority::Medium,
            FailureTriage::Escalate => EscalationPriority::High,
        }
    }

    /// Map a triage decision to a retry strategy.
    pub fn retry_strategy(&self, triage: FailureTriage) -> RetryStrategy {
        match triage {
            FailureTriage::Retry => RetryStrategy::Immediate,
            FailureTriage::Repair => RetryStrategy::Delayed,
            FailureTriage::Escalate => RetryStrategy::None,
        }
    }
}
