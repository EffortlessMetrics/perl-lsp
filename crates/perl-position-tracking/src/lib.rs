//! UTF-8/UTF-16 position tracking, conversion, and span types.
//!
//! This crate provides foundational types for source location tracking in the
//! Perl LSP ecosystem:
//!
//! - [`ByteSpan`]: Byte-offset based spans for parser/AST use
//! - [`LineStartsCache`]: Efficient line index for offset-to-position conversion
//! - [`WirePosition`]/[`WireRange`]: LSP protocol-compatible position types
//!
//! # Example
//!
//! ```
//! use perl_position_tracking::{ByteSpan, LineStartsCache};
//!
//! let source = "line 1\nline 2\nline 3";
//! let cache = LineStartsCache::new(source);
//!
//! // Create a span covering "line 2"
//! let span = ByteSpan::new(7, 13);
//! assert_eq!(span.slice(source), "line 2");
//!
//! // Convert to line/column for LSP
//! let (line, col) = cache.offset_to_position(source, span.start);
//! assert_eq!(line, 1); // 0-indexed
//! assert_eq!(col, 0);
//! ```

pub use convert::{offset_to_utf16_line_col, utf16_line_col_to_offset};
pub use line_index::LineStartsCache;
pub use span::{ByteSpan, SourceLocation};
pub use mapper::{
    apply_edit_utf8, json_to_position, last_line_column_utf8, newline_count, position_to_json,
    LineEnding, PositionMapper,
};
pub use position::{Position, Range};

mod convert;
mod line_index;
pub mod mapper;
mod position;
mod span;

mod wire;
pub use wire::{WireLocation, WirePosition, WireRange};
