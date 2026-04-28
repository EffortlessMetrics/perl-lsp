//! LSP protocol I/O helpers for reading and writing JSON-RPC messages.
//!
//! Provides low-level message framing, response matching, notification filtering,
//! and drain utilities used by the test harness.

use serde_json::{Value, json};
use std::io::{self, Write};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use super::{LspServer, PENDING_CAP};

// Error codes aligned with crates/perl-parser/src/lsp/protocol/errors.rs
/// JSON-RPC reserved: Server error range is -32000 to -32099
pub(crate) const ERR_TEST_TIMEOUT: i64 = -32000;
/// Connection closed - BrokenPipe or similar transport termination
const ERR_CONNECTION_CLOSED: i64 = -32050;
/// Transport error - general I/O failures that aren't connection closures
const ERR_TRANSPORT_ERROR: i64 = -32051;

/// Helper function to send a JSON-RPC message over the wire.
/// Returns io::Result to allow graceful error handling.
pub(crate) fn send_message_inner(writer: &mut impl Write, body: &str) -> io::Result<()> {
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    writer.flush()
}

/// Creates a JSON-RPC 2.0 error response with proper envelope.
///
/// All error responses MUST include `jsonrpc` and `id` fields per JSON-RPC 2.0 spec.
/// The `id` should be extracted from the original request before any error handling.
pub(crate) fn error_response_for_request(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// Creates an error response for connection-closed scenarios (BrokenPipe).
fn connection_closed_error_for_request(id: Option<Value>) -> Value {
    error_response_for_request(id, ERR_CONNECTION_CLOSED, "Connection closed")
}

/// Creates an error response for internal transport errors.
fn transport_error_for_request(id: Option<Value>, msg: &str) -> Value {
    error_response_for_request(id, ERR_TRANSPORT_ERROR, msg)
}

// Legacy functions for backward compatibility with code that doesn't have request context
// These return responses with null id (valid JSON-RPC but less informative)

/// Creates an error response for connection-closed scenarios (legacy, no request context).
pub(crate) fn connection_closed_error() -> Value {
    connection_closed_error_for_request(None)
}

/// Creates an error response for internal transport errors (legacy, no request context).
pub(crate) fn internal_transport_error(msg: &str) -> Value {
    transport_error_for_request(None, msg)
}

/// Maps I/O send errors to proper JSON-RPC error responses.
///
/// BrokenPipe -> CONNECTION_CLOSED (-32050)
/// Other I/O errors -> TRANSPORT_ERROR (-32051)
pub(crate) fn map_send_error(id: Option<Value>, e: io::Error, context: &str) -> Value {
    if e.kind() == io::ErrorKind::BrokenPipe {
        connection_closed_error_for_request(id)
    } else {
        transport_error_for_request(id, &format!("{}: {}", context, e))
    }
}

/// Blocking receive with a sane default timeout to avoid hangs.
pub fn read_response(server: &LspServer) -> Value {
    read_response_timeout(server, super::default_timeout()).unwrap_or_else(
        || json!({"error":{"code":ERR_TEST_TIMEOUT,"message":"test harness timeout"}}),
    )
}

/// Try to receive a response within `dur`. Returns None on timeout.
pub fn read_response_timeout(server: &LspServer, dur: Duration) -> Option<Value> {
    server.rx.lock().unwrap_or_else(|e| e.into_inner()).recv_timeout(dur).ok()
}

/// Try to receive a JSON-RPC response within `dur`, buffering notifications and
/// unrelated traffic for later reads.
pub fn read_response_only_timeout(server: &LspServer, dur: Duration) -> Option<Value> {
    // Scan buffered traffic first.
    {
        let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
        let len = pending.len();
        for _ in 0..len {
            if let Some(msg) = pending.pop_front() {
                if msg.get("id").is_some() {
                    return Some(msg);
                }
                if pending.len() >= PENDING_CAP {
                    pending.pop_front();
                }
                pending.push_back(msg);
            }
        }
    }

    let deadline = Instant::now() + dur;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        let recv_result = {
            let rx = server.rx.lock().unwrap_or_else(|e| e.into_inner());
            rx.recv_timeout(deadline - now)
        };
        match recv_result {
            Ok(msg) => {
                if msg.get("id").is_some() {
                    return Some(msg);
                }
                let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
                if pending.len() >= PENDING_CAP {
                    pending.pop_front();
                }
                pending.push_back(msg);
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => return None,
        }
    }
}

/// Try to receive a notification (message without id) within `dur`. Returns None on timeout or if a response is received.
pub fn read_notification_timeout(server: &LspServer, dur: Duration) -> Option<Value> {
    match server.rx.lock().unwrap_or_else(|e| e.into_inner()).recv_timeout(dur) {
        Ok(val) if val.get("id").is_none() => Some(val),
        Ok(_) => None,  // Got a response, not a notification
        Err(_) => None, // Timeout or disconnected
    }
}

/// Receive the response matching `id` (number or string), buffering other traffic.
pub fn read_response_matching(server: &LspServer, id: &Value, dur: Duration) -> Option<Value> {
    // scan buffered
    {
        let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
        let len = pending.len();
        for _ in 0..len {
            if let Some(msg) = pending.pop_front() {
                if msg.get("id") == Some(id) {
                    return Some(msg);
                }
                if pending.len() >= PENDING_CAP {
                    pending.pop_front();
                }
                pending.push_back(msg);
            }
        }
    }
    // then poll
    let deadline = Instant::now() + dur;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        let recv_result = {
            let rx = server.rx.lock().unwrap_or_else(|e| e.into_inner());
            rx.recv_timeout(deadline - now)
        };
        match recv_result {
            Ok(msg) => {
                if msg.get("id") == Some(id) {
                    return Some(msg);
                }
                let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
                if pending.len() >= PENDING_CAP {
                    pending.pop_front();
                }
                pending.push_back(msg);
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => return None,
        }
    }
}

/// Convenience for numeric ids.
pub fn read_response_matching_i64(server: &LspServer, id: i64, dur: Duration) -> Option<Value> {
    read_response_matching(server, &json!(id), dur)
}

/// Write raw bytes (for malformed/binary frame tests).
pub fn send_raw(server: &LspServer, bytes: &[u8]) {
    // Ignore write errors - BrokenPipe during teardown is expected
    let mut writer = server.writer.lock().unwrap_or_else(|e| e.into_inner());
    let _ = writer.write_all(bytes).and_then(|_| writer.flush());
}

/// Read a notification matching the given method name
pub fn read_notification_method(server: &LspServer, method: &str, dur: Duration) -> Option<Value> {
    let deadline = Instant::now() + dur;

    // scan buffered first
    {
        let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
        let len = pending.len();
        for _ in 0..len {
            if let Some(msg) = pending.pop_front() {
                if msg.get("id").is_none() && msg.get("method") == Some(&json!(method)) {
                    return Some(msg);
                }
                pending.push_back(msg);
            }
        }
    }

    // then poll
    while Instant::now() < deadline {
        let recv_result = {
            let rx = server.rx.lock().unwrap_or_else(|e| e.into_inner());
            rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        };
        match recv_result {
            Ok(msg) => {
                let is_match = msg.get("id").is_none() && msg.get("method") == Some(&json!(method));
                if is_match {
                    return Some(msg);
                }
                let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
                if pending.len() >= PENDING_CAP {
                    pending.pop_front();
                }
                pending.push_back(msg);
            }
            Err(_) => break,
        }
    }
    None
}

/// Drain messages until no traffic for a quiet period (stabilizes CI)
pub fn drain_until_quiet(server: &LspServer, quiet: Duration, ceiling: Duration) {
    let start = Instant::now();
    let mut last = Instant::now();
    while start.elapsed() < ceiling {
        let recv_result = {
            let rx = server.rx.lock().unwrap_or_else(|e| e.into_inner());
            rx.recv_timeout(quiet.saturating_sub(last.elapsed()))
        };
        match recv_result {
            Ok(msg) => {
                let mut pending = server.pending.lock().unwrap_or_else(|e| e.into_inner());
                if pending.len() >= PENDING_CAP {
                    pending.pop_front();
                }
                pending.push_back(msg);
                last = Instant::now();
            }
            Err(_) => break, // quiet period achieved
        }
    }
}

/// Send raw message to server (for testing malformed input)
pub fn send_raw_message(server: &LspServer, content: &str) {
    // Ignore write errors - BrokenPipe during teardown is expected
    let _ =
        send_message_inner(&mut *server.writer.lock().unwrap_or_else(|e| e.into_inner()), content);
}

/// Send request without waiting for response
pub fn send_request_no_wait(server: &LspServer, req: Value) {
    let body = req.to_string();
    // Ignore write errors - BrokenPipe during teardown is expected
    let _ =
        send_message_inner(&mut *server.writer.lock().unwrap_or_else(|e| e.into_inner()), &body);
}
