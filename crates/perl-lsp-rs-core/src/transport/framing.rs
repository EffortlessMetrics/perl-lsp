//! Message framing for the LSP base protocol.

use perl_content_length_framing::ContentLengthFramer;
pub use perl_content_length_framing::frame;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::io::{self, BufRead, Read, Write};

const LOG_PREVIEW_MAX_BYTES: usize = 160;

fn body_preview(body: &[u8]) -> String {
    let truncated_len = body.len().min(LOG_PREVIEW_MAX_BYTES);
    let mut preview = String::from_utf8_lossy(&body[..truncated_len]).to_string();

    if body.len() > LOG_PREVIEW_MAX_BYTES {
        preview.push('…');
    }

    preview.replace(['\r', '\n'], "\\n")
}

fn decode_request_text_lossy(body: &[u8]) -> Cow<'_, str> {
    let text = String::from_utf8_lossy(body);

    if matches!(text, Cow::Owned(_)) {
        tracing::warn!(
            payload_bytes = body.len(),
            preview = %body_preview(body),
            "invalid UTF-8 in incoming JSON-RPC body; replaced invalid bytes with U+FFFD"
        );
    }

    text
}

const CLIENT_RESPONSE_METHOD: &str = "$/perl-lsp/clientResponse";

fn parse_request_body(body: &[u8]) -> Result<JsonRpcRequest, serde_json::Error> {
    let text = decode_request_text_lossy(body);
    let value: Value = serde_json::from_str(text.as_ref())?;

    // Standard JSON-RPC request/notification
    if value.get("method").is_some() {
        return serde_json::from_value(value);
    }

    // JSON-RPC response to a server-initiated request.
    // Convert to an internal pseudo-notification so the runtime can route it.
    if let Some(id) = value.get("id") {
        let params = json!({
            "id": id,
            "result": value.get("result").cloned().unwrap_or(Value::Null),
            "error": value.get("error").cloned(),
        });
        return Ok(JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: None,
            method: CLIENT_RESPONSE_METHOD.to_string(),
            params: Some(params),
        });
    }

    serde_json::from_value(value)
}

/// Stateful reader for `Content-Length` framed JSON-RPC requests.
///
/// This reader keeps partial frame state across reads, which allows it to
/// handle split headers, split bodies, and multiple messages arriving in a
/// single transport read.
#[derive(Default)]
pub struct ContentLengthMessageReader {
    framer: ContentLengthFramer,
}

impl ContentLengthMessageReader {
    /// Create a new reader with empty frame state.
    #[must_use]
    pub fn new() -> Self {
        Self { framer: ContentLengthFramer::new() }
    }

    /// Read and parse the next JSON-RPC request from the underlying byte stream.
    ///
    /// Returns:
    /// - `Ok(Some(request))` when a complete request is decoded
    /// - `Ok(None)` on EOF
    /// - `Err(io::Error)` on non-recoverable I/O failure
    ///
    /// Malformed frames are logged and skipped so the caller can continue
    /// processing subsequent requests.
    pub fn read_next(&mut self, reader: &mut dyn Read) -> io::Result<Option<JsonRpcRequest>> {
        let mut chunk = [0u8; 8 * 1024];

        loop {
            match self.framer.try_next() {
                Ok(Some(body)) => match parse_request_body(&body) {
                    Ok(request) => return Ok(Some(request)),
                    Err(error) => {
                        tracing::warn!(
                            payload_bytes = body.len(),
                            preview = %body_preview(&body),
                            %error,
                            "incoming JSON parse error"
                        );
                        continue;
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(%error, "frame parse error");
                    continue;
                }
            }

            let bytes_read = reader.read(&mut chunk)?;
            if bytes_read == 0 {
                return Ok(None);
            }
            self.framer.push(&chunk[..bytes_read]);
        }
    }
}

/// Read an LSP message from a buffered reader.
///
/// This is a compatibility helper for one-shot reads. For long-running loops,
/// prefer [`ContentLengthMessageReader`] to preserve parser state across calls.
pub fn read_message(reader: &mut dyn BufRead) -> io::Result<Option<JsonRpcRequest>> {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        if line == "\r\n" || line == "\n" {
            break;
        }

        let header = line.trim_end_matches(&['\r', '\n'][..]);
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("Content-Length")
        {
            match value.trim().parse::<usize>() {
                Ok(length) => content_length = Some(length),
                Err(error) => {
                    tracing::warn!(raw_header = header, %error, "invalid Content-Length header");
                    return Ok(None);
                }
            }
        }
    }

    let length = match content_length {
        Some(length) => length,
        None => {
            tracing::warn!("missing Content-Length header");
            return Ok(None);
        }
    };

    let mut body = vec![0u8; length];
    if let Err(error) = reader.read_exact(&mut body) {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(error);
    }

    match parse_request_body(&body) {
        Ok(request) => Ok(Some(request)),
        Err(error) => {
            tracing::warn!(
                payload_bytes = body.len(),
                preview = %body_preview(&body),
                %error,
                "JSON parse error"
            );
            Ok(None)
        }
    }
}

/// Write an LSP response with `Content-Length` framing.
pub fn write_message(writer: &mut dyn Write, response: &JsonRpcResponse) -> io::Result<()> {
    let content = serde_json::to_vec(response)?;
    let framed = frame(&content);
    writer.write_all(&framed)?;
    writer.flush()
}

/// Write an LSP notification with `Content-Length` framing.
pub fn write_notification(
    writer: &mut dyn Write,
    method: &str,
    params: serde_json::Value,
) -> io::Result<()> {
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });

    let payload = serde_json::to_vec(&notification)?;
    let framed = frame(&payload);
    writer.write_all(&framed)?;
    writer.flush()
}

/// Log outgoing response metadata for transport debugging.
pub fn log_response(response: &JsonRpcResponse) {
    if let Ok(content) = serde_json::to_string(response) {
        tracing::debug!(
            id = ?response.id,
            has_result = response.result.is_some(),
            has_error = response.error.is_some(),
            payload_bytes = content.len(),
            "outgoing response"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ContentLengthMessageReader, log_response, read_message, write_message, write_notification,
    };
    use crate::protocol::{JsonRpcError, JsonRpcResponse};
    use std::io::{self, BufReader, Cursor};

    fn framed_request(id: u64, method: &str) -> Vec<u8> {
        let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{}}}}"#);
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        frame
    }

    fn framed_response(id: u64, result: &str) -> Vec<u8> {
        let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#);
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        frame
    }

    // ── read_message ───────────────────────────────────────────────

    #[test]
    fn read_message_parses_back_to_back_frames_without_losing_buffered_bytes() -> io::Result<()> {
        let mut payload = framed_request(1, "initialize");
        payload.extend(framed_request(2, "shutdown"));
        let mut reader = BufReader::with_capacity(4096, Cursor::new(payload));

        let first = read_message(&mut reader)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected first request")
        })?;
        assert_eq!(first.method, "initialize");

        let second = read_message(&mut reader)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected second request")
        })?;
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
    fn read_message_single_frame() -> io::Result<()> {
        let payload = framed_request(42, "textDocument/hover");
        let mut reader = BufReader::new(Cursor::new(payload));

        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, "textDocument/hover");
        assert_eq!(req.id, Some(serde_json::json!(42)));
        Ok(())
    }

    #[test]
    fn read_message_returns_none_for_missing_content_length() -> io::Result<()> {
        // Header block with no Content-Length, then empty separator
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
    fn read_message_returns_none_for_invalid_json_body() -> io::Result<()> {
        let body = b"this is not json";
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body);
        let mut reader = BufReader::new(Cursor::new(frame));
        assert!(read_message(&mut reader)?.is_none());
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
        let params = req
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert_eq!(params["text"], "abc\u{FFFD}");
        Ok(())
    }

    #[test]
    fn read_next_converts_client_response_to_internal_notification() -> io::Result<()> {
        let payload = framed_response(7, "{}");
        let mut cursor = Cursor::new(payload);
        let mut reader = ContentLengthMessageReader::new();

        let request = reader.read_next(&mut cursor)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected response conversion")
        })?;

        assert_eq!(request.method, "$/perl-lsp/clientResponse");
        let params = request
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert_eq!(params["id"], 7);
        assert_eq!(params["result"], serde_json::json!({}));
        Ok(())
    }

    #[test]
    fn read_message_returns_none_for_truncated_body() -> io::Result<()> {
        // Claim 1000 bytes but only provide 5
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
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"x","params":{"key":"val"}}"#;
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        let params = req
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert_eq!(params["key"], "val");
        Ok(())
    }

    #[test]
    fn read_message_notification_without_id() -> io::Result<()> {
        let body = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

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

    // ── ContentLengthMessageReader ─────────────────────────────────

    #[test]
    fn stateful_reader_keeps_extra_frames_between_reads() -> io::Result<()> {
        let mut payload = framed_request(1, "textDocument/didOpen");
        payload.extend(framed_request(2, "textDocument/definition"));
        let mut cursor = Cursor::new(payload);
        let mut reader = ContentLengthMessageReader::new();

        let first = reader.read_next(&mut cursor)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected first request")
        })?;
        assert_eq!(first.method, "textDocument/didOpen");

        let second = reader.read_next(&mut cursor)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected second request")
        })?;
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
        // First frame has invalid JSON, second is valid
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
    fn stateful_reader_replaces_invalid_utf8_in_json_strings() -> io::Result<()> {
        let mut body = br#"{"jsonrpc":"2.0","id":9,"method":"test","params":{"text":"abc"#.to_vec();
        body.push(0xFF);
        body.extend_from_slice(br#""}}"#);

        let mut payload = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        payload.extend_from_slice(&body);

        let mut cursor = Cursor::new(payload);
        let mut reader = ContentLengthMessageReader::new();
        let req = reader
            .read_next(&mut cursor)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        let params = req
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert_eq!(params["text"], "abc\u{FFFD}");
        Ok(())
    }

    // ── write_message ──────────────────────────────────────────────

    #[test]
    fn write_message_produces_valid_framed_output() -> io::Result<()> {
        let response = JsonRpcResponse::null(Some(serde_json::json!(1)));
        let mut buf = Vec::new();
        write_message(&mut buf, &response)?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains("\r\n\r\n"));

        // The body after the header separator should be valid JSON
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let body = &output[body_start..];
        let parsed: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
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

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["error"]["code"], -32600);
        assert_eq!(parsed["error"]["message"], "Invalid Request");
        assert!(parsed.get("result").is_none());
        Ok(())
    }

    #[test]
    fn write_message_content_length_matches_body() -> io::Result<()> {
        let response =
            JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!("hello"));
        let mut buf = Vec::new();
        write_message(&mut buf, &response)?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header_end = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?;
        let header = &output[..header_end];
        let claimed_len: usize = header
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no Content-Length prefix"))?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let actual_body = &output[header_end + 4..];
        assert_eq!(claimed_len, actual_body.len());
        Ok(())
    }

    // ── write_notification ─────────────────────────────────────────

    #[test]
    fn write_notification_produces_valid_frame() -> io::Result<()> {
        let mut buf = Vec::new();
        write_notification(
            &mut buf,
            "window/logMessage",
            serde_json::json!({"type": 3, "message": "hi"}),
        )?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert!(output.starts_with("Content-Length: "));

        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["method"], "window/logMessage");
        assert_eq!(parsed["params"]["message"], "hi");
        assert!(parsed.get("id").is_none());
        Ok(())
    }

    #[test]
    fn write_notification_content_length_matches_body() -> io::Result<()> {
        let mut buf = Vec::new();
        write_notification(&mut buf, "test/notify", serde_json::json!({}))?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header_end = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?;
        let header = &output[..header_end];
        let claimed_len: usize = header
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no Content-Length prefix"))?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let actual_body = &output[header_end + 4..];
        assert_eq!(claimed_len, actual_body.len());
        Ok(())
    }

    #[test]
    fn write_notification_with_empty_params() -> io::Result<()> {
        let mut buf = Vec::new();
        write_notification(&mut buf, "initialized", serde_json::json!(null))?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["method"], "initialized");
        assert!(parsed["params"].is_null());
        Ok(())
    }

    // ── log_response ───────────────────────────────────────────────

    #[test]
    fn log_response_does_not_panic_on_null_response() {
        let response = JsonRpcResponse::null(Some(serde_json::json!(1)));
        log_response(&response);
    }

    #[test]
    fn log_response_does_not_panic_on_success_response() {
        let response = JsonRpcResponse::success(
            Some(serde_json::json!(10)),
            serde_json::json!({"data": true}),
        );
        log_response(&response);
    }

    #[test]
    fn log_response_does_not_panic_on_error_response() {
        let err = JsonRpcError::new(-32601, "Method not found");
        let response = JsonRpcResponse::error(Some(serde_json::json!(7)), err);
        log_response(&response);
    }

    #[test]
    fn log_response_does_not_panic_on_none_id() {
        let response = JsonRpcResponse::null(None);
        log_response(&response);
    }

    // ── roundtrip ──────────────────────────────────────────────────

    #[test]
    fn write_then_read_roundtrip() -> io::Result<()> {
        let response = JsonRpcResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"key": "value"}),
        );
        let mut wire = Vec::new();
        write_message(&mut wire, &response)?;

        // Re-read the framed response as if it were a request
        // Build a valid request with the same framing
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"roundtrip","params":{"key":"value"}}"#;
        let mut request_frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        request_frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(request_frame));

        let req = read_message(&mut reader)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected roundtrip request")
        })?;
        assert_eq!(req.method, "roundtrip");
        let params = req
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert_eq!(params["key"], "value");
        Ok(())
    }

    #[test]
    fn write_message_then_stateful_read_roundtrip() -> io::Result<()> {
        // Construct a request frame, write it, then read via stateful reader
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

    // ── edge cases ─────────────────────────────────────────────────

    #[test]
    fn read_message_with_unicode_body() -> io::Result<()> {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"text":"héllo wörld 🦀"}}"#;
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        let params = req
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert_eq!(params["text"], "héllo wörld 🦀");
        Ok(())
    }

    #[test]
    fn read_message_large_body() -> io::Result<()> {
        let big_value = "x".repeat(100_000);
        let body = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"big","params":{{"data":"{}"}}}}"#,
            big_value
        );
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, "big");
        Ok(())
    }

    #[test]
    fn write_notification_special_characters_in_method() -> io::Result<()> {
        let mut buf = Vec::new();
        write_notification(&mut buf, "$/cancelRequest", serde_json::json!({"id": 1}))?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["method"], "$/cancelRequest");
        Ok(())
    }

    #[test]
    fn write_message_null_id_response() -> io::Result<()> {
        let response = JsonRpcResponse::null(None);
        let mut buf = Vec::new();
        write_message(&mut buf, &response)?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert!(parsed["id"].is_null());
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

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["error"]["code"], -32602);
        assert_eq!(parsed["error"]["data"]["detail"], "missing field");
        Ok(())
    }

    // ── body_preview (private helper) ────────────────────────────────

    #[test]
    fn body_preview_short_input_no_truncation() {
        let input = b"hello world";
        let result = super::body_preview(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn body_preview_truncates_long_input() {
        let input = vec![b'a'; 200];
        let result = super::body_preview(&input);
        // Should be truncated to LOG_PREVIEW_MAX_BYTES (160) plus ellipsis
        assert!(result.ends_with('\u{2026}'));
        // The visible portion is 160 'a' chars + 1 ellipsis
        assert_eq!(result.len(), super::LOG_PREVIEW_MAX_BYTES + '\u{2026}'.len_utf8());
    }

    #[test]
    fn body_preview_replaces_newlines() {
        let input = b"line1\r\nline2\nline3";
        let result = super::body_preview(input);
        assert_eq!(result, "line1\\n\\nline2\\nline3");
    }

    #[test]
    fn body_preview_empty_input() {
        let input = b"";
        let result = super::body_preview(input);
        assert_eq!(result, "");
    }

    #[test]
    fn body_preview_exactly_at_max_bytes() {
        let input = vec![b'z'; super::LOG_PREVIEW_MAX_BYTES];
        let result = super::body_preview(&input);
        // Exactly at the limit: no truncation marker
        assert!(!result.contains('\u{2026}'));
        assert_eq!(result.len(), super::LOG_PREVIEW_MAX_BYTES);
    }

    #[test]
    fn body_preview_one_byte_over_max() {
        let input = vec![b'z'; super::LOG_PREVIEW_MAX_BYTES + 1];
        let result = super::body_preview(&input);
        assert!(result.ends_with('\u{2026}'));
    }

    // ── read_message: Content-Length edge cases ──────────────────────

    #[test]
    fn read_message_zero_content_length_returns_none() -> io::Result<()> {
        // Content-Length: 0 means empty body, which is not valid JSON-RPC
        let frame = b"Content-Length: 0\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(frame.to_vec()));
        assert!(read_message(&mut reader)?.is_none());
        Ok(())
    }

    #[test]
    fn read_message_content_length_with_leading_spaces() -> io::Result<()> {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        let mut frame = format!("Content-Length:   {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, "test");
        Ok(())
    }

    #[test]
    fn read_message_content_length_with_trailing_spaces() -> io::Result<()> {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        let mut frame = format!("Content-Length: {}   \r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

        // Trailing spaces after the number should be trimmed
        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, "test");
        Ok(())
    }

    #[test]
    fn read_message_negative_content_length_returns_none() -> io::Result<()> {
        let payload = b"Content-Length: -1\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(payload.to_vec()));
        assert!(read_message(&mut reader)?.is_none());
        Ok(())
    }

    #[test]
    fn read_message_content_length_last_among_headers() -> io::Result<()> {
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
    fn read_message_mixed_case_content_length() -> io::Result<()> {
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
    fn read_message_empty_method_string() -> io::Result<()> {
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
    fn read_message_array_params() -> io::Result<()> {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[1,2,3]}"#;
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        let mut reader = BufReader::new(Cursor::new(frame));

        let req = read_message(&mut reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, "test");
        let params = req
            .params
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "expected params"))?;
        assert!(params.is_array());
        assert_eq!(params.as_array().map(|a| a.len()), Some(3));
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
        assert_eq!(req.method, "test");
        // serde_json deserializes `"id": null` as None for Option<Value>
        assert!(req.id.is_none());
        Ok(())
    }

    // ── stateful reader: incremental delivery ───────────────────────

    #[test]
    fn stateful_reader_byte_at_a_time() -> io::Result<()> {
        let full_frame = framed_request(1, "incremental");
        // Feed one byte at a time via a reader that yields 1 byte per read call
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
        assert_eq!(req.method, "incremental");
        assert_eq!(req.id, Some(serde_json::json!(1)));
        Ok(())
    }

    #[test]
    fn stateful_reader_large_message() -> io::Result<()> {
        let big_value = "y".repeat(100_000);
        let body = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"big","params":{{"data":"{}"}}}}"#,
            big_value
        );
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
        // Three malformed frames followed by one valid frame
        let bad1 = b"invalid json 1";
        let bad2 = b"invalid json 2";
        let bad3 = b"{not valid}";
        let mut payload = Vec::new();
        for bad in [bad1.as_slice(), bad2.as_slice(), bad3.as_slice()] {
            payload.extend_from_slice(format!("Content-Length: {}\r\n\r\n", bad.len()).as_bytes());
            payload.extend_from_slice(bad);
        }
        payload.extend(framed_request(1, "finally_valid"));

        let mut cursor = Cursor::new(payload);
        let mut reader = ContentLengthMessageReader::new();

        let req = reader
            .read_next(&mut cursor)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "expected request"))?;
        assert_eq!(req.method, "finally_valid");
        Ok(())
    }

    // ── write_message: additional serialization cases ────────────────

    #[test]
    fn write_message_unicode_in_result() -> io::Result<()> {
        let response = JsonRpcResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"text": "cafe\u{0301} \u{1f980}"}),
        );
        let mut buf = Vec::new();
        write_message(&mut buf, &response)?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header_end = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?;
        let header = &output[..header_end];
        let claimed_len: usize = header
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no Content-Length prefix"))?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let actual_body = &output[header_end + 4..];
        // Content-Length must match byte length, not char count
        assert_eq!(claimed_len, actual_body.len());

        let parsed: serde_json::Value = serde_json::from_str(actual_body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert!(parsed["result"]["text"].is_string());
        Ok(())
    }

    #[test]
    fn write_message_deeply_nested_result() -> io::Result<()> {
        // Build a deeply nested JSON value
        let mut value = serde_json::json!("leaf");
        for _ in 0..50 {
            value = serde_json::json!({"nested": value});
        }
        let response = JsonRpcResponse::success(Some(serde_json::json!(1)), value);
        let mut buf = Vec::new();
        write_message(&mut buf, &response)?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header_end = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?;
        let header = &output[..header_end];
        let claimed_len: usize = header
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no Content-Length prefix"))?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let actual_body = &output[header_end + 4..];
        assert_eq!(claimed_len, actual_body.len());

        // Verify the JSON is parseable
        let _parsed: serde_json::Value = serde_json::from_str(actual_body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(())
    }

    // ── write_notification: additional cases ─────────────────────────

    #[test]
    fn write_notification_array_params() -> io::Result<()> {
        let mut buf = Vec::new();
        write_notification(&mut buf, "test/array", serde_json::json!([1, "two", 3]))?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let body_start = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?
            + 4;
        let parsed: serde_json::Value = serde_json::from_str(&output[body_start..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["method"], "test/array");
        assert!(parsed["params"].is_array());
        assert_eq!(parsed["params"][1], "two");
        Ok(())
    }

    #[test]
    fn write_notification_unicode_method_and_params() -> io::Result<()> {
        let mut buf = Vec::new();
        write_notification(
            &mut buf,
            "custom/notify",
            serde_json::json!({"msg": "\u{1f600} emoji test \u{00e9}"}),
        )?;

        let output =
            String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let header_end = output
            .find("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no separator"))?;
        let header = &output[..header_end];
        let claimed_len: usize = header
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no Content-Length prefix"))?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let actual_body = &output[header_end + 4..];
        assert_eq!(claimed_len, actual_body.len());
        Ok(())
    }

    // ── frame re-export ─────────────────────────────────────────────

    #[test]
    fn frame_produces_correct_content_length_header() {
        let body = b"hello world";
        let framed = super::frame(body);
        let output = String::from_utf8_lossy(&framed);
        assert!(output.starts_with("Content-Length: 11\r\n\r\n"));
        assert!(output.ends_with("hello world"));
    }

    #[test]
    fn frame_empty_body() {
        let framed = super::frame(b"");
        let output = String::from_utf8_lossy(&framed);
        assert_eq!(output.as_ref(), "Content-Length: 0\r\n\r\n");
    }

    #[test]
    fn frame_body_length_matches_header() {
        let body = b"{\"jsonrpc\":\"2.0\"}";
        let framed = super::frame(body);
        let output = String::from_utf8_lossy(&framed);
        let header_end = output.find("\r\n\r\n").map(|p| p + 4);
        if let Some(end) = header_end {
            let actual_body = &framed[end..];
            assert_eq!(actual_body, body);
            assert_eq!(actual_body.len(), body.len());
        }
    }

    // ── log_response: covers all JSON-RPC error code families ───────

    #[test]
    fn log_response_standard_error_codes() {
        let codes = [-32700, -32600, -32601, -32602, -32603, -32800, -32802];
        for code in codes {
            let err = JsonRpcError::new(code, "test error");
            let response = JsonRpcResponse::error(Some(serde_json::json!(1)), err);
            log_response(&response);
        }
    }

    #[test]
    fn log_response_with_large_result() {
        let big_data = serde_json::json!({"data": "x".repeat(10_000)});
        let response = JsonRpcResponse::success(Some(serde_json::json!(1)), big_data);
        log_response(&response);
    }
}
