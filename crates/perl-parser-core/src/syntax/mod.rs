//! Syntax-level types and utilities absorbed from Wave D satellite crates.
//!
//! This module contains the internal implementations of AST-adjacent utilities
//! that were previously published as separate satellite crates. They are now
//! internal modules of `perl-parser-core`.

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
