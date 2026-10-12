//! Read-only OMP consultation handle for the MAS conductor (ADR 0036 section 6).
//!
//! The advisor wraps a second `OmpTransport` configured with
//! `read_only: true`. It answers classification questions the
//! deterministic tier marks `Ask` and the laya tier marks `Fallback`.
//! The advisor shares the same model as the primary transport: no
//! second model slot, no second provider.
//!
//! The advisor's `syncBacklog` is bounded by the configured
//! `advisor_backlog` limit (default 16). When the backlog is full the
//! advisor returns `BacklogFull` so the primary transport never blocks
//! on the advisor.

use std::collections::VecDeque;

use crate::advisor_config::AdvisorConfig;
use crate::advisor_consultation::AdvisorConsultation;
use crate::advisor_error::AdvisorError;
use crate::advisor_verdict::AdvisorVerdict;
use crate::transport::OmpTransport;
use crate::transport_error::TransportError;

/// The OMP advisor handle. One per run. The advisor owns its own
/// transport so the primary transport never blocks on a slow advisor.
pub struct OmpAdvisor {
    config: AdvisorConfig,
    transport: Option<OmpTransport>,
    backlog: VecDeque<AdvisorConsultation>,
}

impl OmpAdvisor {
    /// Assemble one advisor. The transport spawns lazily on the first
    /// flush so a disabled advisor never opens a child.
    pub fn new(config: AdvisorConfig) -> Self {
        Self {
            config,
            transport: None,
            backlog: VecDeque::new(),
        }
    }

    /// Whether the advisor is enabled in the run config.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Ask the advisor one question. Returns `Defer` when the advisor
    /// is disabled. Returns `BacklogFull` when the queue is at
    /// capacity. Returns `Defer` when the consultation is queued and
    /// waiting for an answer.
    ///
    /// The transport spawns lazily on the first flush, not on the
    /// first consult. A disabled advisor never spawns.
    pub fn consult(&mut self, question: &str, context: &str) -> Result<AdvisorVerdict, AdvisorError> {
        if !self.config.enabled {
            tracing::debug!("advisor consult skipped: disabled");
            return Ok(AdvisorVerdict::Defer);
        }
        if self.backlog.len() >= self.config.sync_backlog {
            tracing::warn!(backlog = self.backlog.len(), limit = self.config.sync_backlog, "advisor backlog full");
            return Err(AdvisorError::BacklogFull);
        }
        let consultation = AdvisorConsultation {
            question: question.to_owned(),
            context: context.to_owned(),
            response: None,
        };
        self.backlog.push_back(consultation);
        tracing::debug!(question, "advisor consult queued");
        Ok(AdvisorVerdict::Defer)
    }

    /// Drain every pending consultation. The caller hands the list to
    /// the next routing layer for triage.
    pub fn drain_backlog(&mut self) -> Vec<AdvisorConsultation> {
        let pending = std::mem::take(&mut self.backlog);
        pending.into()
    }

    /// Spawn the advisor transport on demand. Returns the existing
    /// transport if one is already open. Returns `TransportError` if
    /// the child refuses to launch.
    pub fn ensure_transport(&mut self) -> Result<&mut OmpTransport, TransportError> {
        if self.transport.is_none() {
            let transport = OmpTransport::spawn(&self.config.launch)?;
            self.transport = Some(transport);
        }
        Ok(self.transport.as_mut().expect("transport just spawned"))
    }
}
