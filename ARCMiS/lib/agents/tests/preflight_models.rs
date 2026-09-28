//! Preflight test: the model-registry check must fail with a fix-naming
//! error when a configured model is absent from the daemon's tag list.

use std::io::Read as _;
use std::io::Write as _;
use std::net::TcpListener;

use agents::util::provider::Provider;

/// Serve one JSON body from a throwaway TCP listener as `/api/tags`.
/// Accepts up to four connections (one test request plus slack); the
/// listener closes when the handle drops at test end.
fn mock_tags_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let mut buf = [0u8; 2048];
            if std::io::Read::read(&mut stream, &mut buf).is_err() {
                break;
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: \
                 close\r\n\r\n{}",
                body.len(),
                body
            );
            if stream.write_all(response.as_bytes()).is_err() {
                break;
            }
        }
    });
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn check_models_accepts_served_and_untagged_latest() {
    let base_url = mock_tags_server(
        r#"{"models":[{"name":"recodeagent-sft:latest"},{"name":"smtek/Swift-Qwen3.8-27B:map-k4v"}]}"#,
    );
    let provider = Provider::Ollama {
        base_url: Some(base_url),
    };
    // Both a tagged and an untagged (implicit :latest) name must resolve.
    provider
        .check_models(&["recodeagent-sft".to_owned(), "smtek/Swift-Qwen3.8-27B:map-k4v".to_owned()])
        .await
        .expect("served models pass");
}

#[tokio::test]
async fn check_models_names_missing_model_and_fix() {
    let base_url = mock_tags_server(r#"{"models":[{"name":"qwen3.8:27b-mtp-q4_K_M"}]}"#);
    let provider = Provider::Ollama {
        base_url: Some(base_url),
    };
    let error = provider.check_models(&["recodeagent-sft".to_owned()]).await.expect_err("missing model must fail");
    let message = format!("{error:#}");
    assert!(message.contains("recodeagent-sft"), "error names the model: {message}");
    assert!(message.contains("ollama create"), "error names the fix: {message}");
}

#[tokio::test]
async fn check_models_fails_clean_when_daemon_down() {
    // Nothing listens on the port: the error must name 'ollama serve',
    // not surface a raw connection-refused chain.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let provider = Provider::Ollama {
        base_url: Some(format!("http://127.0.0.1:{port}")),
    };
    let error = provider.check_models(&["recodeagent-sft".to_owned()]).await.expect_err("down daemon must fail");
    let message = format!("{error:#}");
    assert!(message.contains("ollama serve"), "error names the start command: {message}");
}

#[tokio::test]
async fn check_models_skips_non_ollama_providers() {
    // Netmind has no local registry; the check is a no-op there.
    let provider = Provider::Netmind {
        api_key: "unused".to_owned(),
        base_url: "https://example.invalid/v1".to_owned(),
    };
    provider.check_models(&["recodeagent-sft".to_owned()]).await.expect("netmind skips the registry check");
}
