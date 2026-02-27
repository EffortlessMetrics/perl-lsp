//! Message framing for the LSP base protocol.

use perl_content_length_framing::{ContentLengthFramer, frame};
use perl_lsp_protocol::{JsonRpcRequest, JsonRpcResponse};
use std::io::{self, BufRead, Read, Write};

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
                        eprintln!("LSP server: JSON parse error - {error}");
                        continue;
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    eprintln!("LSP server: frame parse error - {error}");
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
                    eprintln!("LSP server: invalid Content-Length header - {error}");
                    return Ok(None);
                }
            }
        }
    }

    let length = match content_length {
        Some(length) => length,
        None => {
            eprintln!("LSP server: missing Content-Length header");
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
            eprintln!("LSP server: JSON parse error - {error}");
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
            "[perl-lsp:tx] id={:?} has_result={} has_error={} len={}",
            response.id,
            response.result.is_some(),
            response.error.is_some(),
            content.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentLengthMessageReader, read_message};
    use std::io::{self, BufReader, Cursor};

    fn framed_request(id: u64, method: &str) -> Vec<u8> {
        let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{}}}}"#);
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        frame
    }

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
}
