//! LSP wire types for Position and Range.
use crate::{offset_to_utf16_line_col, utf16_line_col_to_offset};
use serde::{Deserialize, Serialize};

/// LSP-compatible position measured as zero-based UTF-16 coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WirePosition {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based UTF-16 column within the line.
    pub character: u32,
}
impl WirePosition {
    /// Creates a new wire position.
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    /// Converts a byte offset in `source` into a [`WirePosition`].
    pub fn from_byte_offset(source: &str, byte_offset: usize) -> Self {
        let (line, character) = offset_to_utf16_line_col(source, byte_offset);
        Self { line, character }
    }

    /// Converts this position back into a byte offset for `source`.
    pub fn to_byte_offset(&self, source: &str) -> usize {
        utf16_line_col_to_offset(source, self.line, self.character)
    }
}

/// LSP-compatible range using UTF-16 wire positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WireRange {
    /// Inclusive range start.
    pub start: WirePosition,
    /// Exclusive range end.
    pub end: WirePosition,
}
impl WireRange {
    /// Creates a new wire range.
    pub fn new(start: WirePosition, end: WirePosition) -> Self {
        Self { start, end }
    }

    /// Converts byte offsets in `source` to a [`WireRange`].
    pub fn from_byte_offsets(source: &str, start_byte: usize, end_byte: usize) -> Self {
        Self {
            start: WirePosition::from_byte_offset(source, start_byte),
            end: WirePosition::from_byte_offset(source, end_byte),
        }
    }

    /// Creates an empty range anchored at `pos`.
    pub fn empty(pos: WirePosition) -> Self {
        Self { start: pos, end: pos }
    }

    /// Creates a range spanning the entire `source` document.
    pub fn whole_document(source: &str) -> Self {
        Self {
            start: WirePosition::new(0, 0),
            end: WirePosition::from_byte_offset(source, source.len()),
        }
    }
}

/// URI + range pair used in protocol payloads and snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireLocation {
    /// Document URI as a string.
    pub uri: String,
    /// Location range within the document.
    pub range: WireRange,
}
impl WireLocation {
    /// Creates a new wire location.
    pub fn new(uri: String, range: WireRange) -> Self {
        Self { uri, range }
    }
}
#[cfg(feature = "lsp-compat")]
impl From<WirePosition> for lsp_types::Position {
    fn from(p: WirePosition) -> Self {
        Self { line: p.line, character: p.character }
    }
}
#[cfg(feature = "lsp-compat")]
impl From<lsp_types::Position> for WirePosition {
    fn from(p: lsp_types::Position) -> Self {
        Self { line: p.line, character: p.character }
    }
}
#[cfg(feature = "lsp-compat")]
impl From<WireRange> for lsp_types::Range {
    fn from(r: WireRange) -> Self {
        Self { start: r.start.into(), end: r.end.into() }
    }
}
#[cfg(feature = "lsp-compat")]
impl From<lsp_types::Range> for WireRange {
    fn from(r: lsp_types::Range) -> Self {
        Self { start: r.start.into(), end: r.end.into() }
    }
}
#[cfg(feature = "lsp-compat")]
fn fallback_lsp_uri() -> lsp_types::Uri {
    for candidate in ["file:///unknown", "file:///", "about:blank", "urn:perl-lsp:unknown"] {
        if let Ok(uri) = candidate.parse::<lsp_types::Uri>() {
            return uri;
        }
    }

    // Last-resort fallback that avoids panicking if URI parser behavior changes unexpectedly.
    let mut suffix = 0usize;
    loop {
        let candidate = format!("http://localhost/{suffix}");
        if let Ok(uri) = candidate.parse::<lsp_types::Uri>() {
            return uri;
        }
        suffix = suffix.saturating_add(1);
    }
}

#[cfg(feature = "lsp-compat")]
impl From<WireLocation> for lsp_types::Location {
    fn from(l: WireLocation) -> Self {
        let uri = match l.uri.parse::<lsp_types::Uri>() {
            Ok(u) => u,
            Err(_) => fallback_lsp_uri(),
        };
        Self { uri, range: l.range.into() }
    }
}

#[cfg(all(test, feature = "lsp-compat"))]
mod tests {
    use super::*;

    #[test]
    fn wire_location_to_lsp_location_preserves_valid_uri() {
        let wire_location = WireLocation::new(
            "file:///tmp/example.pl".to_string(),
            WireRange::new(WirePosition::new(1, 2), WirePosition::new(3, 4)),
        );

        let location: lsp_types::Location = wire_location.into();

        assert_eq!(location.uri.as_str(), "file:///tmp/example.pl");
        assert_eq!(location.range.start.line, 1);
        assert_eq!(location.range.start.character, 2);
        assert_eq!(location.range.end.line, 3);
        assert_eq!(location.range.end.character, 4);
    }

    #[test]
    fn wire_location_to_lsp_location_uses_fallback_for_invalid_uri() {
        let wire_location = WireLocation::new(
            "not a uri".to_string(),
            WireRange::new(WirePosition::new(0, 0), WirePosition::new(0, 1)),
        );

        let location: lsp_types::Location = wire_location.into();

        assert_ne!(location.uri.as_str(), "not a uri");
        assert!(!location.uri.as_str().is_empty());
        assert_eq!(location.range.start.line, 0);
        assert_eq!(location.range.end.character, 1);
    }
}
