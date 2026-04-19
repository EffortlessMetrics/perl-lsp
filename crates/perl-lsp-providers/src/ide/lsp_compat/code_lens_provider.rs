//! LSP feature module (deprecated)
//!
//! **DEPRECATED**: This module has moved to the `perl-lsp-code-lens` crate.
//!
//! For backwards compatibility during the migration period, this module
//! re-exports the extracted implementation. Migrate to `perl_lsp_code_lens`.
//!
//! # Migration
//!
//! ```ignore
//! // Old:
//! use perl_lsp_providers::ide::lsp_compat::code_lens_provider::CodeLensProvider;
//!
//! // New:
//! use perl_lsp_code_lens::{CodeLensProvider, get_shebang_lens, resolve_code_lens};
//! ```

pub use perl_lsp_rs_core::providers::code_lens::*;
