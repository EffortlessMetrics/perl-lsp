//! LSP compatibility shims and deprecated feature modules.
//!
//! These modules remain in `perl-parser` to support legacy imports while the
//! runtime LSP implementation lives in the `perl-lsp` crate.

pub mod code_actions;
pub mod code_actions_enhanced;
pub mod code_actions_pragmas;
pub mod code_actions_provider;
pub mod code_lens_provider;
pub mod completion;
pub mod diagnostics;
pub mod document_highlight;
pub mod document_links;
pub mod folding;
pub mod formatting;
pub mod implementation_provider;
pub mod inlay_hints;
pub mod inlay_hints_provider;
pub mod inline_completions;
pub mod linked_editing;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_document_link;
pub mod lsp_on_type_formatting;
pub mod lsp_selection_range;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_server;
pub mod lsp_utils;
pub mod on_type_formatting;
pub mod pull_diagnostics;
pub mod references;
pub mod rename;
pub mod selection_range;
pub mod semantic_tokens;
pub mod semantic_tokens_provider;
pub mod signature_help;
#[cfg(feature = "lsp-compat")]
pub mod textdoc;
#[cfg(feature = "lsp-compat")]
pub mod type_definition;
/// Type hierarchy provider for Perl class inheritance.
pub mod type_hierarchy;
pub mod uri;
pub mod workspace_symbols;

/// Feature catalog and capability mapping for LSP compatibility.
#[cfg(feature = "lsp-compat")]
pub mod features;
