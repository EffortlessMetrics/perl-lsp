//! Message framing for the LSP base protocol.

use perl_content_length_framing::ContentLengthFramer;
pub use perl_content_length_framing::frame;
use perl_lsp_protocol::{JsonRpcRequest, JsonRpcResponse};
use std::io::{self, BufRead, Read, Write};

const LOG_PREFIX: &str = "[perl-lsp:transport]";
const LOG_PREVIEW_MAX_BYTES: usize = 160;

fn body_preview(body: &[u8]) -> String {
    let truncated_len = body.len().min(LOG_PREVIEW_MAX_BYTES);
    let mut preview = String::from_utf8_lossy(&body[..truncated_len]).to_string();

    if body.len() > LOG_PREVIEW_MAX_BYTES {
        preview.push('…');
    }

    preview.replace(['\r', '\n'], "\\n")
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
                Ok(Some(body)) => match serde_json::from_slice::<JsonRpcRequest>(&body) {
                    Ok(request) => return Ok(Some(request)),
                    Err(error) => {
                        eprintln!(
                            "{LOG_PREFIX} incoming JSON parse error: {error}; payload_bytes={}; preview=\"{}\"",
                            body.len(),
                            body_preview(&body)
                        );
                        continue;
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    eprintln!("{LOG_PREFIX} frame parse error: {error}");
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
                    eprintln!(
                        "{LOG_PREFIX} invalid Content-Length header: {error}; raw_header=\"{}\"",
                        header
                    );
                    return Ok(None);
                }
            }
        }
    }

    let length = match content_length {
        Some(length) => length,
        None => {
            eprintln!("{LOG_PREFIX} missing Content-Length header");
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

    match serde_json::from_slice::<JsonRpcRequest>(&body) {
        Ok(request) => Ok(Some(request)),
        Err(error) => {
            eprintln!(
                "{LOG_PREFIX} JSON parse error: {error}; payload_bytes={}; preview=\"{}\"",
                body.len(),
                body_preview(&body)
            );
            Ok(None)
        }
    }
}

/// Write an LSP response with `Content-Length` framing.
pub fn write_message<W: Write>(writer: &mut W, response: &JsonRpcResponse) -> io::Result<()> {
    let content = serde_json::to_vec(response)?;
    let framed = frame(&content);
    writer.write_all(&framed)?;
    writer.flush()
}

/// Write an LSP notification with `Content-Length` framing.
pub fn write_notification<W: Write>(
    writer: &mut W,
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
        eprintln!(
            "{LOG_PREFIX} outgoing response id={:?} has_result={} has_error={} payload_bytes={}",
            response.id,
            response.result.is_some(),
            response.error.is_some(),
            content.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ContentLengthMessageReader, log_response, read_message, write_message, write_notification,
    };
    use perl_lsp_protocol::{JsonRpcError, JsonRpcResponse};
    use std::io::{self, BufReader, Cursor};

    fn framed_request(id: u64, method: &str) -> Vec<u8> {
        let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{}}}}"#);
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
}
