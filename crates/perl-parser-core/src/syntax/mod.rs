//! Syntax-level types and utilities absorbed from Wave D satellite crates.
//!
//! This module contains the internal implementations of AST-adjacent utilities
//! that were previously published as separate satellite crates. They are now
//! internal modules of `perl-parser-core`.
//!
//! # Visibility pattern
//!
//! - `error` is an internal submodule exposed via `engine::error` (and
//!   root-level re-exports like `ParseError`, `ParseOutput`, `error_classifier`).
//! - `edit` and `heredoc` are re-exported at crate root under their own names
//!   (`syntax::edit`, `heredoc_collector`).
//! - `path_normalize`, `path_security`, `percentile`, `qualified_name`,
//!   `source_file`, and `text_line` are re-exported directly at crate root
//!   so consumers use `perl_parser_core::path_normalize` etc.
//! - `quote` is only accessed via `engine::quote_parser` to keep quote-parsing
//!   concerns encapsulated behind the engine boundary.

/// Edit tracking for incremental parsing (previously `perl-edit`).
pub mod edit;
/// Error types, classification, and recovery strategies (previously `perl-error`).
pub mod error;
/// Heredoc collector and processor (previously `perl-heredoc`).
pub mod heredoc;
/// Secure workspace-relative path normalization (previously `perl-path-normalize`).
pub mod path_normalize;
/// Workspace-bound path validation and traversal prevention (previously `perl-path-security`).
pub mod path_security;
/// Percentile helpers for integer metric samples (previously `perl-percentile`).
pub mod percentile;
/// Perl qualified-name parsing, splitting, and validation helpers (previously `perl-qualified-name`).
pub mod qualified_name;
/// Quote operator parsing helpers (previously `perl-quote`).
pub mod quote;
/// Perl source-file classification helpers (previously `perl-source-file`).
pub mod source_file;
/// Text-line cursor and boundary helpers (previously `perl-text-line`).
pub mod text_line;
