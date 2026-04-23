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
