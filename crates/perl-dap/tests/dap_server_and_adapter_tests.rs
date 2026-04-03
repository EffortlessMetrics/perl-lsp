//! Tests for DapMode, DapConfig, DapServer, TcpAttachConfig,
//! TcpAttachSession, DapEvent, and BridgeAdapter.
//!
//! These tests verify the public API surfaces of the top-level DAP server
//! types and supporting adapter infrastructure without requiring a live
//! Perl debugger process.

use perl_dap::tcp_attach::{DapEvent, TcpAttachConfig, TcpAttachSession};
use perl_dap::{BridgeAdapter, DapConfig, DapMode, DapServer};
use std::time::Duration;

// ── DapMode ────────────────────────────────────────────────────────

#[test]
fn dap_mode_default_is_native() {
    assert_eq!(DapMode::default(), DapMode::Native);
}

#[test]
fn dap_mode_clone_and_eq() {
    let mode = DapMode::Bridge;
    let cloned = mode.clone();
    assert_eq!(mode, cloned);
    assert_ne!(DapMode::Native, DapMode::Bridge);
}

#[test]
fn dap_mode_debug_format() {
    let debug_str = format!("{:?}", DapMode::Native);
    assert!(debug_str.contains("Native"));
    let debug_str = format!("{:?}", DapMode::Bridge);
    assert!(debug_str.contains("Bridge"));
}

// ── DapServer ──────────────────────────────────────────────────────

#[test]
fn dap_server_creation_native() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        DapConfig { log_level: "info".to_string(), mode: DapMode::Native, workspace_root: None };
    let server = DapServer::new(config)?;
    assert_eq!(server.config.mode, DapMode::Native);
    assert_eq!(server.config.log_level, "info");
    assert!(server.config.workspace_root.is_none());
    Ok(())
}

#[test]
fn dap_server_creation_bridge() -> Result<(), Box<dyn std::error::Error>> {
    let config = DapConfig {
        log_level: "debug".to_string(),
        mode: DapMode::Bridge,
        workspace_root: Some(std::path::PathBuf::from("/workspace")),
    };
    let server = DapServer::new(config)?;
    assert_eq!(server.config.mode, DapMode::Bridge);
    assert_eq!(server.config.workspace_root, Some(std::path::PathBuf::from("/workspace")));
    Ok(())
}

#[test]
fn dap_server_socket_rejects_bridge_mode() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        DapConfig { log_level: "info".to_string(), mode: DapMode::Bridge, workspace_root: None };
    let mut server = DapServer::new(config)?;
    let result = server.run_socket(9999);
    assert!(result.is_err(), "Socket transport should be rejected in bridge mode");
    let err_msg = result.err().ok_or("Expected error")?.to_string();
    assert!(err_msg.contains("not supported"), "Error should mention lack of support: {err_msg}");
    Ok(())
}

// ── TcpAttachConfig ────────────────────────────────────────────────

#[test]
fn tcp_attach_config_builder_pattern() {
    let config = TcpAttachConfig::new("192.168.1.1".to_string(), 9000).with_timeout(10000);
    assert_eq!(config.host, "192.168.1.1");
    assert_eq!(config.port, 9000);
    assert_eq!(config.timeout_ms, Some(10000));
}

#[test]
fn tcp_attach_config_default_timeout_duration() {
    let config = TcpAttachConfig::new("localhost".to_string(), 13603);
    assert_eq!(config.timeout_duration(), Duration::from_millis(5000));
}

#[test]
fn tcp_attach_config_custom_timeout_duration() {
    let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(15000);
    assert_eq!(config.timeout_duration(), Duration::from_millis(15000));
}

#[test]
fn tcp_attach_config_validate_whitespace_host() {
    let config = TcpAttachConfig::new("   ".to_string(), 13603);
    assert!(config.validate().is_err(), "Whitespace-only host should be rejected");
}

#[test]
fn tcp_attach_config_validate_port_1_is_valid() {
    let config = TcpAttachConfig::new("localhost".to_string(), 1);
    assert!(config.validate().is_ok());
}

#[test]
fn tcp_attach_config_validate_max_port_is_valid() {
    let config = TcpAttachConfig::new("localhost".to_string(), 65535);
    assert!(config.validate().is_ok());
}

#[test]
fn tcp_attach_config_validate_boundary_timeout() {
    // At 300_000 (5 min) should be valid
    let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(300_000);
    assert!(config.validate().is_ok());

    // At 300_001 should fail
    let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(300_001);
    assert!(config.validate().is_err());
}

#[test]
fn tcp_attach_config_validate_1ms_timeout() {
    let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(1);
    assert!(config.validate().is_ok());
}

// ── TcpAttachSession ───────────────────────────────────────────────

#[test]
fn tcp_attach_session_default_is_disconnected() {
    let session = TcpAttachSession::default();
    assert!(!session.is_connected());
}

#[test]
fn tcp_attach_session_send_message_without_connection_fails() {
    let mut session = TcpAttachSession::new();
    let result = session.send_message(r#"{"seq":1,"type":"request","command":"initialize"}"#);
    assert!(result.is_err(), "Should fail when not connected");
}

#[test]
fn tcp_attach_session_disconnect_when_not_connected_is_ok() {
    let mut session = TcpAttachSession::new();
    let result = session.disconnect();
    assert!(result.is_ok(), "Disconnecting when not connected should be fine");
}

#[test]
fn tcp_attach_session_start_reader_without_connection_fails() {
    let mut session = TcpAttachSession::new();
    let result = session.start_reader();
    assert!(result.is_err(), "Starting reader without connection should fail");
}

#[test]
fn tcp_attach_session_connect_to_invalid_host_fails() {
    let mut session = TcpAttachSession::new();
    // Use a very short timeout to fail fast
    let config = TcpAttachConfig::new("192.0.2.1".to_string(), 59999).with_timeout(100);
    let result = session.connect(&config);
    assert!(result.is_err(), "Connecting to unreachable host should fail");
}

#[test]
fn tcp_attach_session_connect_with_invalid_config_fails() {
    let mut session = TcpAttachSession::new();
    let config = TcpAttachConfig::new("".to_string(), 0);
    let result = session.connect(&config);
    assert!(result.is_err(), "Should fail validation before attempting connection");
}

// ── DapEvent ───────────────────────────────────────────────────────

#[test]
fn dap_event_output_debug_format() {
    let event =
        DapEvent::Output { category: "stdout".to_string(), output: "Hello World\n".to_string() };
    let debug = format!("{:?}", event);
    assert!(debug.contains("Output"));
    assert!(debug.contains("stdout"));
}

#[test]
fn dap_event_stopped_debug_format() {
    let event = DapEvent::Stopped { reason: "breakpoint".to_string(), thread_id: 1 };
    let debug = format!("{:?}", event);
    assert!(debug.contains("Stopped"));
    assert!(debug.contains("breakpoint"));
}

#[test]
fn dap_event_continued_debug_format() {
    let event = DapEvent::Continued { thread_id: 1 };
    let debug = format!("{:?}", event);
    assert!(debug.contains("Continued"));
}

#[test]
fn dap_event_terminated_debug_format() {
    let event = DapEvent::Terminated { reason: "exited".to_string() };
    let debug = format!("{:?}", event);
    assert!(debug.contains("Terminated"));
    assert!(debug.contains("exited"));
}

#[test]
fn dap_event_error_debug_format() {
    let event = DapEvent::Error { message: "connection lost".to_string() };
    let debug = format!("{:?}", event);
    assert!(debug.contains("Error"));
    assert!(debug.contains("connection lost"));
}

#[test]
fn dap_event_clone() {
    let event = DapEvent::Stopped { reason: "step".to_string(), thread_id: 2 };
    let cloned = event.clone();
    let debug_original = format!("{:?}", event);
    let debug_cloned = format!("{:?}", cloned);
    assert_eq!(debug_original, debug_cloned);
}

// ── BridgeAdapter ──────────────────────────────────────────────────

#[test]
fn bridge_adapter_creation() {
    let adapter = BridgeAdapter::new();
    // BridgeAdapter::new() should succeed without panicking
    let debug = format!("{:?}", "BridgeAdapter created");
    assert!(!debug.is_empty());
    drop(adapter);
}

#[test]
fn bridge_adapter_default_creation() {
    let adapter = BridgeAdapter::default();
    // Default should be equivalent to new()
    drop(adapter);
}
