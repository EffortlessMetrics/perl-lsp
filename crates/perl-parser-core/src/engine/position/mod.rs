//! Enhanced position tracking for incremental parsing
//!
//! This module re-exports position types from `perl-position-tracking`.

pub use perl_position_tracking::{Position, Range};
pub use perl_position_tracking::{offset_to_utf16_line_col, utf16_line_col_to_offset};
pub use perl_position_tracking::mapper::{LineEnding, PositionMapper};

pub mod line_index;
