//! Core parser engine exports.

/// Legacy module aliases for moved engine components.
pub use crate::engine::{error, parser, position};

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub use crate::engine::ast;
/// Experimental second-generation AST (work in progress).
pub use crate::engine::ast_v2;
/// Edit tracking for incremental parsing.
pub use crate::engine::edit;
/// Heredoc content collector with FIFO ordering and indent stripping.
pub use crate::engine::heredoc_collector;
/// Recursive descent Perl parser with error recovery and AST generation.
pub use crate::engine::parser::Parser;
/// Parser context with error recovery support.
pub use crate::engine::parser_context;
/// Pragma tracking for `use` and related directives.
pub use crate::engine::pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub use crate::engine::quote_parser;
#[cfg(not(target_arch = "wasm32"))]
/// Error classification and recovery strategies for parse failures.
pub use crate::error::classifier as error_classifier;
/// Error recovery strategies for resilient parsing.
pub use crate::error::recovery as error_recovery;
/// Parser utilities and helpers.
pub use perl_parser_core::util;

/// Line ending detection and UTF-16 position mapping for LSP compliance.
pub use crate::position::{LineEnding, PositionMapper};
/// Line-to-byte offset index for fast position lookups.
pub use perl_parser_core::line_index;
