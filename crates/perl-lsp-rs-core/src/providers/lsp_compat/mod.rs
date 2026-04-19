//! LSP compatibility shims and feature modules.
//!
//! These modules remain in `perl-lsp-providers` to support IDE integration
//! while the runtime LSP implementation lives in the `perl-lsp` crate.
//!
//! Prefer the top-level re-exports (e.g. `perl_lsp_providers::code_actions`)
//! for new code rather than the `ide::lsp_compat` paths.

pub mod code_actions_pragmas;
pub mod code_actions_provider;
pub mod code_lens_provider;
pub mod document_highlight;
pub mod folding;
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
pub mod selection_range;
pub mod signature_help;
#[cfg(feature = "lsp-compat")]
pub mod textdoc;
pub mod uri;

/// Feature catalog and capability mapping for LSP compatibility.
#[cfg(feature = "lsp-compat")]
pub mod features;
