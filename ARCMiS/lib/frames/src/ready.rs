//! First frame on stdout.

use serde::Deserialize;

/// First frame on stdout. Carries the protocol version and frame limits.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyFrame {
    /// Protocol version the server speaks.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    /// Protocol versions the server accepts.
    #[serde(rename = "supportedProtocolVersions")]
    pub supported_protocol_versions: Vec<u32>,
    /// Maximum single-frame byte size.
    #[serde(rename = "maxFrameBytes")]
    pub max_frame_bytes: u64,
    /// Maximum reassembled-frame byte size.
    #[serde(rename = "maxReassembledFrameBytes")]
    pub max_reassembled_frame_bytes: u64,
}
