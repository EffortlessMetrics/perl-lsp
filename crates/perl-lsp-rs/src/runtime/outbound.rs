//! Outbound message channel
//!
//! Decouples message serialization from I/O by sending outbound messages through
//! an unbounded channel to a dedicated writer thread. This eliminates the writer
//! lock as a contention point and enables concurrent handler execution.

use crate::protocol::JsonRpcResponse;
use crate::transport::frame;
use serde_json::{Value, json};
use std::io::{self, Write};
use std::thread;

/// An outbound LSP message (response, notification, or server→client request).
pub(crate) enum OutboundMessage {
    /// JSON-RPC response to a client request.
    Response(JsonRpcResponse),
    /// JSON-RPC notification (no id, no response expected).
    Notification { method: String, params: Value },
    /// JSON-RPC request from server to client (has id, expects response).
    Request { id: i64, method: String, params: Value },
}

/// Cloneable handle for sending outbound messages.
///
/// Multiple tasks/threads can hold a clone and send concurrently;
/// all messages are serialized by the single writer thread.
#[derive(Clone)]
pub(crate) struct OutboundSender {
    tx: tokio::sync::mpsc::UnboundedSender<OutboundMessage>,
}

impl OutboundSender {
    /// Send a JSON-RPC response.
    pub fn send_response(&self, response: JsonRpcResponse) -> io::Result<()> {
        self.tx
            .send(OutboundMessage::Response(response))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "outbound channel closed"))
    }

    /// Send a JSON-RPC notification.
    pub fn send_notification(&self, method: &str, params: Value) -> io::Result<()> {
        self.tx
            .send(OutboundMessage::Notification { method: method.to_string(), params })
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "outbound channel closed"))
    }

    /// Send a server→client JSON-RPC request.
    pub fn send_request(&self, id: i64, method: &str, params: Value) -> io::Result<()> {
        self.tx
            .send(OutboundMessage::Request { id, method: method.to_string(), params })
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "outbound channel closed"))
    }
}

/// Create an `OutboundSender` backed by a writer thread.
///
/// Returns the sender handle and a join-handle for the writer thread.
/// The writer thread runs until the last sender is dropped (channel closes).
pub(crate) fn spawn_writer(
    output: Box<dyn Write + Send>,
) -> (OutboundSender, thread::JoinHandle<()>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = thread::spawn(move || writer_loop_batched(rx, output));
    (OutboundSender { tx }, handle)
}

/// Create an `OutboundSender` backed by a shared `Arc<Mutex<Box<dyn Write + Send>>>`.
///
/// Backward-compatible variant for `with_output()` constructors.
pub(crate) fn spawn_writer_shared(
    output: std::sync::Arc<parking_lot::Mutex<Box<dyn Write + Send>>>,
) -> (OutboundSender, thread::JoinHandle<()>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = thread::spawn(move || writer_loop_batched_shared(rx, output));
    (OutboundSender { tx }, handle)
}

/// Create an already-closed sender for shutdown replacement paths.
pub(crate) fn closed_sender() -> OutboundSender {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    drop(rx);
    OutboundSender { tx }
}

/// Blocking receive loop with message batching.
///
/// Drains the channel and writes all immediately-available messages
/// in a single write+flush cycle, reducing syscalls under burst load.
fn writer_loop_batched(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<OutboundMessage>,
    mut output: Box<dyn Write + Send>,
) {
    let mut batch_buf = Vec::with_capacity(4096);
    while let Some(msg) = rx.blocking_recv() {
        // Serialize first message.
        let bytes = serialize_message(&msg);
        let framed = frame(&bytes);
        batch_buf.extend_from_slice(&framed);

        // Drain any immediately available messages (coalescing).
        while let Ok(msg) = rx.try_recv() {
            let bytes = serialize_message(&msg);
            let framed = frame(&bytes);
            batch_buf.extend_from_slice(&framed);
        }

        // Single write+flush for the whole batch.
        if output.write_all(&batch_buf).is_err() {
            break;
        }
        if output.flush().is_err() {
            break;
        }
        batch_buf.clear();
    }
}

/// Blocking receive loop with message batching for shared writer.
///
/// Same coalescing strategy as [`writer_loop_batched`] but acquires the
/// shared lock once per batch rather than once per message.
fn writer_loop_batched_shared(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<OutboundMessage>,
    output: std::sync::Arc<parking_lot::Mutex<Box<dyn Write + Send>>>,
) {
    let mut batch_buf = Vec::with_capacity(4096);
    while let Some(msg) = rx.blocking_recv() {
        // Serialize first message.
        let bytes = serialize_message(&msg);
        let framed = frame(&bytes);
        batch_buf.extend_from_slice(&framed);

        // Drain any immediately available messages (coalescing).
        while let Ok(msg) = rx.try_recv() {
            let bytes = serialize_message(&msg);
            let framed = frame(&bytes);
            batch_buf.extend_from_slice(&framed);
        }

        // Acquire lock once for the entire batch.
        let mut out = output.lock();
        if out.write_all(&batch_buf).is_err() {
            break;
        }
        if out.flush().is_err() {
            break;
        }
        drop(out);
        batch_buf.clear();
    }
}

/// Serialize an `OutboundMessage` to JSON bytes.
///
/// Returns an empty `Vec` on serialization failure and logs the error via
/// `tracing::error!` so callers have diagnostic visibility rather than a
/// silently-malformed empty frame being delivered to the client.
fn serialize_message(msg: &OutboundMessage) -> Vec<u8> {
    match msg {
        OutboundMessage::Response(resp) => serde_json::to_vec(resp).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize outbound response: {e}");
            Vec::new()
        }),
        OutboundMessage::Notification { method, params } => {
            let val = json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
            });
            serde_json::to_vec(&val).unwrap_or_else(|e| {
                tracing::error!(method = %method, "Failed to serialize outbound notification: {e}");
                Vec::new()
            })
        }
        OutboundMessage::Request { id, method, params } => {
            let val = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            });
            serde_json::to_vec(&val).unwrap_or_else(|e| {
                tracing::error!(id = %id, method = %method, "Failed to serialize outbound request: {e}");
                Vec::new()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::error::Error;
    use std::io::Write;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    struct SharedBuffer {
        inner: Arc<parking_lot::Mutex<Vec<u8>>>,
    }

    impl SharedBuffer {
        fn new() -> Self {
            Self::default()
        }

        fn bytes(&self) -> Vec<u8> {
            self.inner.lock().clone()
        }
    }

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn parse_framed_payloads(raw: &[u8]) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
        let mut cursor = 0usize;
        let mut payloads = Vec::new();

        while cursor < raw.len() {
            let remainder = &raw[cursor..];
            let separator = b"\r\n\r\n";
            let Some(header_end) = remainder.windows(separator.len()).position(|w| w == separator)
            else {
                return Err("missing frame header separator".into());
            };
            let header_bytes = &remainder[..header_end];
            let header = std::str::from_utf8(header_bytes)?;
            let content_length = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .ok_or("missing Content-Length header")?
                .parse::<usize>()?;

            let body_start = cursor + header_end + separator.len();
            let body_end = body_start + content_length;
            if body_end > raw.len() {
                return Err("framed body shorter than declared Content-Length".into());
            }
            let body = &raw[body_start..body_end];
            payloads.push(serde_json::from_slice(body)?);
            cursor = body_end;
        }

        Ok(payloads)
    }

    #[test]
    fn closed_sender_returns_broken_pipe_for_all_message_types() {
        let sender = closed_sender();
        let response_result =
            sender.send_response(JsonRpcResponse::success(Some(json!(1)), json!({})));
        let notification_result = sender.send_notification("window/logMessage", json!({"x": 1}));
        let request_result = sender.send_request(7, "workspace/configuration", json!({}));

        for result in [response_result, notification_result, request_result] {
            match result {
                Ok(()) => panic!("expected BrokenPipe error when outbound channel is closed"),
                Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe),
            }
        }
    }

    #[test]
    fn spawn_writer_serializes_response_notification_and_request() -> Result<(), Box<dyn Error>> {
        let buffer = SharedBuffer::new();
        let (sender, handle) = spawn_writer(Box::new(buffer.clone()));

        sender.send_response(JsonRpcResponse::success(Some(json!(11)), json!({"ok": true})))?;
        sender.send_notification("window/logMessage", json!({"type": 3, "message": "hello"}))?;
        sender.send_request(42, "workspace/configuration", json!({"items": []}))?;

        drop(sender);
        handle.join().map_err(|_| "writer thread panicked")?;

        let payloads = parse_framed_payloads(&buffer.bytes())?;
        assert_eq!(payloads.len(), 3);
        assert_eq!(payloads[0]["id"], 11);
        assert_eq!(payloads[0]["result"]["ok"], true);
        assert_eq!(payloads[1]["method"], "window/logMessage");
        assert_eq!(payloads[2]["id"], 42);
        assert_eq!(payloads[2]["method"], "workspace/configuration");

        Ok(())
    }

    #[test]
    fn spawn_writer_shared_serializes_payloads() -> Result<(), Box<dyn Error>> {
        let buffer = SharedBuffer::new();
        let shared =
            Arc::new(parking_lot::Mutex::new(Box::new(buffer.clone()) as Box<dyn Write + Send>));

        let (sender, handle) = spawn_writer_shared(Arc::clone(&shared));
        sender.send_notification("telemetry/event", json!({"name": "batch"}))?;
        sender.send_request(9, "client/registerCapability", json!({"registrations": []}))?;

        drop(sender);
        handle.join().map_err(|_| "writer thread panicked")?;

        let payloads = parse_framed_payloads(&buffer.bytes())?;
        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0]["method"], "telemetry/event");
        assert_eq!(payloads[1]["id"], 9);
        assert_eq!(payloads[1]["method"], "client/registerCapability");

        Ok(())
    }
}
