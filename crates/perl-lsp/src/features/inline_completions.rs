//! Inline completions provider — re-exported from `perl-lsp-inline-completion`.
//!
//! The authoritative implementation lives in the microcrate, which provides
//! context-aware ghost-text completions with correct UTF-16 position handling.

pub use perl_lsp_inline_completion::{
    InlineCompletionItem, InlineCompletionList, InlineCompletionProvider,
};
