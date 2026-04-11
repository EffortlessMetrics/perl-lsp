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
/// content-length framing), the writer eagerly parses ALL framed messages
/// in each write call and routes server-initiated notifications and requests
/// into dedicated queues so that `drain_notifications` / `wait_for_notification`
/// can consume them without re-parsing.
///
/// # Correctness note
///
/// The outbound writer may coalesce multiple messages into a single `write()`
/// call (batched I/O optimisation).  Previously the classification only parsed
/// the FIRST JSON object it found, silently discarding subsequent messages in
/// the same batch.  The fix is to use a `ContentLengthFramer` to extract every
/// complete frame from the incoming bytes before classifying.
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

        // Parse and classify ALL framed messages in this batch.
        // Using a ContentLengthFramer ensures that when the outbound writer
        // coalesces multiple messages into one write() call, every message
        // gets classified — not just the first one.
        let mut framer = ContentLengthFramer::new();
        framer.push(buf);

        loop {
            match framer.try_next() {
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
                Ok(None) => break, // No more complete frames in this batch
                Err(_) => break,   // Framing error — stop parsing this batch
            }
        }

        self.signal.notify_all();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
