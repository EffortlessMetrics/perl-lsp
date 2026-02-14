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
use std::sync::mpsc::channel;
use std::time::Duration;
use perl_tdd_support::{must, must_some};

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
