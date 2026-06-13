//! Core parser engine for Perl source code.
//!
//! `perl-parser-core` is the foundational crate that wires together the lexer,
//! AST, error recovery, and position mapping into a single [`Parser`] entry
//! point. Higher-level crates -- semantic analysis, workspace indexing, and the
//! LSP server -- all build on top of this crate.
//!
//! # Key types
//!
//! | Type | Role |
//! |------|------|
//! | [`Parser`] | Recursive-descent parser; call [`Parser::parse`] to get an AST |
//! | [`Node`] / [`NodeKind`] | AST node and its discriminant (re-exported from `perl-ast`) |
//! | [`ParseError`] | Syntax error collected during parsing |
//! | [`ParseOutput`] | AST + diagnostics bundle for IDE workflows |
//! | [`Token`] / [`TokenKind`] | Lexer tokens consumed by the parser |
//! | [`SourceLocation`] | Byte-offset span for every node |
//!
//! # Quick start
//!
//! ```rust
//! use perl_parser_core::{Parser, Node, NodeKind};
//!
//! let mut parser = Parser::new("my $x = 42;");
//! let ast = parser.parse().expect("should parse");
//!
//! // The root is always a Program node
//! assert!(matches!(ast.kind, NodeKind::Program { .. }));
//!
//! // Non-fatal errors are collected, not returned as Err
//! assert!(parser.errors().is_empty());
//! ```
//!
//! For IDE workflows that need error-tolerant parsing, use
//! [`Parser::parse_with_recovery`]:
//!
//! ```rust
//! use perl_parser_core::Parser;
//!
//! let mut parser = Parser::new("if (");
//! let output = parser.parse_with_recovery();
//!
//! // Always returns an AST (possibly with ERROR nodes)
//! assert!(!output.diagnostics.is_empty());
//! ```

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![allow(
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::wildcard_imports,
    clippy::enum_glob_use,
    clippy::match_same_arms,
    clippy::if_not_else,
    clippy::struct_excessive_bools,
    clippy::items_after_statements,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::collapsible_match,
    clippy::collapsible_if,
    clippy::only_used_in_recursion,
    clippy::items_after_test_module,
    clippy::while_let_loop,
    clippy::single_range_in_vec_init,
    clippy::arc_with_non_send_sync,
    clippy::needless_range_loop,
    clippy::result_large_err,
    clippy::if_same_then_else,
    clippy::should_implement_trait,
    clippy::manual_flatten,
    clippy::needless_raw_string_hashes,
    clippy::single_char_pattern,
    clippy::uninlined_format_args
)]

/// Builtin function signatures and metadata.
pub use perl_lexer::builtins;
/// Parser engine components and supporting utilities.
pub mod engine;
/// Normalized high-level constructs lowered from the parser AST.
pub mod hir;
/// Syntax-level types absorbed from Wave D satellite crates.
pub mod syntax;
/// Token stream and trivia utilities for the parser.
pub mod tokens;

/// Index into the diagnostics array in [`ParseOutput`] (from `ast_v2`).
pub use ast_v2::{DiagnosticId, MissingKind};
/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub use engine::ast;
/// Experimental second-generation AST (work in progress).
pub use engine::ast_v2;
/// Parser context with error recovery support.
pub use engine::parser_context;
/// Pragma tracking for `use` and related directives.
pub use engine::pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub use engine::quote_parser;
/// Legacy module aliases for moved engine components.
pub use engine::{error, parser, position};
/// Parser utilities and helpers.
pub use perl_lexer::tokenizer::util;
/// Edit tracking for incremental parsing (internal module, previously `perl-edit`).
pub use syntax::edit;
/// Heredoc content collector with FIFO ordering and indent stripping (internal module, previously `perl-heredoc`).
pub use syntax::heredoc as heredoc_collector;
/// Secure workspace-relative path normalization (previously `perl-path-normalize`).
pub use syntax::path_normalize;
/// Workspace-bound path validation and traversal prevention (previously `perl-path-security`).
pub use syntax::path_security;
/// Percentile helpers for integer metric samples (previously `perl-percentile`).
pub use syntax::percentile;
/// Perl qualified-name parsing, splitting, and validation helpers (previously `perl-qualified-name`).
pub use syntax::qualified_name;
/// Canonical qw/q/qq operator content extractor shared across the workspace.
pub use syntax::quote::{parse_quote_operator_content, parse_qw_words};
/// Perl source-file classification helpers (previously `perl-source-file`).
pub use syntax::source_file;
/// Text-line cursor and boundary helpers (previously `perl-text-line`).
pub use syntax::text_line;

/// Recursive-descent parser -- the main entry point for parsing Perl source.
pub use engine::parser::Parser;

/// Error classification and recovery strategies for parse failures.
pub use error::classifier as error_classifier;
/// Error recovery helpers and strategies.
pub use error::recovery as error_recovery;
/// Result of an error recovery attempt.
pub use error_recovery::RecoveryResult;

/// Line indexing and position mapping utilities.
pub mod line_index {
    /// Fast lookup from byte offset to line number.
    pub use perl_position_tracking::LineIndex;
}
/// Line ending detection for mixed-EOL source files.
pub use position::{LineEnding, PositionMapper};

/// Core AST types re-exported for convenience.
pub use ast::{Node, NodeKind, SourceLocation};
/// Recovery-salvage metrics and classifier for accuracy closeout reporting.
pub use error::classifier::{RecoverySalvageMetrics, classify_recovery_salvage};
/// Parse error, budget, and output types.
pub use error::{
    BudgetTracker, ParseBudget, ParseError, ParseOutput, ParseResult, RecoverySalvageClass,
    RecoverySalvageProfile,
};

/// Builtin function signature lookup tables.
pub use builtins::builtin_signatures;
/// Perfect hash function (PHF) based builtin signature lookup.
pub use builtins::builtin_signatures_phf;

/// Token stream module for lexer-to-parser bridge.
pub use tokens::token_stream;
/// Lightweight token wrapper for AST integration.
pub use tokens::token_wrapper;
/// Trivia (whitespace and comments) representation.
pub use tokens::trivia;
/// Trivia-preserving parser and formatting utilities.
pub use tokens::trivia_parser;

/// Individual token, its classification, and the streaming iterator.
pub use token_stream::{Token, TokenKind, TokenStream};
/// Trivia types attached to AST nodes for formatting preservation.
pub use trivia::{NodeWithTrivia, Trivia, TriviaToken};
/// Trivia-preserving parser and source formatting helper.
pub use trivia_parser::{TriviaPreservingParser, format_with_trivia};
