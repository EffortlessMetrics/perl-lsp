//! Parser engine components and supporting utilities.

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub mod ast;
/// Experimental second-generation AST (work in progress).
pub mod ast_v2;
/// Edit tracking for incremental parsing.
pub mod edit;
/// Heredoc content collector with FIFO ordering and indent stripping.
pub mod heredoc_collector;
/// Parser context with error recovery support.
pub mod parser_context;
/// Pragma tracking for `use` and related directives.
pub mod pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub mod quote_parser;
/// Parser utilities and helpers.
pub mod util;
