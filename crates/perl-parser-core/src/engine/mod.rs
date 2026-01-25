//! Parser engine components and supporting utilities.

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub mod ast;
/// Experimental second-generation AST (work in progress).
#[allow(missing_docs)]
pub mod ast_v2;
/// Edit tracking for incremental parsing.
pub mod edit;
/// Error types and recovery strategies for parser failures.
pub mod error;
/// Heredoc content collector with FIFO ordering and indent stripping.
pub mod heredoc_collector;
/// Core parser implementation for Perl source.
pub mod parser;
/// Parser context with error recovery support.
pub mod parser_context;
/// Position tracking types and UTF-16 mapping utilities.
pub mod position;
/// Pragma tracking for `use` and related directives.
pub mod pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub mod quote_parser;
/// Parser utilities and helpers.
pub mod regex_validator;
pub mod util;
