//! LSP feature module (deprecated)
//!
//! **DEPRECATED**: This module has moved to the `perl-lsp` crate.
//!
//! For backwards compatibility during the migration period, this module
//! is kept as an empty stub. Migrate to `perl_lsp::features::code_actions_provider`.
//!
//! # Migration
//!
//! ```ignore
//! // Old:
//! use perl_parser::code_actions_provider;
//!
//! // New:
//! use perl_lsp::features::code_actions_provider;
//! ```

// This module intentionally has no contents.
// All functionality has moved to the perl-lsp crate.
// Direct re-export is not possible due to circular dependency.
