//! LSP call hierarchy provider (deprecated).
//!
//! **DEPRECATED**: This module has moved to the `perl-lsp` crate.
//!
//! It remains as a compatibility stub so legacy imports continue to compile while
//! the LSP call hierarchy capability and protocol handling live in `perl_lsp`.
//!
//! # Migration
//!
//! ```ignore
//! // Old:
//! use perl_parser::call_hierarchy_provider::CallHierarchyProvider;
//!
//! // New:
//! use perl_lsp::call_hierarchy_provider::CallHierarchyProvider;
//! ```
