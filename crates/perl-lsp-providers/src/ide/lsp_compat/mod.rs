//! LSP compatibility shims and deprecated feature modules.
//!
//! These modules remain in `perl-parser` to support legacy imports while the
//! runtime LSP implementation lives in the `perl-lsp` crate.

// Extracted modules are now in separate crates:
// - completion -> perl-lsp-completion
// - diagnostics -> perl-lsp-diagnostics
// - inlay_hints -> perl-lsp-inlay-hints
// - rename -> perl-lsp-rename
// - code_actions -> perl-lsp-code-actions
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::code_actions instead")]
pub mod code_actions;
pub mod code_actions_pragmas;
pub mod code_actions_provider;
pub mod code_lens_provider;
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::completion instead")]
pub mod completion;
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::diagnostics instead")]
pub mod diagnostics;
pub mod document_highlight;
pub mod folding;
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::formatting instead")]
pub mod formatting;
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::inlay_hints instead")]
pub mod inlay_hints;
pub mod inline_completions;
pub mod linked_editing;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_document_link;
pub mod lsp_errors;
pub mod lsp_on_type_formatting;
pub mod lsp_selection_range;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_server;
pub mod lsp_utils;
pub mod on_type_formatting;
pub mod pull_diagnostics;
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::rename instead")]
pub mod rename;
pub mod selection_range;
pub mod signature_help;
#[cfg(feature = "lsp-compat")]
pub mod textdoc;
pub mod uri;

/// Feature catalog and capability mapping for LSP compatibility.
#[cfg(feature = "lsp-compat")]
pub mod features;
