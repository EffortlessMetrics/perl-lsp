//! Main Perl parser implementation for Perl parsing workflow pipeline
//!
//! This module implements a high-performance recursive descent parser with operator precedence
//! handling that consumes tokens from perl-lexer and produces comprehensive ASTs for email
//! script analysis throughout the Parse → Index → Navigate → Complete → Analyze workflow.
//!
//! # LSP Workflow Integration
//!
//! The parser serves as the entry point for the Parse stage, converting raw Perl script
//! content into structured ASTs that flow through subsequent pipeline stages:
//!
//! - **Extract**: Parses Perl scripts embedded in PST Perl code
//! - **Normalize**: Provides AST foundation for standardization transformations
//! - **Thread**: Enables control flow and dependency analysis across Perl scripts
//! - **Render**: Supports AST-to-source reconstruction with formatting preservation
//! - **Index**: Facilitates symbol extraction and searchable metadata generation
//!
//! # Performance Characteristics
//!
//! Optimized for enterprise-scale Perl parsing:
//! - Handles 50GB+ Perl files with efficient memory management
//! - Recursive descent with configurable depth limits for safety
//! - Token stream abstraction minimizes memory allocation during parsing
//! - Error recovery enables continued processing of malformed Perl scripts
//!
//! # Usage Example
//!
//! ```rust
//! use perl_parser::Parser;
//!
//! let mut parser = Parser::new("my $var = 42; sub hello { print $var; }");
//! match parser.parse() {
//!     Ok(ast) => {
//!         // AST ready for LSP workflow processing
//!         println!("Parsed Perl script: {}", ast.to_sexp());
//!     }
//!     Err(e) => {
//!         // Handle parsing errors with recovery strategies
//!         eprintln!("Parse error in Perl script: {}", e);
//!     }
//! }
//! ```

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    error::{ParseError, ParseResult},
    heredoc_collector::{self, HeredocContent, PendingHeredoc, collect_all},
    quote_parser,
    token_stream::{Token, TokenKind, TokenStream},
};
use std::collections::VecDeque;
use std::sync::Arc;

/// High-performance Perl parser for Perl script analysis within LSP workflow
///
/// The parser processes Perl script content through recursive descent parsing with
/// operator precedence handling, producing comprehensive ASTs suitable for analysis
/// across all LSP workflow stages. Designed for enterprise-scale performance with
/// 50GB+ Perl file processing capabilities.
///
/// # Email Processing Context
///
/// This parser specializes in handling Perl scripts commonly found in Perl code:
/// - Email filtering and routing scripts
/// - Message processing automation code
/// - Configuration and setup scripts embedded in emails
/// - Inline Perl code within email templates and forms
///
/// # Performance Features
///
/// - Configurable recursion depth limits prevent stack overflow on malformed content
/// - Token stream abstraction minimizes memory allocation during large file processing
/// - Error recovery strategies maintain parsing progress despite syntax issues
/// - Position tracking enables precise error reporting for debugging complex Perl scripts
pub struct Parser<'a> {
    /// Token stream providing access to lexed Perl script content
    tokens: TokenStream<'a>,
    /// Current recursion depth for overflow protection during complex Perl script parsing
    recursion_depth: usize,
    /// Position tracking for error reporting and AST location information
    last_end_position: usize,
    /// Context flag for disambiguating for-loop initialization syntax
    in_for_loop_init: bool,
    /// Statement boundary tracking for indirect object syntax detection
    at_stmt_start: bool,
    /// FIFO queue of pending heredoc declarations awaiting content collection
    pending_heredocs: VecDeque<PendingHeredoc>,
    /// Source bytes for heredoc content collection (shared with token stream)
    src_bytes: &'a [u8],
    /// Byte cursor tracking position for heredoc content collection
    byte_cursor: usize,
}

const MAX_RECURSION_DEPTH: usize = 500;

impl<'a> Parser<'a> {
    /// Create a new parser for processing Perl script content within LSP workflow
    ///
    /// # Arguments
    ///
    /// * `input` - Email script source code to be parsed during Parse stage
    ///
    /// # Returns
    ///
    /// A configured parser ready for Perl script analysis with optimal settings
    /// for enterprise-scale Perl codebase processing workflows.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::Parser;
    ///
    /// let script = "use strict; my $filter = qr/important/;";
    /// let mut parser = Parser::new(script);
    /// // Parser ready for LSP workflow processing
    /// ```
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokens: TokenStream::new(input),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
            at_stmt_start: true,
            pending_heredocs: VecDeque::new(),
            src_bytes: input.as_bytes(),
            byte_cursor: 0,
        }
    }

    /// Parse Perl script content and return comprehensive AST for LSP workflow processing
    ///
    /// This method performs complete parsing of Perl script content, producing an AST
    /// suitable for analysis throughout the Parse → Index → Navigate → Complete → Analyze
    /// pipeline stages. Designed for robust processing of complex Perl scripts found
    /// in enterprise Perl files.
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - Successfully parsed AST with Program root node containing all statements
    /// * `Err(ParseError)` - Parsing failure with detailed error context for recovery strategies
    ///
    /// # Errors
    ///
    /// Returns `ParseError` when:
    /// - Email script syntax is malformed or incomplete
    /// - Unexpected end of input during parsing
    /// - Recursion depth limit exceeded (protects against deeply nested structures)
    /// - Invalid token sequences that cannot be recovered from
    ///
    /// Recovery strategy: Use error classifier to categorize failures and apply
    /// appropriate fallback parsing strategies for continued Perl parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::Parser;
    ///
    /// let mut parser = Parser::new("my $email_count = scalar(@emails);");
    /// match parser.parse() {
    ///     Ok(ast) => {
    ///         // AST ready for LSP workflow stages
    ///         assert!(matches!(ast.kind, perl_parser::NodeKind::Program { .. }));
    ///     }
    ///     Err(e) => {
    ///         // Handle parsing errors with appropriate recovery
    ///         eprintln!("Email script parsing failed: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Email Processing Context
    ///
    /// This method is optimized for parsing Perl scripts commonly found in email environments:
    /// - Email filtering and routing logic
    /// - Message processing automation scripts
    /// - Configuration scripts embedded in Perl code
    /// - Template processing code within email systems
    pub fn parse(&mut self) -> ParseResult<Node> {
        self.parse_program()
    }
}

include!("helpers.rs");
include!("heredoc.rs");
include!("statements.rs");
include!("variables.rs");
include!("control_flow.rs");
include!("declarations.rs");
include!("expressions/mod.rs");
include!("expressions/precedence.rs");
include!("expressions/unary.rs");
include!("expressions/postfix.rs");
include!("expressions/primary.rs");
include!("expressions/calls.rs");
include!("expressions/hashes.rs");
include!("expressions/quotes.rs");

#[cfg(test)]
mod tests;
