//! LSP wire types for Position and Range.
use crate::{offset_to_utf16_line_col, utf16_line_col_to_offset};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WirePosition {
    pub line: u32,
    pub character: u32,
}
impl WirePosition {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
    pub fn from_byte_offset(source: &str, byte_offset: usize) -> Self {
        let (line, character) = offset_to_utf16_line_col(source, byte_offset);
        Self { line, character }
    }
    pub fn to_byte_offset(&self, source: &str) -> usize {
        utf16_line_col_to_offset(source, self.line, self.character)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WireRange {
    pub start: WirePosition,
    pub end: WirePosition,
}
impl WireRange {
    pub fn new(start: WirePosition, end: WirePosition) -> Self {
        Self { start, end }
    }
    pub fn from_byte_offsets(source: &str, start_byte: usize, end_byte: usize) -> Self {
        Self {
            start: WirePosition::from_byte_offset(source, start_byte),
            end: WirePosition::from_byte_offset(source, end_byte),
        }
    }
    pub fn empty(pos: WirePosition) -> Self {
        Self { start: pos, end: pos }
    }
    pub fn whole_document(source: &str) -> Self {
        Self {
            start: WirePosition::new(0, 0),
            end: WirePosition::from_byte_offset(source, source.len()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireLocation {
    pub uri: String,
    pub range: WireRange,
}
impl WireLocation {
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
impl From<WireLocation> for lsp_types::Location {
    fn from(l: WireLocation) -> Self {
        use std::sync::LazyLock;

        // Fallback URI for when parsing fails.
        // Invariant: "file:///unknown" is a valid URI per RFC 3986.
        static FALLBACK_URI: LazyLock<lsp_types::Uri> = LazyLock::new(|| {
            "file:///unknown".parse().unwrap_or_else(|_| unreachable!("file:///unknown must be a valid URI"))
        });

        let uri = match l.uri.parse::<lsp_types::Uri>() {
            Ok(u) => u,
            Err(_) => FALLBACK_URI.clone(),
        };
        Self { uri, range: l.range.into() }
    }
}
