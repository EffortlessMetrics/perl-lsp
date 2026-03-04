//! Integration tests for `perl-lsp-transport`.
//!
//! These tests exercise the public API through in-memory byte streams,
//! verifying framing, parsing, error handling, and round-trip behaviour
//! without any real I/O.

use perl_lsp_protocol::{JsonRpcError, JsonRpcResponse};
use perl_lsp_transport::{
    ContentLengthMessageReader, log_response, read_message, write_message, write_notification,
};
use std::io::{self, BufReader, Cursor};

// ── helpers ────────────────────────────────────────────────────────

/// Build a Content-Length framed JSON-RPC request.
fn framed_request(id: u64, method: &str) -> Vec<u8> {
    let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{}}}}"#);
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    frame
}

/// Build a Content-Length framed JSON-RPC request with custom params.
fn framed_request_with_params(id: u64, method: &str, params: &str) -> Vec<u8> {
    let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#);
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    frame
}

/// Build a Content-Length framed notification (no id).
fn framed_notification(method: &str) -> Vec<u8> {
    let body = format!(r#"{{"jsonrpc":"2.0","method":"{method}","params":{{}}}}"#);
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    frame
}

/// Parse the framed output, returning (claimed_content_length, body_string).
fn parse_frame(raw: &[u8]) -> io::Result<(usize, String)> {
    let output = String::from_utf8(raw.to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let header_end = output
        .find("\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no header separator"))?;
    let header = &output[..header_end];
    let claimed_len: usize = header
        .strip_prefix("Content-Length: ")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no Content-Length prefix"))?
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let body = output[header_end + 4..].to_string();
    Ok((claimed_len, body))
}

// ═══════════════════════════════════════════════════════════════════
// read_message – one-shot reader
// ═══════════════════════════════════════════════════════════════════

#[test]
fn read_message_single_request() -> io::Result<()> {
    let payload = framed_request(1, "initialize");
    let mut reader = BufReader::new(Cursor::new(payload));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "initialize");
    assert_eq!(req.id, Some(serde_json::json!(1)));
    Ok(())
}

#[test]
fn read_message_back_to_back() -> io::Result<()> {
    let mut payload = framed_request(1, "initialize");
    payload.extend(framed_request(2, "shutdown"));
    let mut reader = BufReader::new(Cursor::new(payload));

    let first = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected first"))?;
    let second = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected second"))?;

    assert_eq!(first.method, "initialize");
    assert_eq!(second.method, "shutdown");
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_returns_none_on_empty_input() -> io::Result<()> {
    let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_returns_none_for_missing_content_length() -> io::Result<()> {
    let payload = b"X-Custom: foo\r\n\r\n";
    let mut reader = BufReader::new(Cursor::new(payload.to_vec()));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_returns_none_for_invalid_content_length() -> io::Result<()> {
    let payload = b"Content-Length: notanumber\r\n\r\n";
    let mut reader = BufReader::new(Cursor::new(payload.to_vec()));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_returns_none_for_invalid_json() -> io::Result<()> {
    let body = b"this is not json";
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body);
    let mut reader = BufReader::new(Cursor::new(frame));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_returns_none_for_truncated_body() -> io::Result<()> {
    let mut frame = b"Content-Length: 1000\r\n\r\n".to_vec();
    frame.extend_from_slice(b"short");
    let mut reader = BufReader::new(Cursor::new(frame));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_case_insensitive_header() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let mut frame = format!("content-length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

#[test]
fn read_message_lf_only_separator() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":7,"method":"m","params":{}}"#;
    let mut frame = format!("Content-Length: {}\n\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "m");
    Ok(())
}

#[test]
fn read_message_preserves_params() -> io::Result<()> {
    let payload = framed_request_with_params(1, "x", r#"{"key":"val","num":42}"#);
    let mut reader = BufReader::new(Cursor::new(payload));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert_eq!(params["key"], "val");
    assert_eq!(params["num"], 42);
    Ok(())
}

#[test]
fn read_message_notification_without_id() -> io::Result<()> {
    let payload = framed_notification("initialized");
    let mut reader = BufReader::new(Cursor::new(payload));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "initialized");
    assert!(req.id.is_none());
    Ok(())
}

#[test]
fn read_message_string_id() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":"abc-123","method":"test","params":{}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.id, Some(serde_json::json!("abc-123")));
    Ok(())
}

#[test]
fn read_message_ignores_extra_headers() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let mut frame = format!(
        "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

#[test]
fn read_message_with_unicode_body() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"text":"héllo wörld 🦀"}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert_eq!(params["text"], "héllo wörld 🦀");
    Ok(())
}

#[test]
fn read_message_large_body() -> io::Result<()> {
    let big_value = "x".repeat(100_000);
    let body =
        format!(r#"{{"jsonrpc":"2.0","id":1,"method":"big","params":{{"data":"{}"}}}}"#, big_value);
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "big");
    Ok(())
}

#[test]
fn read_message_five_sequential_frames() -> io::Result<()> {
    let methods = ["a", "b", "c", "d", "e"];
    let mut payload = Vec::new();
    for (i, m) in methods.iter().enumerate() {
        payload.extend(framed_request((i + 1) as u64, m));
    }
    let mut reader = BufReader::new(Cursor::new(payload));

    for m in &methods {
        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, *m);
    }
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_negative_numeric_id() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":-1,"method":"test","params":{}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.id, Some(serde_json::json!(-1)));
    Ok(())
}

#[test]
fn read_message_null_params() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":null}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// ContentLengthMessageReader – stateful streaming reader
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stateful_reader_single_frame() -> io::Result<()> {
    let payload = framed_request(99, "shutdown");
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "shutdown");
    assert_eq!(req.id, Some(serde_json::json!(99)));
    Ok(())
}

#[test]
fn stateful_reader_back_to_back() -> io::Result<()> {
    let mut payload = framed_request(1, "textDocument/didOpen");
    payload.extend(framed_request(2, "textDocument/definition"));
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let first = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected first request"))?;
    assert_eq!(first.method, "textDocument/didOpen");

    let second = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected second request"))?;
    assert_eq!(second.method, "textDocument/definition");

    assert!(reader.read_next(&mut cursor)?.is_none());
    Ok(())
}

#[test]
fn stateful_reader_returns_none_on_empty_input() -> io::Result<()> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    let mut reader = ContentLengthMessageReader::new();
    assert!(reader.read_next(&mut cursor)?.is_none());
    Ok(())
}

#[test]
fn stateful_reader_default_trait() -> io::Result<()> {
    let payload = framed_request(1, "test");
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::default();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

#[test]
fn stateful_reader_skips_malformed_json_continues() -> io::Result<()> {
    let bad_body = b"not json at all!";
    let mut payload = format!("Content-Length: {}\r\n\r\n", bad_body.len()).into_bytes();
    payload.extend_from_slice(bad_body);
    payload.extend(framed_request(2, "valid"));

    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader.read_next(&mut cursor)?.ok_or_else(|| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "expected request after skip")
    })?;
    assert_eq!(req.method, "valid");
    Ok(())
}

#[test]
fn stateful_reader_three_frames() -> io::Result<()> {
    let mut payload = framed_request(1, "a");
    payload.extend(framed_request(2, "b"));
    payload.extend(framed_request(3, "c"));
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let methods: Vec<String> = (0..3)
        .filter_map(|_| reader.read_next(&mut cursor).ok().flatten().map(|r| r.method))
        .collect();
    assert_eq!(methods, vec!["a", "b", "c"]);
    Ok(())
}

#[test]
fn stateful_reader_many_frames() -> io::Result<()> {
    let count = 50;
    let mut payload = Vec::new();
    for i in 0..count {
        payload.extend(framed_request(i, &format!("method_{i}")));
    }
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    for i in 0..count {
        let req = reader.read_next(&mut cursor)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, format!("expected request {i}"))
        })?;
        assert_eq!(req.method, format!("method_{i}"));
    }
    assert!(reader.read_next(&mut cursor)?.is_none());
    Ok(())
}

#[test]
fn stateful_reader_notification_without_id() -> io::Result<()> {
    let payload = framed_notification("initialized");
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected notification"))?;
    assert_eq!(req.method, "initialized");
    assert!(req.id.is_none());
    Ok(())
}

#[test]
fn stateful_reader_with_params() -> io::Result<()> {
    let payload = framed_request_with_params(
        1,
        "hover",
        r#"{"uri":"file:///test.pl","position":{"line":0,"character":5}}"#,
    );
    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert_eq!(params["uri"], "file:///test.pl");
    assert_eq!(params["position"]["line"], 0);
    assert_eq!(params["position"]["character"], 5);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// write_message
// ═══════════════════════════════════════════════════════════════════

#[test]
fn write_message_null_response() -> io::Result<()> {
    let response = JsonRpcResponse::null(Some(serde_json::json!(1)));
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    assert!(parsed["result"].is_null());
    Ok(())
}

#[test]
fn write_message_success_response() -> io::Result<()> {
    let response = JsonRpcResponse::success(
        Some(serde_json::json!(5)),
        serde_json::json!({"capabilities": {}}),
    );
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["id"], 5);
    assert!(parsed.get("result").is_some());
    assert!(parsed.get("error").is_none());
    Ok(())
}

#[test]
fn write_message_error_response() -> io::Result<()> {
    let err = JsonRpcError::new(-32600, "Invalid Request");
    let response = JsonRpcResponse::error(Some(serde_json::json!(3)), err);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["error"]["code"], -32600);
    assert_eq!(parsed["error"]["message"], "Invalid Request");
    assert!(parsed.get("result").is_none());
    Ok(())
}

#[test]
fn write_message_error_with_data() -> io::Result<()> {
    let err = JsonRpcError::with_data(
        -32602,
        "Invalid params",
        serde_json::json!({"detail": "missing field"}),
    );
    let response = JsonRpcResponse::error(Some(serde_json::json!(8)), err);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["error"]["code"], -32602);
    assert_eq!(parsed["error"]["data"]["detail"], "missing field");
    Ok(())
}

#[test]
fn write_message_null_id_response() -> io::Result<()> {
    let response = JsonRpcResponse::null(None);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(parsed["id"].is_null());
    Ok(())
}

#[test]
fn write_message_string_id() -> io::Result<()> {
    let response =
        JsonRpcResponse::success(Some(serde_json::json!("req-42")), serde_json::json!(null));
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["id"], "req-42");
    Ok(())
}

#[test]
fn write_message_large_result() -> io::Result<()> {
    let big_value = serde_json::json!({"data": "x".repeat(50_000)});
    let response = JsonRpcResponse::success(Some(serde_json::json!(1)), big_value);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());
    assert!(body.len() > 50_000);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// write_notification
// ═══════════════════════════════════════════════════════════════════

#[test]
fn write_notification_basic() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(
        &mut buf,
        "window/logMessage",
        serde_json::json!({"type": 3, "message": "hi"}),
    )?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["method"], "window/logMessage");
    assert_eq!(parsed["params"]["message"], "hi");
    assert!(parsed.get("id").is_none());
    Ok(())
}

#[test]
fn write_notification_null_params() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(&mut buf, "initialized", serde_json::json!(null))?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["method"], "initialized");
    assert!(parsed["params"].is_null());
    Ok(())
}

#[test]
fn write_notification_special_method_name() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(&mut buf, "$/cancelRequest", serde_json::json!({"id": 1}))?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["method"], "$/cancelRequest");
    Ok(())
}

#[test]
fn write_notification_empty_params() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(&mut buf, "test/notify", serde_json::json!({}))?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["method"], "test/notify");
    Ok(())
}

#[test]
fn write_notification_nested_params() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(
        &mut buf,
        "textDocument/publishDiagnostics",
        serde_json::json!({
            "uri": "file:///test.pl",
            "diagnostics": [
                {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 5}},
                 "message": "error", "severity": 1}
            ]
        }),
    )?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["params"]["uri"], "file:///test.pl");
    assert_eq!(parsed["params"]["diagnostics"][0]["severity"], 1);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// log_response – smoke tests (no panics)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn log_response_null_response() {
    let response = JsonRpcResponse::null(Some(serde_json::json!(1)));
    log_response(&response);
}

#[test]
fn log_response_success_response() {
    let response =
        JsonRpcResponse::success(Some(serde_json::json!(10)), serde_json::json!({"data": true}));
    log_response(&response);
}

#[test]
fn log_response_error_response() {
    let err = JsonRpcError::new(-32601, "Method not found");
    let response = JsonRpcResponse::error(Some(serde_json::json!(7)), err);
    log_response(&response);
}

#[test]
fn log_response_none_id() {
    let response = JsonRpcResponse::null(None);
    log_response(&response);
}

#[test]
fn log_response_error_with_data() {
    let err = JsonRpcError::with_data(-32000, "custom", serde_json::json!({"ctx": "info"}));
    let response = JsonRpcResponse::error(Some(serde_json::json!(99)), err);
    log_response(&response);
}

// ═══════════════════════════════════════════════════════════════════
// round-trip tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn roundtrip_write_then_read_via_read_message() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"roundtrip","params":{"key":"value"}}"#;
    let mut request_frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    request_frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(request_frame));

    let req = read_message(&mut reader)?.ok_or_else(|| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "expected roundtrip request")
    })?;
    assert_eq!(req.method, "roundtrip");

    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert_eq!(params["key"], "value");
    Ok(())
}

#[test]
fn roundtrip_write_then_read_via_stateful_reader() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":50,"method":"textDocument/completion","params":{}}"#;
    let mut wire = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    wire.extend_from_slice(body.as_bytes());

    let mut cursor = Cursor::new(wire);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "textDocument/completion");
    assert_eq!(req.id, Some(serde_json::json!(50)));
    Ok(())
}

#[test]
fn roundtrip_write_response_verify_framing() -> io::Result<()> {
    let response = JsonRpcResponse::success(
        Some(serde_json::json!(42)),
        serde_json::json!({"hover": {"contents": "documentation"}}),
    );
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["id"], 42);
    assert_eq!(parsed["result"]["hover"]["contents"], "documentation");
    Ok(())
}

#[test]
fn roundtrip_notification_write_verify() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(
        &mut buf,
        "window/showMessage",
        serde_json::json!({"type": 3, "message": "hello"}),
    )?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["method"], "window/showMessage");
    assert_eq!(parsed["params"]["type"], 3);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// mixed read/write interleaving
// ═══════════════════════════════════════════════════════════════════

#[test]
fn interleaved_read_and_write() -> io::Result<()> {
    // Read a request
    let payload = framed_request(1, "textDocument/hover");
    let mut reader = BufReader::new(Cursor::new(payload));
    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "textDocument/hover");

    // Write a response
    let response =
        JsonRpcResponse::success(req.id.clone(), serde_json::json!({"contents": "sub docs"}));
    let mut out = Vec::new();
    write_message(&mut out, &response)?;

    let (_, body) = parse_frame(&out)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["id"], 1);
    assert_eq!(parsed["result"]["contents"], "sub docs");

    // Write a notification
    let mut notif_buf = Vec::new();
    write_notification(
        &mut notif_buf,
        "textDocument/publishDiagnostics",
        serde_json::json!({"uri": "file:///test.pl", "diagnostics": []}),
    )?;

    let (_, notif_body) = parse_frame(&notif_buf)?;
    let notif_parsed: serde_json::Value = serde_json::from_str(&notif_body)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(notif_parsed["method"], "textDocument/publishDiagnostics");
    Ok(())
}

#[test]
fn multiple_write_messages_to_same_buffer() -> io::Result<()> {
    let mut buf = Vec::new();

    let r1 = JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!("first"));
    write_message(&mut buf, &r1)?;
    let first_len = buf.len();

    let r2 = JsonRpcResponse::success(Some(serde_json::json!(2)), serde_json::json!("second"));
    write_message(&mut buf, &r2)?;

    // Both frames are in the buffer
    assert!(buf.len() > first_len);

    let output =
        String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let frames: Vec<&str> = output.split("Content-Length: ").filter(|s| !s.is_empty()).collect();
    assert_eq!(frames.len(), 2);
    Ok(())
}
