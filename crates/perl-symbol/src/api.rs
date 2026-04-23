//! Public API re-exports for `perl-symbol`.
//!
//! This module defines the full public surface. All items in submodules are
//! re-exported here explicitly (no wildcards) so the public contract is reviewable
//! at a glance.
//!
//! ## Conventions
//!
//! - `types::{SymbolKind, VarKind}` are also re-exported at the crate root for
//!   ergonomic consumer migration (semantic-analyzer and workspace-index had
//!   `pub use perl_symbol_types::{SymbolKind, VarKind};` as part of their own
//!   public API — a one-line rename keeps their downstream callers working).

// types — at crate root for ergonomics
pub use crate::types::{SymbolKind, VarKind};

// cursor — full public surface
pub use crate::cursor::{
    CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position,
    is_modchar, is_word_boundary, token_under_cursor,
};

// index — full public surface
pub use crate::index::SymbolIndex;

// surface — full public surface
pub use crate::surface::{SymbolDecl, extract_symbol_decls};
