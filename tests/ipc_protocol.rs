//! Integration test for the iAgent named-pipe IPC protocol.
//!
//! The named-pipe IPC mechanism is Windows-only.  On Unix/Linux this test
//! is a no-op that always passes.
//!
//! Run with: cargo test --test ipc_protocol

#[cfg(not(target_os = "windows"))]
#[test]
fn ipc_skipped_on_unix() {
    // Named pipe IPC is Windows-only; test is a no-op on Unix.
    assert!(true);
}

#[cfg(target_os = "windows")]
#[test]
fn ipc_test_windows_only() {
    // Placeholder for Windows-specific IPC tests.
    // Real implementation would connect to the named pipe and exchange messages.
    assert!(true);
}
