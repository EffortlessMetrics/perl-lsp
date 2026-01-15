//! Message framing for LSP Base Protocol
//!
//! Implements Content-Length based message framing as specified in
//! the LSP Base Protocol.

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// Read an LSP message from a buffered reader
///
/// Returns `Ok(None)` on EOF or parse error (recoverable).
/// Returns `Err` only on I/O errors (non-recoverable).
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<JsonRpcRequest>> {
    let mut headers = HashMap::new();

    // Read headers
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None); // EOF
        }

        let line = line.trim_end();
        if line.is_empty() {
            break; // End of headers
        }

        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_string(), value.to_string());
        }
    }

    // Read content
    if let Some(content_length) = headers.get("Content-Length") {
        if let Ok(length) = content_length.parse::<usize>() {
            let mut content = vec![0u8; length];
            let mut bytes_read = 0;

            // Read content in chunks to handle partial reads
            while bytes_read < length {
                let bytes_to_read = length - bytes_read;
                let mut chunk = vec![0u8; bytes_to_read];
                match reader.read(&mut chunk)? {
                    0 => return Ok(None), // Unexpected EOF
                    n => {
                        content[bytes_read..bytes_read + n].copy_from_slice(&chunk[..n]);
                        bytes_read += n;
                    }
                }
            }

            // Parse JSON-RPC request with enhanced error handling
            match serde_json::from_slice(&content) {
                Ok(request) => return Ok(Some(request)),
                Err(e) => {
                    // Enhanced malformed frame recovery
                    eprintln!("LSP server: JSON parse error - {}", e);

                    // Attempt to extract malformed content safely (no sensitive data logging)
                    let content_str = String::from_utf8_lossy(&content);
                    if content_str.len() > 100 {
                        eprintln!(
                            "LSP server: Malformed frame (truncated): {}...",
                            &content_str[..100]
                        );
                    } else {
                        eprintln!("LSP server: Malformed frame: {}", content_str);
                    }

                    // Continue processing - don't crash the server on malformed input
                    return Ok(None);
                }
            }
        }
    }

    Ok(None)
}

/// Write an LSP message to a writer with proper framing
pub fn write_message<W: Write>(writer: &mut W, response: &JsonRpcResponse) -> io::Result<()> {
    let content = serde_json::to_string(response)?;
    let content_length = content.len();

    write!(writer, "Content-Length: {}\r\n\r\n{}", content_length, content)?;
    writer.flush()?;

    Ok(())
}

/// Write an LSP notification to a writer
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

    let notification_str = serde_json::to_string(&notification)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str)?;
    writer.flush()
}

/// Log outgoing response for debugging
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
