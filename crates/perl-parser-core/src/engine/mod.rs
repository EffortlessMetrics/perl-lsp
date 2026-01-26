//! Parser engine components and supporting utilities.

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub use perl_ast::ast;
/// Experimental second-generation AST (work in progress).
#[allow(missing_docs)]
pub use perl_ast::v2 as ast_v2;
/// Edit tracking for incremental parsing.
pub use perl_edit as edit;
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
pub use perl_pragma as pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub use perl_quote as quote_parser;
/// Parser utilities and helpers.
pub use perl_regex as regex_validator;
pub mod util;
