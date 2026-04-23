//! LSP feature catalog (deprecated)
//!
//! **DEPRECATED**: This module has moved to the `perl-lsp` crate.
//!
//! For backwards compatibility during the migration period, this module
//! is kept as a stub. Migrate to `perl_lsp::features`.
//!
//! # Migration
//!
//! ```ignore
//! // Old:
//! use perl_parser::features::map::feature_ids_from_caps;
//!
//! // New:
//! use perl_lsp::features::map::feature_ids_from_caps;
//! ```

/// Deprecated feature mapping module (moved to `perl-lsp`).
pub mod map;
