//! UTF-8/UTF-16 position tracking and conversion.
mod convert;
mod line_index;
pub use convert::{offset_to_utf16_line_col, utf16_line_col_to_offset};
pub use line_index::LineStartsCache;
#[cfg(feature = "lsp-compat")]
mod wire;
#[cfg(feature = "lsp-compat")]
pub use wire::{WireLocation, WirePosition, WireRange};
