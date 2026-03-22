//! Message framing and classification for the LSP test harness.
//!
//! Contains `TestWriter`, a `std::io::Write` implementation that captures
//! server output, classifies JSON-RPC messages into notifications vs.
//! server-initiated requests, and signals waiters when new data arrives.

#![allow(dead_code)]

use parking_lot::{Condvar, Mutex};
use perl_content_length_framing::ContentLengthFramer;
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
///
/// The writer uses `ContentLengthFramer` to correctly extract ALL messages
/// from a batch write, handling the case where the outbound writer coalesces
/// multiple messages (e.g., `telemetry/event` + `publishDiagnostics`) into a
/// single `write()` call.
pub(super) struct TestWriter {
    pub(super) buffer: Arc<Mutex<Vec<u8>>>,
    pub(super) signal: Arc<Condvar>,
    pub(super) notifications: Arc<Mutex<VecDeque<Value>>>,
    pub(super) server_requests: Arc<Mutex<VecDeque<Value>>>,
    framer: ContentLengthFramer,
}

impl TestWriter {
    pub(super) fn new(
        buffer: Arc<Mutex<Vec<u8>>>,
        signal: Arc<Condvar>,
        notifications: Arc<Mutex<VecDeque<Value>>>,
        server_requests: Arc<Mutex<VecDeque<Value>>>,
    ) -> Self {
        Self { buffer, signal, notifications, server_requests, framer: ContentLengthFramer::new() }
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        {
            let mut buffer = self.buffer.lock();
            buffer.extend_from_slice(buf);
        }

        // Feed the incoming bytes into the content-length framer so that ALL
        // messages in a batched write are properly extracted and classified.
        // Previously this only parsed a single JSON value from the raw bytes,
        // which silently dropped the second message in any coalesced batch.
        self.framer.push(buf);
        loop {
            match self.framer.try_next() {
                Ok(Some(body)) => {
                    if let Ok(value) = serde_json::from_slice::<Value>(&body) {
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
                Ok(None) => break,
                Err(e) => {
                    eprintln!("TestWriter framing error: {e}");
                    break;
                }
            }
        }

        self.signal.notify_all();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
