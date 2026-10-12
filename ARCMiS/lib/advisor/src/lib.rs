//! `mas_advisor` is the read-only OMP consultation handle (ADR 0036 section 6).
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

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

use std::collections::VecDeque;

use transport::OmpLaunchConfig;
use transport::OmpTransport;
use transport::TransportError;

pub use error::AdvisorError;

/// Knobs for one advisor. Every value bubbles from the call site
/// (config rule): the advisor never reads the environment.
#[derive(Debug, Clone)]
pub struct AdvisorConfig {
    /// Master switch. A disabled advisor returns `Defer` for every
    /// consultation and never spawns a transport.
    pub enabled: bool,
    /// Maximum number of pending consultations. The advisor returns
    /// `BacklogFull` once the backlog reaches this size.
    pub sync_backlog: usize,
    /// Launch config for the advisor's transport. The caller sets
    /// `read_only: true` so the advisor cannot write files or run
    /// commands.
    pub launch: OmpLaunchConfig,
}

impl Default for AdvisorConfig {
    fn default() -> Self {
        // WHY read_only: the advisor answers questions, never edits
        // files. Disabling every built-in tool at the CLI level keeps
        // the session read-only even if a prompt asks for a write.
        let launch = OmpLaunchConfig {
            read_only: true,
            ..OmpLaunchConfig::default()
        };
        Self {
            enabled: false,
            sync_backlog: 16,
            launch,
        }
    }
}

/// One consultation the advisor received. The response is `None`
/// while the consultation is pending and `Some` once the advisor
/// answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisorConsultation {
    /// The question the conductor asked.
    pub question: String,
    /// The context the conductor attached.
    pub context: String,
    /// The advisor's answer, when one arrives.
    pub response: Option<String>,
}

/// The advisor's answer to one consultation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisorVerdict {
    /// The advisor answered the question.
    Answer(String),
    /// The advisor accepted the consultation but has no answer yet,
    /// or the advisor is disabled. The caller waits for the next
    /// drain or falls back to the next routing layer.
    Defer,
    /// The advisor returned an error string. The caller logs and
    /// falls back.
    Error(String),
}

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
