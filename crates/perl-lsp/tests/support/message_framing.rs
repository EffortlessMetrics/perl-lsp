//! Message framing and classification for the LSP test harness.
//!
//! Contains `TestWriter`, a `std::io::Write` implementation that captures
//! server output, classifies JSON-RPC messages into notifications vs.
//! server-initiated requests, and signals waiters when new data arrives.

#![allow(dead_code)]

use parking_lot::{Condvar, Mutex};
use perl_lsp::LspServer;
use serde_json::Value;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;

/// Wrapper to send `LspServer` across thread boundaries.
pub(super) struct SendableServer(pub(super) LspServer);

// SAFETY: `LspServer` is only accessed from the single server thread
// spawned in `LspHarness::new_raw`.
unsafe impl Send for SendableServer {}

/// Test writer that captures server output and classifies messages.
///
/// Every byte written by the LSP server flows through this writer. In
/// addition to buffering the raw bytes (so that `LspHarness` can apply
/// content-length framing), the writer eagerly parses the payload and
/// routes server-initiated notifications and requests into dedicated
/// queues so that `drain_notifications` / `wait_for_notification` can
/// consume them without re-parsing.
pub(super) struct TestWriter {
    pub(super) buffer: Arc<Mutex<Vec<u8>>>,
    pub(super) signal: Arc<Condvar>,
    pub(super) notifications: Arc<Mutex<VecDeque<Value>>>,
    pub(super) server_requests: Arc<Mutex<VecDeque<Value>>>,
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        {
            let mut buffer = self.buffer.lock();
            buffer.extend_from_slice(buf);
        }
        // Parse and classify message outside buffer lock to avoid contention
        let content = String::from_utf8_lossy(buf);
        if let Some(json_start) = content.find('{') {
            let json_str = &content[json_start..];
            if let Ok(value) = serde_json::from_str::<Value>(json_str) {
                let has_method = value.get("method").is_some();
                let has_id = value.get("id").is_some();
                if has_method && !has_id {
                    // Server-initiated notification (no id)
                    self.notifications.lock().push_back(value);
                } else if has_method && has_id {
                    // Server-initiated request (e.g., workspace/configuration)
                    self.server_requests.lock().push_back(value);
                }
                // Responses (has id, no method) stay in the raw buffer only
            }
        }
        self.signal.notify_all();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
