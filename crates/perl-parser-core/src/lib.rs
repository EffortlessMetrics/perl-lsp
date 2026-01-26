//! Core parser engine for perl-parser.
//!
//! Provides the AST, parser, token stream utilities, and position mapping
//! needed by higher-level crates (semantic analysis, workspace indexing, LSP).

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
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
pub use perl_builtins as builtins;
/// Parser engine components and supporting utilities.
pub mod engine;
/// Token stream and trivia utilities for the parser.
pub mod tokens;

pub use ast_v2::{DiagnosticId, MissingKind};
/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub use engine::ast;
/// Experimental second-generation AST (work in progress).
pub use engine::ast_v2;
/// Edit tracking for incremental parsing.
pub use engine::edit;
/// Heredoc content collector with FIFO ordering and indent stripping.
pub use engine::heredoc_collector;
/// Parser context with error recovery support.
pub use engine::parser_context;
/// Pragma tracking for `use` and related directives.
pub use engine::pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub use engine::quote_parser;
/// Parser utilities and helpers.
pub use engine::util;
/// Legacy module aliases for moved engine components.
pub use engine::{error, parser, position};

/// Parser entrypoint for Perl source.
pub use engine::parser::Parser;

/// Error classification and recovery strategies for parse failures.
pub use error::classifier as error_classifier;
pub use error::recovery as error_recovery;
pub use error::recovery_parser;
pub use error_recovery::RecoveryResult;

pub use position::line_index;
pub use position::position_mapper;
#[doc(hidden)]
pub use position::positions;

pub use ast::{Node, NodeKind, SourceLocation};
pub use error::{BudgetTracker, ParseBudget, ParseError, ParseOutput, ParseResult};

pub use builtins::builtin_signatures;
pub use builtins::builtin_signatures_phf;

pub use tokens::token_stream;
pub use tokens::token_wrapper;
pub use tokens::trivia;
pub use tokens::trivia_parser;

pub use token_stream::{Token, TokenKind, TokenStream};
pub use trivia::{NodeWithTrivia, Trivia, TriviaToken};
pub use trivia_parser::{TriviaPreservingParser, format_with_trivia};
