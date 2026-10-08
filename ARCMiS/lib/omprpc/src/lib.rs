//! omp coding-agent RPC wire types and stdio transport.
#![allow(non_snake_case)]

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
pub mod client;
pub mod frame;
pub mod frame_payloads;
pub mod transport;
