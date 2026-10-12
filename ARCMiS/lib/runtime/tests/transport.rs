//! Integration tests for the transport module.

use std::io;
use std::path::PathBuf;

use mas_runtime::map_sdk_error;
use mas_runtime::OmpLaunchConfig;
use mas_runtime::TransportError;
use omp_rpc::Error as SdkError;

#[test]
fn launch_argv_keeps_the_binary_name_a_parameter() {
    let config = OmpLaunchConfig {
        program: "ompl".to_string(),
        model: Some("provider/model-x".to_string()),
        approval_mode: "yolo".to_string(),
        extra_args: vec!["--flag".to_string()],
        ..OmpLaunchConfig::default()
    };
    assert_eq!(config.argv(), vec!["--mode", "rpc", "--model=provider/model-x", "--approval-mode=yolo", "--flag"]);
}

#[test]
fn launch_argv_omits_the_model_flag_when_unset() {
    let config = OmpLaunchConfig::default();
    assert_eq!(config.argv(), vec!["--mode", "rpc", "--approval-mode=yolo"]);
}

#[test]
fn launch_default_matches_a_plain_shell_launch() {
    let config = OmpLaunchConfig::default();
    assert_eq!(config.program, "omp");
    assert_eq!(config.working_dir, PathBuf::from("."));
    assert_eq!(config.approval_mode, "yolo");
}

#[test]
fn sdk_errors_map_onto_the_transport_variants() {
    assert!(matches!(map_sdk_error(SdkError::Closed), TransportError::Closed));
    assert!(matches!(
        map_sdk_error(SdkError::Timeout {
            command: "prompt".to_string()
        }),
        TransportError::Timeout
    ));
    let rejected = SdkError::Command {
        command: "prompt".to_string(),
        error: "model exploded".to_string(),
        code: Some("prompt_failed".to_string()),
    };
    assert!(matches!(map_sdk_error(rejected), TransportError::BadRequest(_)));
    let broken = SdkError::Protocol("bad chunk".to_string());
    assert!(matches!(map_sdk_error(broken), TransportError::BadRequest(_)));
    let missing = SdkError::Io(io::Error::new(io::ErrorKind::NotFound, "no such file"));
    assert!(matches!(map_sdk_error(missing), TransportError::Send(_)));
}
