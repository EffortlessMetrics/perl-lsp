//! Shared `Content-Length` frame parsing and encoding for JSON-RPC transports.
//!
//! This crate provides a transport-level framing primitive used by both LSP and
//! DAP components:
//!
//! `Content-Length: <len>\r\n\r\n<raw JSON bytes>`
//!
//! The framer is intentionally payload-agnostic and operates only on bytes.

use std::fmt;

const HEADER_SENTINEL: &[u8] = b"Content-Length:";
const HEADER_END: &[u8] = b"\r\n\r\n";
const RESYNC_TAIL_BYTES: usize = 8 * 1024;
const MAX_DESYNC_BUFFER_BYTES: usize = 64 * 1024;

/// Maximum allowed message body size in bytes.
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Framing errors for `Content-Length` transport parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramingError {
    /// Header bytes could not be interpreted as a valid frame header.
    InvalidHeader,
    /// Header bytes were not valid UTF-8.
    InvalidHeaderUtf8,
    /// `Content-Length` header was missing from a complete header block.
    MissingContentLength,
    /// `Content-Length` value was not a valid non-negative integer.
    InvalidContentLength,
    /// `Content-Length` exceeded [`MAX_FRAME_SIZE`].
    FrameTooLarge { len: usize },
}

impl fmt::Display for FramingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => write!(f, "invalid Content-Length header"),
            Self::InvalidHeaderUtf8 => write!(f, "header contains invalid UTF-8"),
            Self::MissingContentLength => write!(f, "missing Content-Length header"),
            Self::InvalidContentLength => write!(f, "invalid Content-Length value"),
            Self::FrameTooLarge { len } => write!(f, "frame too large: {len} bytes"),
        }
    }
}

impl std::error::Error for FramingError {}

/// Stateful extractor for `Content-Length` framed payloads.
#[derive(Default, Debug, Clone)]
pub struct ContentLengthFramer {
    buf: Vec<u8>,
}

impl ContentLengthFramer {
    /// Create a new empty framer.
    #[must_use]
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Append raw transport bytes.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        self.resync_if_needed();
    }

    /// Attempt to extract one complete message body.
    ///
    /// Returns:
    /// - `Ok(Some(body))` when a complete frame is available
    /// - `Ok(None)` when more bytes are needed
    /// - `Err(...)` for malformed headers or disallowed sizes
    pub fn try_next(&mut self) -> Result<Option<Vec<u8>>, FramingError> {
        self.resync_if_needed();

        let Some(start) = find_header_start(&self.buf) else {
            if let Some(header_end) = find_subslice(&self.buf, HEADER_END) {
                match std::str::from_utf8(&self.buf[..header_end]) {
                    Ok(header) => {
                        let has_header_shape = header
                            .split("\r\n")
                            .any(|line| !line.trim().is_empty() && line.contains(':'));
                        self.consume_header_block(header_end);
                        if has_header_shape {
                            return Err(FramingError::MissingContentLength);
                        }
                        return Err(FramingError::InvalidHeader);
                    }
                    Err(_) => {
                        self.consume_header_block(header_end);
                        return Err(FramingError::InvalidHeaderUtf8);
                    }
                }
            }
            return Ok(None);
        };
        if start > 0 {
            self.buf.drain(..start);
        }

        let Some(header_end) = find_subslice(&self.buf, HEADER_END) else {
            return Ok(None);
        };

        let header_bytes = &self.buf[..header_end];
        let header_str = match std::str::from_utf8(header_bytes) {
            Ok(header) => header,
            Err(_) => {
                self.consume_header_block(header_end);
                return Err(FramingError::InvalidHeaderUtf8);
            }
        };

        let length = match parse_content_length(header_str) {
            ContentLengthParse::Found(len) => len,
            ContentLengthParse::Missing => {
                self.consume_header_block(header_end);
                return Err(FramingError::MissingContentLength);
            }
            ContentLengthParse::Invalid => {
                self.consume_header_block(header_end);
                return Err(FramingError::InvalidContentLength);
            }
            ContentLengthParse::MalformedHeader => {
                self.consume_header_block(header_end);
                return Err(FramingError::InvalidHeader);
            }
        };

        if length > MAX_FRAME_SIZE {
            self.consume_header_block(header_end);
            return Err(FramingError::FrameTooLarge { len: length });
        }

        let body_start = header_end + HEADER_END.len();
        let Some(body_end) = body_start.checked_add(length) else {
            self.consume_header_block(header_end);
            return Err(FramingError::InvalidContentLength);
        };

        if self.buf.len() < body_end {
            return Ok(None);
        }

        let body = self.buf[body_start..body_end].to_vec();
        self.buf.drain(..body_end);
        self.resync_if_needed();
        Ok(Some(body))
    }

    fn consume_header_block(&mut self, header_end: usize) {
        let drain_to = (header_end + HEADER_END.len()).min(self.buf.len());
        self.buf.drain(..drain_to);
        self.resync_if_needed();
    }

    fn resync_if_needed(&mut self) {
        match find_header_start(&self.buf) {
            Some(0) => {}
            Some(prefix_len) => {
                self.buf.drain(..prefix_len);
            }
            None => {
                if self.buf.len() > MAX_DESYNC_BUFFER_BYTES {
                    let keep = RESYNC_TAIL_BYTES.min(self.buf.len());
                    self.buf.drain(..self.buf.len() - keep);
                }
            }
        }
    }
}

/// Build a full `Content-Length` framed message from a payload body.
#[must_use]
pub fn frame(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_SENTINEL.len() + 32 + HEADER_END.len() + body.len());
    out.extend_from_slice(b"Content-Length: ");
    out.extend_from_slice(body.len().to_string().as_bytes());
    out.extend_from_slice(HEADER_END);
    out.extend_from_slice(body);
    out
}

enum ContentLengthParse {
    Found(usize),
    Missing,
    Invalid,
    MalformedHeader,
}

fn parse_content_length(header: &str) -> ContentLengthParse {
    let mut found = None;
    for line in header.split("\r\n") {
        if line.is_empty() {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return ContentLengthParse::MalformedHeader;
        };

        if name.trim().eq_ignore_ascii_case("Content-Length") {
            match value.trim().parse::<usize>() {
                Ok(length) => found = Some(length),
                Err(_) => return ContentLengthParse::Invalid,
            }
        }
    }

    found.map_or(ContentLengthParse::Missing, ContentLengthParse::Found)
}

fn find_header_start(hay: &[u8]) -> Option<usize> {
    hay.windows(HEADER_SENTINEL.len())
        .position(|window| window.eq_ignore_ascii_case(HEADER_SENTINEL))
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{ContentLengthFramer, FramingError, MAX_FRAME_SIZE, frame};

    fn take_body(result: Result<Option<Vec<u8>>, FramingError>) -> Vec<u8> {
        match result {
            Ok(Some(body)) => body,
            other => {
                assert!(
                    matches!(other, Ok(Some(_))),
                    "expected Ok(Some(_)) from framer, got {other:?}"
                );
                Vec::new()
            }
        }
    }

    fn assert_pending(result: Result<Option<Vec<u8>>, FramingError>) {
        assert!(
            matches!(result, Ok(None)),
            "expected Ok(None) from framer, got {result:?}"
        );
    }

    fn take_error(result: Result<Option<Vec<u8>>, FramingError>) -> FramingError {
        match result {
            Err(error) => error,
            other => {
                assert!(other.is_err(), "expected Err(_) from framer, got {other:?}");
                FramingError::InvalidHeader
            }
        }
    }

    #[test]
    fn extracts_single_frame() {
        let mut framer = ContentLengthFramer::new();
        let body = br#"{"jsonrpc":"2.0","id":1}"#;

        framer.push(&frame(body));
        let got = take_body(framer.try_next());
        assert_eq!(got, body);
        assert_pending(framer.try_next());
    }

    #[test]
    fn handles_split_header_and_body() {
        let mut framer = ContentLengthFramer::new();
        let body = br#"{"x":1}"#;
        let msg = frame(body);

        framer.push(&msg[..5]);
        assert_pending(framer.try_next());

        framer.push(&msg[5..msg.len() - 2]);
        assert_pending(framer.try_next());

        framer.push(&msg[msg.len() - 2..]);
        let got = take_body(framer.try_next());
        assert_eq!(got, body);
    }

    #[test]
    fn extracts_multiple_frames_back_to_back() {
        let mut framer = ContentLengthFramer::new();
        let a = br#"{"a":1}"#;
        let b = br#"{"b":2}"#;
        let mut combined = frame(a);
        combined.extend_from_slice(&frame(b));

        framer.push(&combined);
        assert_eq!(take_body(framer.try_next()), a);
        assert_eq!(take_body(framer.try_next()), b);
        assert_pending(framer.try_next());
    }

    #[test]
    fn drains_garbage_prefix_before_header() {
        let mut framer = ContentLengthFramer::new();
        let body = br#"{"ok":true}"#;
        let mut msg = b"junkjunk".to_vec();
        msg.extend_from_slice(&frame(body));

        framer.push(&msg);
        assert_eq!(take_body(framer.try_next()), body);
    }

    #[test]
    fn rejects_non_numeric_content_length() {
        let mut framer = ContentLengthFramer::new();
        framer.push(b"Content-Length: nope\r\n\r\n{}");

        let err = take_error(framer.try_next());
        assert_eq!(err, FramingError::InvalidContentLength);
    }

    #[test]
    fn rejects_missing_content_length() {
        let mut framer = ContentLengthFramer::new();
        framer.push(b"Content-Type: application/json\r\n\r\n{}");

        let err = take_error(framer.try_next());
        assert_eq!(err, FramingError::MissingContentLength);
    }

    #[test]
    fn rejects_invalid_utf8_in_header() {
        let mut framer = ContentLengthFramer::new();
        framer.push(b"Content-Length: 2\r\nX-Test: \xFF\r\n\r\n{}");

        let err = take_error(framer.try_next());
        assert_eq!(err, FramingError::InvalidHeaderUtf8);
    }

    #[test]
    fn rejects_oversized_frame() {
        let mut framer = ContentLengthFramer::new();
        let too_large = MAX_FRAME_SIZE + 1;
        let header = format!("Content-Length: {too_large}\r\n\r\n");
        framer.push(header.as_bytes());

        let err = take_error(framer.try_next());
        assert_eq!(err, FramingError::FrameTooLarge { len: too_large });
    }

    #[test]
    fn supports_case_insensitive_header_name() {
        let mut framer = ContentLengthFramer::new();
        let body = br#"{"ok":1}"#;
        let msg = format!(
            "content-length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap_or("")
        );
        framer.push(msg.as_bytes());

        let got = take_body(framer.try_next());
        assert_eq!(got, body);
    }
}
