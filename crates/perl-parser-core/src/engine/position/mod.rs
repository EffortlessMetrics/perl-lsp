//! Enhanced position tracking for incremental parsing
//!
//! This module re-exports position types from `perl-position-tracking`.

pub use perl_position_tracking::{Position, Range};
pub use perl_position_tracking::{offset_to_utf16_line_col, utf16_line_col_to_offset};

pub mod line_index;
pub mod position_mapper;
#[doc(hidden)]
pub mod positions;
