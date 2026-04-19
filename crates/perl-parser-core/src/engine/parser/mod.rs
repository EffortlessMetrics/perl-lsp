//! Recursive descent Perl parser.
//!
//! Consumes tokens from `perl-lexer` and produces AST nodes with error recovery.
//! The parser handles operator precedence, quote-like operators, and heredocs,
//! while tracking recursion depth to prevent stack overflows on malformed input.
//!
//! # IDE-Friendly Error Recovery
//!
//! This parser uses an **IDE-friendly error recovery model**:
//!
//! - **Returns `Ok(ast)` with ERROR nodes** for most parse failures (recovered errors)
//! - **Returns `Err`** only for catastrophic failures (recursion limits, etc.)
//!
//! This means `result.is_err()` is **not** the correct way to check for parse errors.
//! Instead, check for ERROR nodes in the AST or use `parser.errors()`:
//!
//! ```rust,ignore
//! let mut parser = Parser::new(code);
//! match parser.parse() {
//!     Err(_) => println!("Catastrophic parse failure"),
//!     Ok(ast) => {
//!         // Check for recovered errors via ERROR nodes
//!         if ast.to_sexp().contains("ERROR") {
//!             println!("Parse errors recovered: {:?}", parser.errors());
//!         }
//!     }
//! }
//! ```
//!
//! ## Why IDE-Friendly?
//!
//! Traditional compilers return `Err` on any syntax error. This prevents:
//! - Code completion in incomplete code
//! - Go-to-definition while typing
//! - Hover information in files with errors
//!
//! By returning partial ASTs with ERROR nodes, editors can provide useful
//! features even when code is incomplete or contains errors.
//!
//! # Performance
//!
//! - **Time complexity**: O(n) for typical token streams
//! - **Space complexity**: O(n) for AST storage with bounded recursion memory usage
//! - **Optimizations**: Fast-path parsing and efficient recovery to maintain performance
//! - **Benchmarks**: ~150µs–1ms for typical files; low ms for large file inputs
//! - **Large-scale notes**: Tuned to scale for large workspaces (50GB PST-style scans)
//!
//! # Usage
//!
//! ```rust
//! use perl_parser_core::Parser;
//!
//! let mut parser = Parser::new("my $var = 42; sub hello { print $var; }");
//! let ast = parser.parse();
//! ```

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    error::{ParseError, ParseOutput, ParseResult, RecoveryKind, RecoverySite},
    heredoc_collector::{self, HeredocContent, PendingHeredoc, collect_all},
    quote_parser,
    token_stream::{Token, TokenKind, TokenStream},
};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Strip Perl-style line comments from `qw()` content.
///
/// In Perl, `#` inside `qw()` begins a comment that extends to the end of the
/// line (see perlop: "A # character within the list is treated as a comment
/// character"). This function removes those comment segments so that
/// `split_whitespace()` sees only the actual list elements.
fn strip_qw_comments(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            if let Some(pos) = line.find('#') {
                &line[..pos]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parser state for a single Perl source input.
///
/// Construct with [`Parser::new`] and call [`Parser::parse`] to obtain an AST.
/// Non-fatal syntax errors are collected and can be accessed via [`Parser::errors`].
pub struct Parser<'a> {
    /// Token stream providing access to lexed Perl script content
    tokens: TokenStream<'a>,
    /// Current recursion depth for overflow protection during complex Perl script parsing
    recursion_depth: usize,
    /// Position tracking for error reporting and AST location information
    last_end_position: usize,
    /// Context flag for disambiguating for-loop initialization syntax
    in_for_loop_init: bool,
    /// Depth of nested class bodies for context-sensitive class-body constructs
    in_class_body: usize,
    /// Statement boundary tracking for indirect object syntax detection
    at_stmt_start: bool,
    /// FIFO queue of pending heredoc declarations awaiting content collection
    pending_heredocs: VecDeque<PendingHeredoc>,
    /// Custom attributes registered by Attribute::Handlers declarations in this file.
    custom_attribute_handlers: HashSet<String>,
    /// Whether `use Attribute::Handlers;` has been seen in this file.
    attribute_handlers_enabled: bool,
    /// Source bytes for heredoc content collection (shared with token stream)
    src_bytes: &'a [u8],
    /// Byte cursor tracking position for heredoc content collection
    byte_cursor: usize,
    /// Start time of parsing for timeout enforcement (specifically heredocs)
    heredoc_start_time: Option<Instant>,
    /// Collection of parse errors encountered during parsing (for error recovery)
    errors: Vec<ParseError>,
    /// Optional cancellation flag for cooperative cancellation from the LSP server.
    cancellation_flag: Option<Arc<AtomicBool>>,
    /// Counter to amortize cancellation checks (only check every 64 statements)
    cancellation_check_counter: usize,
}

// Recursion limit is set conservatively to prevent stack overflow
// before the limit triggers. The actual stack usage depends on the
// number of function frames between recursion checks (about 20-30
// for the precedence parsing chain). 128 * 30 = ~3840 frames which
// is safe. Real Perl code rarely exceeds 20-30 nesting levels.
const MAX_RECURSION_DEPTH: usize = 128;

impl<'a> Parser<'a> {
    /// Create a new parser for the provided Perl source.
    ///
    /// # Arguments
    ///
    /// * `input` - Perl source code to be parsed
    ///
    /// # Returns
    ///
    /// A configured parser ready to parse the provided source.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let script = "use strict; my $filter = qr/important/;";
    /// let mut parser = Parser::new(script);
    /// // Parser ready to parse the source
    /// ```
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokens: TokenStream::new(input),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
            in_class_body: 0,
            at_stmt_start: true,
            pending_heredocs: VecDeque::new(),
            custom_attribute_handlers: HashSet::new(),
            attribute_handlers_enabled: false,
            src_bytes: input.as_bytes(),
            byte_cursor: 0,
            heredoc_start_time: None,
            errors: Vec::new(),
            cancellation_flag: None,
            cancellation_check_counter: 0,
        }
    }

    /// Create a new parser with a cancellation flag for cooperative cancellation.
    ///
    /// When the flag is set to `true`, the parser will return `Err(ParseError::Cancelled)`
    /// at the next cancellation check point (every 64 statements).
    ///
    /// # Arguments
    ///
    /// * `input` - Perl source code to parse.
    /// * `cancellation_flag` - Shared flag used to request cancellation.
    ///
    /// # Returns
    ///
    /// A parser configured with cooperative cancellation checks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use std::sync::{
    ///     atomic::AtomicBool,
    ///     Arc,
    /// };
    ///
    /// let cancellation_flag = Arc::new(AtomicBool::new(false));
    /// let mut parser = Parser::new_with_cancellation("my $x = 1;", cancellation_flag);
    /// let _ = parser.parse();
    /// ```
    ///
    /// # Arguments
    ///
    /// `input` and `cancellation_flag` configure source + cancellation.
    ///
    /// # Returns
    ///
    /// A parser configured with cooperative cancellation checks.
    ///
    /// # Examples
    ///
    /// See the cancellation usage example above.
    pub fn new_with_cancellation(input: &'a str, cancellation_flag: Arc<AtomicBool>) -> Self {
        let mut p = Parser::new(input);
        p.cancellation_flag = Some(cancellation_flag);
        p
    }

    /// Create a parser from pre-lexed tokens, skipping the lexer pass.
    ///
    /// This constructor is the integration point for the incremental parsing
    /// pipeline: when cached tokens are available for an unchanged region of
    /// source, they can be fed directly into the parser without re-lexing.
    ///
    /// # Arguments
    ///
    /// * `tokens` — Pre-lexed `Token` values produced by a prior [`TokenStream`]
    ///   pass. Trivia tokens (whitespace, comments) should already be filtered
    ///   out, as [`TokenStream::from_vec`] does not apply trivia skipping.
    ///   An `Eof` token does **not** need to be included; the stream synthesises
    ///   one when the buffer is exhausted.
    /// * `source` — The original Perl source text. This is still required for
    ///   heredoc content collection which operates directly on byte offsets in
    ///   the source rather than on the token stream.
    ///
    /// # Returns
    ///
    /// A configured parser that will consume `tokens` in order without invoking
    /// the lexer. The resulting AST is structurally identical to one produced by
    /// [`Parser::new`] with the same source, provided the token list is complete
    /// and accurate.
    ///
    /// # Context-sensitive token disambiguation
    ///
    /// The standard parser uses `relex_as_term` to re-lex ambiguous tokens (e.g.
    /// `/` as division vs. regex) in context-sensitive positions. When using
    /// pre-lexed tokens the kind is fixed from the original lex pass, so the
    /// original parse context must have been correct. In practice this means
    /// `from_tokens` is safe to use when the token stream comes from a previous
    /// successful parse of the same source.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::{Parser, Token, TokenKind, TokenStream};
    ///
    /// let source = "my $x = 42;";
    ///
    /// // Collect pre-lexed tokens (normally cached from a prior parse)
    /// let mut stream = TokenStream::new(source);
    /// let mut tokens = Vec::new();
    /// loop {
    ///     match stream.next() {
    ///         Ok(t) if t.kind == TokenKind::Eof => break,
    ///         Ok(t) => tokens.push(t),
    ///         Err(_) => break,
    ///     }
    /// }
    ///
    /// let mut parser = Parser::from_tokens(tokens, source);
    /// let ast = parser.parse()?;
    /// assert!(matches!(ast.kind, perl_parser_core::NodeKind::Program { .. }));
    /// # Ok::<(), perl_parser_core::ParseError>(())
    /// ```
    ///
    /// # Arguments
    ///
    /// * `tokens` - Pre-lexed non-trivia tokens.
    /// * `source` - Original source text used by heredoc processing.
    ///
    /// # Returns
    ///
    /// A parser that consumes the provided token vector.
    ///
    /// # Examples
    ///
    /// See the pre-lexed token example above.
    pub fn from_tokens(tokens: Vec<Token>, source: &'a str) -> Self {
        Parser {
            tokens: TokenStream::from_vec(tokens),
            recursion_depth: 0,
            last_end_position: 0,
            in_for_loop_init: false,
            in_class_body: 0,
            at_stmt_start: true,
            pending_heredocs: VecDeque::new(),
            custom_attribute_handlers: HashSet::new(),
            attribute_handlers_enabled: false,
            src_bytes: source.as_bytes(),
            byte_cursor: 0,
            heredoc_start_time: None,
            errors: Vec::new(),
            cancellation_flag: None,
            cancellation_check_counter: 0,
        }
    }

    /// Check for cooperative cancellation, amortised over every 64 calls.
    ///
    /// Returns `Err(ParseError::Cancelled)` if the cancellation flag has been set.
    #[inline]
    fn check_cancelled(&mut self) -> ParseResult<()> {
        self.cancellation_check_counter = self.cancellation_check_counter.wrapping_add(1);
        if self.cancellation_check_counter & 63 == 0 {
            if let Some(ref flag) = self.cancellation_flag {
                if flag.load(Ordering::Relaxed) {
                    return Err(ParseError::Cancelled);
                }
            }
        }
        Ok(())
    }

    /// Create a new parser with custom enhanced recovery configuration.
    ///
    /// This constructor exists for API compatibility while enhanced recovery
    /// configuration is being phased in.
    ///
    /// # Arguments
    ///
    /// * `input` - Perl source text to tokenize and parse.
    /// * `_config` - Placeholder recovery configuration parameter.
    ///
    /// # Returns
    ///
    /// A parser instance initialized for the provided source text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let parser = Parser::new_with_recovery_config("my $x = 1;", ());
    /// assert_eq!(parser.errors().len(), 0);
    /// ```
    pub fn new_with_recovery_config(input: &'a str, _config: ()) -> Self {
        Parser::new(input)
    }

    /// Parse the source and return the AST for the Parse stage.
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - Parsed AST with a `Program` root node.
    /// * `Err(ParseError)` - Non-recoverable parsing failure.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` for non-recoverable conditions such as recursion limits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let mut parser = Parser::new("my $count = 1;");
    /// let ast = parser.parse()?;
    /// assert!(matches!(ast.kind, perl_parser_core::NodeKind::Program { .. }));
    /// # Ok::<(), perl_parser_core::ParseError>(())
    /// ```
    pub fn parse(&mut self) -> ParseResult<Node> {
        // Check cancellation before starting — handles pre-set flags immediately.
        if let Some(ref flag) = self.cancellation_flag {
            if flag.load(Ordering::Relaxed) {
                return Err(ParseError::Cancelled);
            }
        }
        self.parse_program()
    }

    /// Get all parse errors collected during parsing
    ///
    /// When error recovery is enabled, the parser continues after syntax errors
    /// and collects them for later retrieval. This is useful for IDE integration
    /// where you want to show all errors at once.
    ///
    /// # Returns
    ///
    /// A slice of all `ParseError`s encountered during parsing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let mut parser = Parser::new("my $x = ; sub foo {");
    /// let _ast = parser.parse(); // Parse with recovery
    /// let errors = parser.errors();
    /// // errors will contain details about syntax errors
    /// ```
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Parse with error recovery and return comprehensive output.
    ///
    /// This method is preferred for LSP Analyze workflows and always returns
    /// a `ParseOutput` containing the AST and any collected diagnostics.
    ///
    /// # Returns
    ///
    /// `ParseOutput` with the AST and diagnostics collected during parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::Parser;
    ///
    /// let mut parser = Parser::new("my $x = ;");
    /// let output = parser.parse_with_recovery();
    /// assert!(!output.diagnostics.is_empty() || matches!(output.ast.kind, perl_parser_core::NodeKind::Program { .. }));
    /// ```
    pub fn parse_with_recovery(&mut self) -> ParseOutput {
        let ast = match self.parse() {
            Ok(node) => node,
            Err(e) => {
                // If parse() returned Err, it was a non-recoverable error (e.g. recursion limit)
                // Ensure it's recorded if not already
                if !self.errors.contains(&e) {
                    self.errors.push(e.clone());
                }

                // Return a dummy Program node with the error
                Node::new(
                    NodeKind::Program { statements: vec![] },
                    SourceLocation { start: 0, end: 0 },
                )
            }
        };

        ParseOutput::with_errors(ast, self.errors.clone())
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
mod builtin_block_list_tests;
#[cfg(test)]
mod builtin_expansion_tests;
#[cfg(test)]
mod chained_deref_method_tests;
#[cfg(test)]
mod coderef_invocation_tests;
#[cfg(test)]
mod complex_args_tests;
#[cfg(test)]
mod control_flow_expr_tests;
#[cfg(test)]
mod declaration_in_args_tests;
#[cfg(test)]
mod error_recovery_tests;
#[cfg(test)]
mod eval_goto_tests;
#[cfg(test)]
mod for_builtin_block_tests;
#[cfg(test)]
mod format_comprehensive_tests;
#[cfg(test)]
mod format_tests;
#[cfg(test)]
mod forward_declaration_tests;
#[cfg(test)]
mod from_tokens_tests;
#[cfg(test)]
mod glob_assignment_tests;
#[cfg(test)]
mod glob_tests;
#[cfg(test)]
mod hash_vs_block_tests;
#[cfg(test)]
mod heredoc_security_tests;
#[cfg(test)]
mod indirect_call_tests;
#[cfg(test)]
mod indirect_object_tests;
#[cfg(test)]
mod loop_control_tests;
#[cfg(test)]
mod qualified_variable_subscript_tests;
#[cfg(test)]
mod regex_delimiter_tests;
#[cfg(test)]
mod slash_ambiguity_tests;
#[cfg(test)]
mod statement_modifier_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tie_tests;
#[cfg(test)]
mod unclosed_block_recovery_tests;
#[cfg(test)]
mod use_overload_tests;
#[cfg(test)]
mod x_repetition_tests;

#[cfg(test)]
mod strip_qw_comments_unit_tests {
    use super::strip_qw_comments;

    #[test]
    fn test_strip_basic() {
        let result = strip_qw_comments("foo # comment\n bar");
        assert_eq!(
            result.split_whitespace().collect::<Vec<_>>(),
            vec!["foo", "bar"]
        );
    }
}
