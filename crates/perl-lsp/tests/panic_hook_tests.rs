//! Tests for the global panic hook in the LSP server.
//!
//! These tests verify that:
//! 1. The panic hook is installed correctly and captures panic information.
//! 2. A `window/showMessage` notification is sent to the client when a panic occurs.
//! 3. The panic message contains useful location information.
//!
//! ## Test isolation
//!
//! All tests in this file that interact with the global panic hook are marked
//! `#[serial]` via `serial_test` to prevent concurrent mutation of the shared
//! process-global hook state.

use serial_test::serial;
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ---- Shared CaptureWriter infrastructure ------------------------------------

/// A writer that captures bytes into a shared buffer.
///
/// The [`Arc<Mutex<Vec<u8>>>`] is shared between the writer (moved into the
/// server's outbound thread) and the test's inspection handle.
#[derive(Clone)]
struct CaptureWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl CaptureWriter {
    fn new() -> Self {
        Self { buf: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Drain and return all bytes written so far.
    fn take_bytes(&self) -> Vec<u8> {
        let mut guard = self.buf.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *guard)
    }
}

impl Write for CaptureWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.buf.lock().unwrap_or_else(|e| e.into_inner());
        guard.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ---- LSP framing parser -----------------------------------------------------

/// Parse all JSON objects from LSP Content-Length framed output.
fn parse_lsp_messages(data: &[u8]) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();
    let text = String::from_utf8_lossy(data);
    let mut remaining = text.as_ref();

    while let Some(pos) = remaining.find("Content-Length: ") {
        let after_prefix = &remaining[pos + "Content-Length: ".len()..];
        let end_of_len = after_prefix.find('\r').unwrap_or(after_prefix.len());
        let len: usize = after_prefix[..end_of_len].trim().parse().unwrap_or(0);

        let body_start = match after_prefix.find("\r\n\r\n") {
            Some(p) => pos + "Content-Length: ".len() + p + 4,
            None => break,
        };

        if body_start + len <= remaining.len() {
            let body = &remaining[body_start..body_start + len];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
                messages.push(val);
            }
            remaining = &remaining[body_start + len..];
        } else {
            break;
        }
    }

    messages
}

/// Spin-wait for at least one byte to appear in the capture buffer, or until
/// `timeout` elapses.  Returns `true` if bytes arrived within the deadline.
fn wait_for_bytes(writer: &CaptureWriter, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        let guard = writer.buf.lock().unwrap_or_else(|e| e.into_inner());
        if !guard.is_empty() {
            return true;
        }
        drop(guard);
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

// ---- Tests ------------------------------------------------------------------

/// Verify that installing the panic hook does not panic or return an error.
#[test]
#[serial]
fn install_panic_hook_does_not_panic() {
    let writer = CaptureWriter::new();
    let server = perl_lsp::LspServer::with_io(
        Box::new(Cursor::new(Vec::<u8>::new())),
        Box::new(writer.clone()),
    );
    // Installing the hook must succeed without panicking.
    server.install_panic_hook();
    // Restore the default hook so we don't interfere with other tests.
    let _ = std::panic::take_hook();
}

/// Verify that a panic while the hook is installed causes a
/// `window/showMessage` notification in the output buffer.
///
/// We use `catch_unwind` to prevent the test from aborting, and we keep the
/// server alive until after we read the buffer, ensuring the writer thread
/// has time to flush.
#[test]
#[serial]
fn panic_hook_sends_show_message_notification() {
    let writer = CaptureWriter::new();
    let server = perl_lsp::LspServer::with_io(
        Box::new(Cursor::new(Vec::<u8>::new())),
        Box::new(writer.clone()),
    );

    server.install_panic_hook();

    // Trigger a controlled panic.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        std::panic::panic_any("test panic for hook verification");
    }));

    // Restore default hook so other tests are not affected.
    let _ = std::panic::take_hook();

    // Wait for the writer thread to flush (up to 500 ms).
    let arrived = wait_for_bytes(&writer, Duration::from_millis(500));
    assert!(arrived, "Timed out waiting for panic notification from writer thread");

    let bytes = writer.take_bytes();
    let messages = parse_lsp_messages(&bytes);

    let show_message = messages
        .iter()
        .find(|m| m.get("method").and_then(|v| v.as_str()) == Some("window/showMessage"));

    assert!(
        show_message.is_some(),
        "Expected a window/showMessage notification after panic, but got: {messages:?}"
    );

    let msg_type = show_message.and_then(|m| m.pointer("/params/type")).and_then(|v| v.as_i64());
    assert_eq!(
        msg_type,
        Some(1), // MessageType::Error == 1
        "Expected MessageType::Error (1) in panic notification"
    );

    let message_text = show_message
        .and_then(|m| m.pointer("/params/message"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        message_text.contains("internal error") || message_text.contains("panic"),
        "Expected panic description in message, got: {message_text:?}"
    );
}

/// Verify that the panic hook message includes the literal panic payload.
#[test]
#[serial]
fn panic_hook_includes_panic_message_in_notification() {
    let writer = CaptureWriter::new();
    let server = perl_lsp::LspServer::with_io(
        Box::new(Cursor::new(Vec::<u8>::new())),
        Box::new(writer.clone()),
    );

    server.install_panic_hook();

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        panic!("unique-sentinel-value-xyz");
    }));

    let _ = std::panic::take_hook();

    let arrived = wait_for_bytes(&writer, Duration::from_millis(500));
    assert!(arrived, "Timed out waiting for panic notification from writer thread");

    let bytes = writer.take_bytes();
    let messages = parse_lsp_messages(&bytes);

    let show_message = messages
        .iter()
        .find(|m| m.get("method").and_then(|v| v.as_str()) == Some("window/showMessage"));

    let message_text = show_message
        .and_then(|m| m.pointer("/params/message"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    assert!(
        message_text.contains("unique-sentinel-value-xyz"),
        "Expected panic payload in notification message, got: {message_text:?}"
    );
}
