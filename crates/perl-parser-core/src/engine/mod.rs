//! Parser engine components and supporting utilities.

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub mod ast;
/// Edit tracking for incremental parsing (internal module, previously `perl-edit`).
pub use crate::syntax::edit;
/// Experimental second-generation AST (work in progress).
pub use perl_ast_v2 as ast_v2;
/// Error types and recovery strategies for parser failures.
pub mod error;
/// Heredoc content collector with FIFO ordering and indent stripping (internal module, previously `perl-heredoc`).
pub use crate::syntax::heredoc as heredoc_collector;
/// Core parser implementation for Perl source.
pub mod parser;
/// Parser context with error recovery support.
pub mod parser_context;
/// Position tracking types and UTF-16 mapping utilities.
pub mod position;
/// Parser for Perl quote and quote-like operators (internal module, previously `perl-quote`).
pub use crate::syntax::quote as quote_parser;
/// Pragma tracking for `use` and related directives.
pub use perl_pragma as pragma_tracker;
/// Parser utilities and helpers.
pub use perl_regex as regex_validator;
