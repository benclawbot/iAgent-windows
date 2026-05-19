//! End-to-end smoke tests for the iagent binary
//!
//! These tests verify the critical happy path without requiring real API credentials.
//! They are designed to be fast and run in CI as a pre-build sanity check.

use crate::test_support::*;
use std::process::Command;

// ============================================================================
// Smoke Tests: Binary Starts and Responds
// These tests verify the binary starts without crashing and responds to basic commands.
// ============================================================================

/// Test that the iagent binary responds to --version without crashing
#[tokio::test]
async fn smoke_version_command() -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_iagent"))
        .arg("--version")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Version command should succeed. Exit code: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        stdout.contains("iagent") || stdout.contains("0.12"),
        "Version output should contain 'jcode' or version number. Got: {}",
        stdout
    );

    Ok(())
}

/// Test that the iagent binary responds to --help without crashing
#[tokio::test]
async fn smoke_help_command() -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_iagent"))
        .arg("--help")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Help command should succeed. Exit code: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        !stdout.is_empty(),
        "Help output should not be empty. stderr: {}",
        stderr
    );

    // Should contain common CLI help content
    let help_lower = stdout.to_lowercase();
    assert!(
        help_lower.contains("usage")
            || help_lower.contains("options")
            || help_lower.contains("commands"),
        "Help output should mention usage, options, or commands. Got: {}",
        stdout
    );

    Ok(())
}

/// Test that the iagent binary can start a server and produce a text response
/// This uses the mock provider to avoid needing real API credentials.
#[tokio::test]
async fn smoke_basic_round_trip() -> Result<()> {
    let _env = setup_test_env()?;
    let runtime_dir = short_runtime_dir(format!(
        "iagent-smoke-roundtrip-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&runtime_dir)?;
    let socket_path = runtime_dir.join("iagent.sock");
    let debug_socket_path = runtime_dir.join("iagent-debug.sock");

    // Create a mock provider with a simple response
    let provider = MockProvider::new();
    provider.queue_response(vec![
        StreamEvent::ConnectionType {
            connection: "mock-smoke".to_string(),
        },
        StreamEvent::TextDelta("smoke-test-response".to_string()),
        StreamEvent::MessageEnd {
            stop_reason: Some("end_turn".to_string()),
        },
        StreamEvent::SessionId("smoke-session-1".to_string()),
    ]);

    let provider: Arc<dyn iagent::provider::Provider> = Arc::new(provider);
    let server_instance =
        server::Server::new_with_paths(provider, socket_path.clone(), debug_socket_path.clone());
    let server_handle = tokio::spawn(async move { server_instance.run().await });

    let result = async {
        wait_for_socket(&socket_path).await?;
        let mut client = server::Client::connect_with_path(socket_path.clone()).await?;

        // Subscribe and wait for initial connection
        let subscribe_id = client.subscribe().await?;
        let events = collect_until_done_unix(&mut client, subscribe_id).await?;

        // Verify we got the mock response
        let has_connection_event = events
            .iter()
            .any(|e| matches!(e, ServerEvent::ConnectionType { .. }));
        assert!(
            has_connection_event,
            "Should have received connection type event. Events: {:?}",
            events
        );

        // Send a message and verify we get a response
        let message_id = client.send_message("smoke test").await?;
        let message_events = collect_until_done_unix(&mut client, message_id).await?;

        // Verify we got the smoke test response
        let response_text = message_events
            .iter()
            .filter_map(|e| match e {
                ServerEvent::TextDelta { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        assert!(
            response_text.contains("smoke-test-response"),
            "Should receive smoke-test-response. Got: {}",
            response_text
        );

        Ok::<_, anyhow::Error>(())
    }
    .await;

    abort_server_and_cleanup(&server_handle, &socket_path, &debug_socket_path);
    result
}

/// Test that the iagent binary responds to version with short timeout
/// This test is useful for CI to quickly verify the binary is not completely broken
#[tokio::test]
async fn smoke_version_quick() -> Result<()> {
    use std::time::Duration;

    // Use tokio's process::Command for async timeout support
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_iagent"))
        .arg("--version")
        .spawn()?;

    let result = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output()).await;

    assert!(
        result.is_ok(),
        "Version command should complete within 5 seconds"
    );

    let output = result??;
    assert!(
        output.status.success(),
        "Version command should succeed. Exit code: {:?}",
        output.status.code()
    );

    Ok(())
}
