//! Enhanced position tracking for incremental parsing
//!
//! This module re-exports position types from `perl-position-tracking`.

pub use perl_position_tracking::mapper::{LineEnding, PositionMapper, apply_edit_utf8};
pub use perl_position_tracking::{
    LineIndex, LineStartsCache, Position, Range, WireLocation, WirePosition, WireRange,
};
pub use perl_position_tracking::{offset_to_utf16_line_col, utf16_line_col_to_offset};
