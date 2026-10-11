//! `error` names the run failure classes callers must distinguish.

/// One conductor run failure. Every variant names a distinct recovery
/// posture: restart the transport, correct the configuration, tighten
/// the gateway policy, or raise the budgets.
#[derive(Debug, thiserror::Error)]
pub enum ConductorError {
    /// The omp transport closed or failed its handshake. Restart the
    /// session and re-enter the run loop.
    #[error("transport failed: {0}")]
    Transport(String),
    /// The run configuration was invalid. Correct the file and start a new
    /// run: no retry corrects a bad configuration.
    #[error("configuration invalid: {0}")]
    Config(String),
    /// The tool gateway denied a call pattern that the run marked
    /// mandatory. Tighten the call pattern or loosen the policy.
    #[error("gateway rejected a mandatory call: {0}")]
    Gateway(String),
    /// The transport closed while work remained outstanding.
    #[error("budget exhausted: {0}")]
    BudgetExhausted(String),
    /// The run regressed until a regression edge held no rework path.
    #[error("regression exhausted: {0}")]
    RegressionExhausted(String),
}
