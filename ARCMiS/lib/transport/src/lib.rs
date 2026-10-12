//! `mas_transport` is the seam between the conductor and the omp child
//! (ADR 0034, ADR 0035).
//!
//! The conductor drives one run through `send` and `recv` only. The
//! default implementation wraps one `omp_rpc::Client` session from the
//! xoannova oh-my-pi-omp SDK (ADR 0035). Tests swap in a scripted
//! transport. The trait stays object-safe: one run holds one boxed
//! transport.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod forward_frames;
pub mod map_sdk_error;
pub mod omp_launch_config;
pub mod omp_transport;
pub mod transport_trait;

pub use forward_frames::forward_frames;
pub use map_sdk_error::map_sdk_error;
pub use omp_launch_config::OmpLaunchConfig;
pub use omp_transport::OmpTransport;
pub use transport_trait::Transport;

pub use error::TransportError;
