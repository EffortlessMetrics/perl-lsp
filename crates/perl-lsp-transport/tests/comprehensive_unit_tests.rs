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
fn read_message_replaces_invalid_utf8_in_json_strings() -> io::Result<()> {
    let mut body = br#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"text":"abc"#.to_vec();
    body.push(0xFF);
    body.extend_from_slice(br#""}}"#);

    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(&body);
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert_eq!(params["text"], "abc\u{FFFD}");
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

// ===================================================================
// Serialization/deserialization: JSON-RPC message type coverage
// ===================================================================

#[test]
fn read_message_zero_content_length_returns_none() -> io::Result<()> {
    let frame = b"Content-Length: 0\r\n\r\n";
    let mut reader = BufReader::new(Cursor::new(frame.to_vec()));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_negative_content_length_returns_none() -> io::Result<()> {
    let payload = b"Content-Length: -5\r\n\r\n";
    let mut reader = BufReader::new(Cursor::new(payload.to_vec()));
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_missing_params_field() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    assert!(req.params.is_none());
    Ok(())
}

#[test]
fn read_message_array_params() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[1,2,3]}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert!(params.is_array());
    assert_eq!(params.as_array().map(|a| a.len()), Some(3));
    Ok(())
}

#[test]
fn read_message_float_id() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1.5,"method":"test","params":{}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.id, Some(serde_json::json!(1.5)));
    Ok(())
}

#[test]
fn read_message_null_id() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":null,"method":"test","params":{}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    // serde_json deserializes `"id": null` as None for Option<Value>
    assert!(req.id.is_none());
    Ok(())
}

#[test]
fn read_message_empty_method() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"","params":{}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "");
    Ok(())
}

#[test]
fn read_message_all_caps_content_length() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let mut frame = format!("CONTENT-LENGTH: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

#[test]
fn read_message_content_length_with_extra_whitespace() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let mut frame = format!("Content-Length:   {}  \r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

#[test]
fn read_message_content_length_last_header() -> io::Result<()> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let mut frame =
        format!("X-Custom-A: foo\r\nX-Custom-B: bar\r\nContent-Length: {}\r\n\r\n", body.len())
            .into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    Ok(())
}

#[test]
fn read_message_valid_json_but_missing_method() -> io::Result<()> {
    // Valid JSON object but missing required "method" field
    let body = r#"{"jsonrpc":"2.0","id":1,"params":{}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    // Should return None because deserialization to JsonRpcRequest fails
    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_json_array_not_object() -> io::Result<()> {
    let body = r#"[1,2,3]"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    assert!(read_message(&mut reader)?.is_none());
    Ok(())
}

#[test]
fn read_message_deeply_nested_params() -> io::Result<()> {
    let body =
        r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"a":{"b":{"c":{"d":"deep"}}}}}"#;
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(frame));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    let params =
        req.params.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
    assert_eq!(params["a"]["b"]["c"]["d"], "deep");
    Ok(())
}

// ===================================================================
// Stateful reader: additional edge cases
// ===================================================================

#[test]
fn stateful_reader_byte_at_a_time_delivery() -> io::Result<()> {
    let full_frame = framed_request(1, "byte_by_byte");
    struct OneByteReader {
        data: Vec<u8>,
        pos: usize,
    }
    impl io::Read for OneByteReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            buf[0] = self.data[self.pos];
            self.pos += 1;
            Ok(1)
        }
    }
    let mut source = OneByteReader { data: full_frame, pos: 0 };
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut source)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "byte_by_byte");
    Ok(())
}

#[test]
fn stateful_reader_large_message() -> io::Result<()> {
    let big_value = "z".repeat(100_000);
    let body =
        format!(r#"{{"jsonrpc":"2.0","id":1,"method":"big","params":{{"data":"{}"}}}}"#, big_value);
    let mut payload = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    payload.extend_from_slice(body.as_bytes());

    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "big");
    Ok(())
}

#[test]
fn stateful_reader_multiple_malformed_then_valid() -> io::Result<()> {
    let bad_bodies: &[&[u8]] = &[b"bad1", b"bad2", b"bad3"];
    let mut payload = Vec::new();
    for bad in bad_bodies {
        payload.extend_from_slice(format!("Content-Length: {}\r\n\r\n", bad.len()).as_bytes());
        payload.extend_from_slice(bad);
    }
    payload.extend(framed_request(1, "valid_after_errors"));

    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader.read_next(&mut cursor)?.ok_or_else(|| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "expected request after malformed")
    })?;
    assert_eq!(req.method, "valid_after_errors");
    Ok(())
}

#[test]
fn stateful_reader_mixed_notifications_and_requests() -> io::Result<()> {
    let mut payload = framed_notification("initialized");
    payload.extend(framed_request(1, "textDocument/hover"));
    payload.extend(framed_notification("textDocument/didOpen"));
    payload.extend(framed_request(2, "shutdown"));

    let mut cursor = Cursor::new(payload);
    let mut reader = ContentLengthMessageReader::new();

    let msg1 = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected msg 1"))?;
    assert_eq!(msg1.method, "initialized");
    assert!(msg1.id.is_none());

    let msg2 = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected msg 2"))?;
    assert_eq!(msg2.method, "textDocument/hover");
    assert_eq!(msg2.id, Some(serde_json::json!(1)));

    let msg3 = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected msg 3"))?;
    assert_eq!(msg3.method, "textDocument/didOpen");
    assert!(msg3.id.is_none());

    let msg4 = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected msg 4"))?;
    assert_eq!(msg4.method, "shutdown");
    assert_eq!(msg4.id, Some(serde_json::json!(2)));

    assert!(reader.read_next(&mut cursor)?.is_none());
    Ok(())
}

// ===================================================================
// write_message: additional serialization verification
// ===================================================================

#[test]
fn write_message_unicode_content_length_is_byte_count() -> io::Result<()> {
    // Unicode chars take multiple bytes; Content-Length must be bytes, not chars
    let response = JsonRpcResponse::success(
        Some(serde_json::json!(1)),
        serde_json::json!({"text": "\u{1f600}\u{1f600}\u{1f600}"}),
    );
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    // Content-Length must match byte length (not char count)
    assert_eq!(claimed_len, body.len());
    // Body must contain the emoji text in some form (serde_json outputs UTF-8 directly)
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert!(parsed["result"]["text"].is_string());
    Ok(())
}

#[test]
fn write_message_deeply_nested_json() -> io::Result<()> {
    let mut value = serde_json::json!("leaf");
    for _ in 0..50 {
        value = serde_json::json!({"n": value});
    }
    let response = JsonRpcResponse::success(Some(serde_json::json!(1)), value);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());
    // Verify it parses back
    let _: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(())
}

#[test]
fn write_message_empty_result_object() -> io::Result<()> {
    let response = JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!({}));
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["result"], serde_json::json!({}));
    Ok(())
}

#[test]
fn write_message_result_is_array() -> io::Result<()> {
    let response = JsonRpcResponse::success(
        Some(serde_json::json!(1)),
        serde_json::json!([{"label": "foo"}, {"label": "bar"}]),
    );
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert!(parsed["result"].is_array());
    assert_eq!(parsed["result"][0]["label"], "foo");
    Ok(())
}

#[test]
fn write_message_result_is_string() -> io::Result<()> {
    let response =
        JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!("just a string"));
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["result"], "just a string");
    Ok(())
}

#[test]
fn write_message_result_is_boolean() -> io::Result<()> {
    let response = JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!(true));
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["result"], true);
    Ok(())
}

#[test]
fn write_message_result_is_number() -> io::Result<()> {
    let response = JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!(42));
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["result"], 42);
    Ok(())
}

// ===================================================================
// write_notification: additional cases
// ===================================================================

#[test]
fn write_notification_array_params() -> io::Result<()> {
    let mut buf = Vec::new();
    write_notification(&mut buf, "test/array", serde_json::json!([1, "two", 3]))?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["method"], "test/array");
    assert!(parsed["params"].is_array());
    Ok(())
}

#[test]
fn write_notification_large_payload() -> io::Result<()> {
    let big_data = serde_json::json!({"data": "x".repeat(50_000)});
    let mut buf = Vec::new();
    write_notification(&mut buf, "test/large", big_data)?;

    let (claimed_len, body) = parse_frame(&buf)?;
    assert_eq!(claimed_len, body.len());
    assert!(body.len() > 50_000);
    Ok(())
}

// ===================================================================
// Error response JSON-RPC standard error codes
// ===================================================================

#[test]
fn write_message_parse_error_code() -> io::Result<()> {
    let err = JsonRpcError::new(-32700, "Parse error");
    let response = JsonRpcResponse::error(None, err);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["error"]["code"], -32700);
    assert_eq!(parsed["error"]["message"], "Parse error");
    assert!(parsed["id"].is_null());
    Ok(())
}

#[test]
fn write_message_method_not_found_code() -> io::Result<()> {
    let err = JsonRpcError::new(-32601, "Method not found");
    let response = JsonRpcResponse::error(Some(serde_json::json!(1)), err);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["error"]["code"], -32601);
    Ok(())
}

#[test]
fn write_message_internal_error_code() -> io::Result<()> {
    let err = JsonRpcError::new(-32603, "Internal error");
    let response = JsonRpcResponse::error(Some(serde_json::json!(1)), err);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["error"]["code"], -32603);
    Ok(())
}

#[test]
fn write_message_server_cancelled_code() -> io::Result<()> {
    let err = JsonRpcError::new(-32802, "Request cancelled");
    let response = JsonRpcResponse::error(Some(serde_json::json!(1)), err);
    let mut buf = Vec::new();
    write_message(&mut buf, &response)?;

    let (_, body) = parse_frame(&buf)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert_eq!(parsed["error"]["code"], -32802);
    Ok(())
}

// ===================================================================
// frame function (re-exported from perl-content-length-framing)
// ===================================================================

#[test]
fn frame_reexport_produces_valid_content_length() -> io::Result<()> {
    use perl_lsp_transport::frame;

    let body = br#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let framed = frame(body);

    let output = String::from_utf8(framed.clone())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    assert!(output.starts_with("Content-Length: "));
    assert!(output.contains("\r\n\r\n"));

    let header_end = output
        .find("\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?;
    let header = &output[..header_end];
    let claimed_len: usize = header
        .strip_prefix("Content-Length: ")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no prefix"))?
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let actual_body = &framed[header_end + 4..];
    assert_eq!(claimed_len, actual_body.len());
    assert_eq!(actual_body, body);
    Ok(())
}

#[test]
fn frame_reexport_empty_body() {
    use perl_lsp_transport::frame;

    let framed = frame(b"");
    let output = String::from_utf8_lossy(&framed);
    assert_eq!(output.as_ref(), "Content-Length: 0\r\n\r\n");
}

#[test]
fn frame_then_read_message_roundtrip() -> io::Result<()> {
    use perl_lsp_transport::frame;

    let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let framed = frame(body.as_bytes());
    let mut reader = BufReader::new(Cursor::new(framed));

    let req = read_message(&mut reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "test");
    assert_eq!(req.id, Some(serde_json::json!(1)));
    Ok(())
}

#[test]
fn frame_then_stateful_read_roundtrip() -> io::Result<()> {
    use perl_lsp_transport::frame;

    let body = r#"{"jsonrpc":"2.0","id":2,"method":"hover","params":{}}"#;
    let framed = frame(body.as_bytes());
    let mut cursor = Cursor::new(framed);
    let mut reader = ContentLengthMessageReader::new();

    let req = reader
        .read_next(&mut cursor)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
    assert_eq!(req.method, "hover");
    assert_eq!(req.id, Some(serde_json::json!(2)));
    Ok(())
}
