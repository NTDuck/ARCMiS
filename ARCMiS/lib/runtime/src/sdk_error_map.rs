//! Map SDK errors onto the local transport error.

use omp_rpc::Error as SdkError;

use crate::transport_error::TransportError;

/// Map one SDK error onto the local transport error.
pub fn map_sdk_error(error: SdkError) -> TransportError {
    match error {
        SdkError::Closed => TransportError::Closed,
        SdkError::Timeout {
            ..
        } => TransportError::Timeout,
        SdkError::Command {
            command,
            error: detail,
            code: Some(code),
        } => TransportError::BadRequest(format!("{command} failed ({code}): {detail}")),
        SdkError::Command {
            command,
            error: detail,
            code: None,
        } => TransportError::BadRequest(format!("{command} failed: {detail}")),
        SdkError::Protocol(detail) => TransportError::BadRequest(detail),
        SdkError::InvalidArgument(detail) => TransportError::BadRequest(detail),
        SdkError::Io(error) => TransportError::Send(error.to_string()),
        SdkError::Json(error) => TransportError::Send(error.to_string()),
    }
}
