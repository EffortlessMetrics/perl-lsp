//! Content-Length header parsing and encoding primitives.
//!
//! This crate centralizes parsing and generation of JSON-RPC transport
//! `Content-Length` headers used by LSP and DAP framing layers.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::fmt;

/// Canonical header name for JSON-RPC content length framing.
pub const CONTENT_LENGTH_HEADER_NAME: &str = "Content-Length";

/// Header terminator separating headers from payload body.
pub const HEADER_END: &[u8] = b"\r\n\r\n";

/// Header-level parsing errors for [`parse_content_length`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentLengthHeaderError {
    /// Header block did not include a `Content-Length` field.
    MissingContentLength,
    /// `Content-Length` value was not a valid non-negative integer.
    InvalidContentLength,
    /// Header block contained malformed lines (for example, no `:` separator).
    MalformedHeader,
}

impl fmt::Display for ContentLengthHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingContentLength => write!(f, "missing Content-Length header"),
            Self::InvalidContentLength => write!(f, "invalid Content-Length value"),
            Self::MalformedHeader => write!(f, "malformed Content-Length header block"),
        }
    }
}

impl std::error::Error for ContentLengthHeaderError {}

/// Parse `Content-Length` from a full CRLF-delimited header block.
///
/// Returns the parsed body length when present and valid.
pub fn parse_content_length(header: &str) -> Result<usize, ContentLengthHeaderError> {
    let mut found = None;

    for line in header.split("\r\n") {
        if line.is_empty() {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(ContentLengthHeaderError::MalformedHeader);
        };

        if name.trim().eq_ignore_ascii_case(CONTENT_LENGTH_HEADER_NAME) {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|_| ContentLengthHeaderError::InvalidContentLength)?;
            found = Some(parsed);
        }
    }

    found.ok_or(ContentLengthHeaderError::MissingContentLength)
}

/// Build a complete `Content-Length` header block for a payload length.
#[must_use]
pub fn encode_content_length_header(length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(CONTENT_LENGTH_HEADER_NAME.len() + 32 + HEADER_END.len() + 2);
    out.extend_from_slice(CONTENT_LENGTH_HEADER_NAME.as_bytes());
    out.extend_from_slice(b": ");
    out.extend_from_slice(length.to_string().as_bytes());
    out.extend_from_slice(HEADER_END);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        CONTENT_LENGTH_HEADER_NAME, ContentLengthHeaderError, HEADER_END,
        encode_content_length_header, parse_content_length,
    };

    #[test]
    fn parses_content_length_case_insensitive() {
        let len = parse_content_length("content-length: 12\r\nX-Test: yes");
        assert_eq!(len, Ok(12));
    }

    #[test]
    fn rejects_missing_header() {
        let error = parse_content_length("X-Test: yes");
        assert_eq!(error, Err(ContentLengthHeaderError::MissingContentLength));
    }

    #[test]
    fn rejects_invalid_header_value() {
        let error = parse_content_length("Content-Length: nope");
        assert_eq!(error, Err(ContentLengthHeaderError::InvalidContentLength));
    }

    #[test]
    fn rejects_malformed_header_lines() {
        let error = parse_content_length("Content-Length");
        assert_eq!(error, Err(ContentLengthHeaderError::MalformedHeader));
    }

    #[test]
    fn encodes_complete_header_block() {
        let encoded = encode_content_length_header(7);
        let expected = format!("{CONTENT_LENGTH_HEADER_NAME}: 7").into_bytes();
        assert!(encoded.starts_with(&expected));
        assert!(encoded.ends_with(HEADER_END));
    }
}
