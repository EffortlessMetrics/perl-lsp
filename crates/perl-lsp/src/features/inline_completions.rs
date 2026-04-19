//! Inline completions provider — delegated to perl-lsp-inline-completion (#2756).
//!
//! The upstream crate uses `utf16_line_col_to_offset` for correct UTF-16
//! character-position handling as required by the LSP protocol (§3.17).
//! The previous local implementation used raw byte offsets which gave
//! silently wrong results for documents containing non-BMP characters.

pub use perl_lsp_rs_core::providers::inline_completion::{
    InlineCompletionItem, InlineCompletionList, InlineCompletionProvider,
};
