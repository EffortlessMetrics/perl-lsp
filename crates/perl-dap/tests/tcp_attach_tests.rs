//! TCP Attach Tests
//!
//! Comprehensive tests for TCP attach functionality in the DAP adapter.
//!
//! These tests validate:
//! - TCP connection establishment
//! - Message proxying between client and debugger
//! - Event handling and propagation
//! - Error recovery and timeout handling
//! - Cross-platform compatibility

use perl_dap::tcp_attach::{DapEvent, TcpAttachConfig, TcpAttachSession};
use perl_lsp_rs_core::transport::framing::frame;
use perl_tdd_support::must;
use std::io::Write;
use std::net::TcpListener;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

/// Test helper to create a valid TCP attach configuration
fn create_valid_config() -> TcpAttachConfig {
    TcpAttachConfig::new("127.0.0.1".to_string(), 13603)
}

#[test]
fn test_tcp_attach_config_validation() {
    // Test valid configuration
    let config = create_valid_config();
    assert!(config.validate().is_ok());

    // Test with timeout
    let config = create_valid_config().with_timeout(5000);
    assert!(config.validate().is_ok());

    // Test empty host
    let config = TcpAttachConfig::new("".to_string(), 13603);
    assert!(config.validate().is_err());

    // Test invalid port
    let config = TcpAttachConfig::new("localhost".to_string(), 0);
    assert!(config.validate().is_err());

    // Test zero timeout
    let config = create_valid_config().with_timeout(0);
    assert!(config.validate().is_err());

    // Test timeout too large
    let config = create_valid_config().with_timeout(300_001);
    assert!(config.validate().is_err());
}

#[test]
fn test_tcp_attach_timeout_duration() {
    // Test default timeout
    let config = create_valid_config();
    assert_eq!(config.timeout_duration(), Duration::from_millis(5000));

    // Test custom timeout
    let config = create_valid_config().with_timeout(10000);
    assert_eq!(config.timeout_duration(), Duration::from_millis(10000));
}

#[test]
fn test_tcp_attach_session_creation() {
    let session = TcpAttachSession::new();
    assert!(!session.is_connected());
}

#[test]
fn test_tcp_attach_session_event_sender() {
    let mut session = TcpAttachSession::new();
    let (tx, rx) = channel::<DapEvent>();
    session.set_event_sender(tx.clone());

    // Send an event and verify it's received
    let event =
        DapEvent::Output { category: "stdout".to_string(), output: "test output".to_string() };
    must(tx.send(event));

    let received = must(rx.recv_timeout(Duration::from_millis(100)));
    match received {
        DapEvent::Output { category, output } => {
            assert_eq!(category, "stdout");
            assert_eq!(output, "test output");
        }
        _ => must(Err::<(), _>("Received unexpected event type")),
    }
}

#[test]
fn test_tcp_attach_event_variants() {
    // Test all event variants
    let (tx, rx) = channel::<DapEvent>();

    // Test Output event
    must(tx.send(DapEvent::Output { category: "stdout".to_string(), output: "test".to_string() }));
    if let DapEvent::Output { .. } = must(rx.recv_timeout(Duration::from_millis(100))) {
        // Success
    } else {
        must(Err::<(), _>("Expected Output event"));
    }

    // Test Stopped event
    must(tx.send(DapEvent::Stopped { reason: "breakpoint".to_string(), thread_id: 1 }));
    if let DapEvent::Stopped { .. } = must(rx.recv_timeout(Duration::from_millis(100))) {
        // Success
    } else {
        must(Err::<(), _>("Expected Stopped event"));
    }

    // Test Continued event
    must(tx.send(DapEvent::Continued { thread_id: 1 }));
    if let DapEvent::Continued { .. } = must(rx.recv_timeout(Duration::from_millis(100))) {
        // Success
    } else {
        must(Err::<(), _>("Expected Continued event"));
    }

    // Test Terminated event
    must(tx.send(DapEvent::Terminated { reason: "normal".to_string() }));
    if let DapEvent::Terminated { .. } = must(rx.recv_timeout(Duration::from_millis(100))) {
        // Success
    } else {
        must(Err::<(), _>("Expected Terminated event"));
    }

    // Test Error event
    must(tx.send(DapEvent::Error { message: "test error".to_string() }));
    if let DapEvent::Error { .. } = must(rx.recv_timeout(Duration::from_millis(100))) {
        // Success
    } else {
        must(Err::<(), _>("Expected Error event"));
    }
}

#[test]
fn test_tcp_attach_session_disconnect() {
    let mut session = TcpAttachSession::new();
    assert!(!session.is_connected());

    // Disconnecting when not connected should not fail
    let result = session.disconnect();
    assert!(result.is_ok());
    assert!(!session.is_connected());
}

#[test]
fn test_tcp_attach_config_edge_cases() {
    // Test with IPv6 address
    let config = TcpAttachConfig::new("::1".to_string(), 13603);
    assert!(config.validate().is_ok());

    // Test with hostname
    let config = TcpAttachConfig::new("example.com".to_string(), 13603);
    assert!(config.validate().is_ok());

    // Test with IP address
    let config = TcpAttachConfig::new("192.168.1.1".to_string(), 13603);
    assert!(config.validate().is_ok());

    // Test with maximum valid port
    let config = TcpAttachConfig::new("localhost".to_string(), 65535);
    assert!(config.validate().is_ok());

    // Test with minimum valid timeout
    let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(1);
    assert!(config.validate().is_ok());

    // Test with maximum valid timeout
    let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(300_000);
    assert!(config.validate().is_ok());
}

#[test]
fn test_tcp_attach_config_whitespace_handling() {
    // Test with whitespace in host - should be trimmed and valid
    let config = TcpAttachConfig::new("  localhost  ".to_string(), 13603);
    // The validation trims whitespace, so this should be valid
    assert!(config.validate().is_ok());

    // Test with only whitespace - should be invalid after trimming
    let config = TcpAttachConfig::new("   ".to_string(), 13603);
    assert!(config.validate().is_err());
}

#[test]
fn test_tcp_attach_default_implementation() {
    // Test Default trait implementation
    let session1 = TcpAttachSession::new();
    let session2 = TcpAttachSession::default();

    // Both should be disconnected initially
    assert!(!session1.is_connected());
    assert!(!session2.is_connected());
}

#[test]
fn test_tcp_attach_event_serialization() {
    // Test that events can be cloned and sent through channels
    let (tx, rx) = channel::<DapEvent>();

    let original =
        DapEvent::Output { category: "stderr".to_string(), output: "error message".to_string() };

    // Clone and send
    must(tx.send(original.clone()));

    let received = must(rx.recv_timeout(Duration::from_millis(100)));
    match received {
        DapEvent::Output { category, output } => {
            assert_eq!(category, "stderr");
            assert_eq!(output, "error message");
        }
        _ => must(Err::<(), _>("Expected Output event")),
    }
}

#[test]
fn test_tcp_attach_reader_handles_concatenated_frames() {
    let listener = must(TcpListener::bind(("127.0.0.1", 0)));
    let port = must(listener.local_addr()).port();

    let server_handle = thread::spawn(move || {
        let (mut socket, _) = must(listener.accept());

        let output_event = serde_json::json!({
            "type": "event",
            "seq": 1,
            "event": "output",
            "body": {
                "category": "stdout",
                "output": "hello"
            }
        })
        .to_string();
        let continued_event = serde_json::json!({
            "type": "event",
            "seq": 2,
            "event": "continued",
            "body": {
                "threadId": 7
            }
        })
        .to_string();

        let mut bytes = frame(output_event.as_bytes());
        bytes.extend_from_slice(&frame(continued_event.as_bytes()));
        must(socket.write_all(&bytes));
        must(socket.flush());
    });

    let mut session = TcpAttachSession::new();
    let (event_tx, event_rx) = channel::<DapEvent>();
    session.set_event_sender(event_tx);

    let config = TcpAttachConfig::new("127.0.0.1".to_string(), port).with_timeout(2000);
    must(session.connect(&config));
    must(session.start_reader());

    let first = must(event_rx.recv_timeout(Duration::from_secs(2)));
    let second = must(event_rx.recv_timeout(Duration::from_secs(2)));

    match first {
        DapEvent::Output { category, output } => {
            assert_eq!(category, "stdout");
            assert_eq!(output, "hello");
        }
        other => must(Err::<(), _>(format!("Expected Output event, got {other:?}"))),
    }

    match second {
        DapEvent::Continued { thread_id } => {
            assert_eq!(thread_id, 7);
        }
        other => must(Err::<(), _>(format!("Expected Continued event, got {other:?}"))),
    }

    must(server_handle.join().map_err(|_| "Server thread panicked".to_string()));
}

#[test]
fn test_tcp_attach_connect_timeout_reports_address() {
    // Port 9 (discard) is expected to be closed in local test environments.
    let mut session = TcpAttachSession::new();
    let config = TcpAttachConfig::new("127.0.0.1".to_string(), 9).with_timeout(50);

    let err = must(session.connect(&config).err().ok_or("connect unexpectedly succeeded"));
    let message = err.to_string();
    assert!(message.contains("127.0.0.1:9"), "error should include address: {message}");
}

#[test]
fn test_tcp_attach_reader_emits_terminated_when_remote_closes() {
    let listener = must(TcpListener::bind(("127.0.0.1", 0)));
    let port = must(listener.local_addr()).port();

    let server_handle = thread::spawn(move || {
        let (socket, _) = must(listener.accept());
        drop(socket);
    });

    let mut session = TcpAttachSession::new();
    let (event_tx, event_rx) = channel::<DapEvent>();
    session.set_event_sender(event_tx);
    let config = TcpAttachConfig::new("127.0.0.1".to_string(), port).with_timeout(2000);

    must(session.connect(&config));
    must(session.start_reader());

    let event = must(event_rx.recv_timeout(Duration::from_secs(2)));
    match event {
        DapEvent::Terminated { reason } => {
            assert_eq!(reason, "connection_closed");
        }
        other => must(Err::<(), _>(format!("Expected Terminated event, got {other:?}"))),
    }

    must(server_handle.join().map_err(|_| "Server thread panicked".to_string()));
}

#[test]
fn test_tcp_attach_can_reconnect_after_disconnect() {
    let first_listener = must(TcpListener::bind(("127.0.0.1", 0)));
    let first_port = must(first_listener.local_addr()).port();
    let first_server = thread::spawn(move || {
        let (socket, _) = must(first_listener.accept());
        drop(socket);
    });

    let second_listener = must(TcpListener::bind(("127.0.0.1", 0)));
    let second_port = must(second_listener.local_addr()).port();
    let second_server = thread::spawn(move || {
        let (socket, _) = must(second_listener.accept());
        drop(socket);
    });

    let mut session = TcpAttachSession::new();
    let first_config = TcpAttachConfig::new("127.0.0.1".to_string(), first_port).with_timeout(2000);
    must(session.connect(&first_config));
    assert!(session.is_connected());

    must(session.disconnect());
    assert!(!session.is_connected());

    let second_config =
        TcpAttachConfig::new("127.0.0.1".to_string(), second_port).with_timeout(2000);
    must(session.connect(&second_config));
    assert!(session.is_connected());

    must(session.disconnect());
    assert!(!session.is_connected());

    must(first_server.join().map_err(|_| "First server panicked".to_string()));
    must(second_server.join().map_err(|_| "Second server panicked".to_string()));
}
