//! Context-aware Perl lexer with mode-based tokenization
//!
//! This crate provides a high-performance lexer for Perl that handles the inherently
//! context-sensitive nature of the language. The lexer uses a mode-tracking system to
//! correctly disambiguate ambiguous syntax like `/` (division vs. regex) and properly
//! parse complex constructs like heredocs, quote-like operators, and nested delimiters.
//!
//! # Architecture
//!
//! The lexer is organized around several key concepts:
//!
//! - **Mode Tracking**: [`LexerMode`] tracks whether the parser expects a term or an operator,
//!   enabling correct disambiguation of context-sensitive tokens.
//! - **Checkpointing**: [`LexerCheckpoint`] and [`Checkpointable`] support incremental parsing
//!   by allowing the lexer state to be saved and restored.
//! - **Budget Limits**: Protection against pathological input with configurable size limits
//!   for regex patterns, heredoc bodies, and delimiter nesting depth.
//! - **Position Tracking**: [`Position`] maintains line/column information for error reporting
//!   and LSP integration.
//! - **Unicode Support**: Full Unicode identifier support following Perl 5.14+ semantics.
//!
//! # Usage
//!
//! ## Basic Tokenization
//!
//! ```rust,ignore
//! use perl_lexer::{PerlLexer, TokenType};
//!
//! let code = r#"my $x = 42;"#;
//! let mut lexer = PerlLexer::new(code);
//!
//! while let Some(token) = lexer.next_token() {
//!     println!("{:?}: {}", token.token_type, token.text);
//!     if matches!(token.token_type, TokenType::EOF) {
//!         break;
//!     }
//! }
//! ```
//!
//! ## Context-Aware Parsing
//!
//! The lexer automatically tracks context to disambiguate operators:
//!
//! ```rust
//! use perl_lexer::{PerlLexer, TokenType};
//!
//! // Division operator (after a term)
//! let mut lexer = PerlLexer::new("42 / 2");
//! // Regex operator (at start of expression)
//! let mut lexer2 = PerlLexer::new("/pattern/");
//! ```
//!
//! ## Checkpointing for Incremental Parsing
//!
//! ```rust,ignore
//! use perl_lexer::{PerlLexer, Checkpointable};
//!
//! let mut lexer = PerlLexer::new("my $x = 1;");
//! let checkpoint = lexer.checkpoint();
//!
//! // Parse some tokens
//! let _ = lexer.next_token();
//!
//! // Restore to checkpoint
//! lexer.restore(&checkpoint);
//! ```
//!
//! ## Configuration Options
//!
//! ```rust
//! use perl_lexer::{PerlLexer, LexerConfig};
//!
//! let config = LexerConfig {
//!     parse_interpolation: true,  // Parse string interpolation
//!     track_positions: true,      // Track line/column positions
//!     max_lookahead: 1024,        // Maximum lookahead for disambiguation
//! };
//!
//! let mut lexer = PerlLexer::with_config("my $x = 1;", config);
//! ```
//!
//! # Context Sensitivity Examples
//!
//! Perl's grammar is highly context-sensitive. The lexer handles these cases:
//!
//! - **Division vs. Regex**: `/` is division after terms, regex at expression start
//! - **Modulo vs. Hash Sigil**: `%` is modulo after terms, hash sigil at expression start
//! - **Glob vs. Exponent**: `**` can be exponentiation or glob pattern start
//! - **Defined-or vs. Regex**: `//` is defined-or after terms, regex at expression start
//! - **Heredoc Markers**: `<<` can be left shift, here-doc, or numeric less-than-less-than
//!
//! # Budget Limits
//!
//! To prevent hangs on pathological input, the lexer enforces these limits:
//!
//! - **MAX_REGEX_BYTES**: 64KB maximum for regex patterns
//! - **MAX_HEREDOC_BYTES**: 256KB maximum for heredoc bodies
//! - **MAX_DELIM_NEST**: 128 levels maximum nesting depth for delimiters
//!
//! When limits are exceeded, the lexer emits an `UnknownRest` token preserving
//! all previously parsed symbols, allowing continued analysis.
//!
//! # Integration with perl-parser
//!
//! The lexer is designed to work seamlessly with `perl_parser::Parser`:
//!
//! ```rust,ignore
//! use perl_parser::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "sub hello { print qq{Hello, world!\\n}; }";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//! # Ok(())
//! # }
//! ```
//!
//! The parser automatically creates and manages a `PerlLexer` instance internally.

#![warn(clippy::all)]
#![allow(
    // Core allows for lexer code
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,

    // Lexer-specific patterns that are fine
    clippy::match_same_arms,
    clippy::redundant_else,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::items_after_statements,
    clippy::struct_excessive_bools,
    clippy::uninlined_format_args
)]

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

pub mod checkpoint;
pub mod error;
pub mod mode;
mod quote_handler;
pub mod token;
mod unicode;

pub use checkpoint::{CheckpointCache, Checkpointable, LexerCheckpoint};
pub use error::{LexerError, Result};
pub use mode::LexerMode;
pub use perl_position_tracking::Position;
pub use token::{StringPart, Token, TokenType};

use unicode::{is_perl_identifier_continue, is_perl_identifier_start};

/// Specification for a pending heredoc
#[derive(Clone)]
struct HeredocSpec {
    label: Arc<str>,
    body_start: usize,  // byte offset where the body begins
    allow_indent: bool, // true if we saw <<~ (Perl 5.26 indented heredocs)
}

// Budget limits to prevent hangs on pathological input
// When these limits are exceeded, the lexer gracefully truncates the token
// as UnknownRest, preserving all previously parsed symbols and allowing
// continued analysis of the remainder. LSP clients may emit a soft diagnostic
// about truncation but won't crash or hang.
const MAX_REGEX_BYTES: usize = 64 * 1024; // 64KB max for regex patterns
const MAX_HEREDOC_BYTES: usize = 256 * 1024; // 256KB max for heredoc bodies
const MAX_DELIM_NEST: usize = 128; // Max nesting depth for delimiters
const MAX_HEREDOC_DEPTH: usize = 100; // Max nesting depth for heredocs
const HEREDOC_TIMEOUT_MS: u64 = 5000; // 5 seconds timeout for heredoc parsing

/// Configuration for the lexer
#[derive(Debug, Clone)]
pub struct LexerConfig {
    /// Enable interpolation parsing in strings
    pub parse_interpolation: bool,
    /// Track token positions for error reporting
    pub track_positions: bool,
    /// Maximum lookahead for disambiguation
    pub max_lookahead: usize,
}

impl Default for LexerConfig {
    fn default() -> Self {
        Self { parse_interpolation: true, track_positions: true, max_lookahead: 1024 }
    }
}

/// Mode-aware Perl lexer
pub struct PerlLexer<'a> {
    input: &'a str,
    /// Cached input bytes for faster access
    input_bytes: &'a [u8],
    position: usize,
    mode: LexerMode,
    config: LexerConfig,
    /// Stack for nested delimiters in s{}{} constructs
    delimiter_stack: Vec<char>,
    /// Track if we're inside prototype parens after 'sub'
    in_prototype: bool,
    /// Paren depth to track when we exit prototype
    prototype_depth: usize,
    /// Current position with line/column tracking
    #[allow(dead_code)]
    current_pos: Position,
    /// Track if we just skipped a newline (for __DATA__/__END__ detection)
    after_newline: bool,
    /// Queue of pending heredocs waiting for their bodies
    pending_heredocs: Vec<HeredocSpec>,
    /// Track the byte offset of the current line's start
    line_start_offset: usize,
    /// If true, emit `HeredocBody` tokens; otherwise just consume them.
    emit_heredoc_body_tokens: bool,
    /// Current quote operator being parsed
    current_quote_op: Option<quote_handler::QuoteOperatorInfo>,
    /// Track if EOF has been emitted to prevent infinite loops
    eof_emitted: bool,
    /// Start time for timeout protection
    start_time: std::time::Instant,
}

impl<'a> PerlLexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self::with_config(input, LexerConfig::default())
    }

    /// Create a new lexer with custom configuration
    pub fn with_config(input: &'a str, config: LexerConfig) -> Self {
        Self {
            input,
            input_bytes: input.as_bytes(),
            position: 0,
            mode: LexerMode::ExpectTerm,
            config,
            delimiter_stack: Vec::new(),
            in_prototype: false,
            prototype_depth: 0,
            current_pos: Position::start(),
            after_newline: true, // Start of file counts as after newline
            pending_heredocs: Vec::new(),
            line_start_offset: 0,
            emit_heredoc_body_tokens: false,
            current_quote_op: None,
            eof_emitted: false,
            start_time: std::time::Instant::now(),
        }
    }

    /// Create a new lexer that emits `HeredocBody` tokens (for LSP folding)
    pub fn with_body_tokens(input: &'a str) -> Self {
        let mut lexer = Self::new(input);
        lexer.emit_heredoc_body_tokens = true;
        lexer
    }

    /// Normalize file start by skipping BOM if present
    fn normalize_file_start(&mut self) {
        // Skip UTF-8 BOM (EF BB BF) if at file start
        if self.position == 0 && self.matches_bytes(&[0xEF, 0xBB, 0xBF]) {
            self.position = 3;
            self.line_start_offset = 3;
        }
    }

    /// Set the lexer mode (for resetting state at statement boundaries)
    pub fn set_mode(&mut self, mode: LexerMode) {
        self.mode = mode;
    }

    /// Helper to check if remaining bytes on a line are only spaces/tabs
    #[inline]
    fn trailing_ws_only(bytes: &[u8], mut p: usize) -> bool {
        while p < bytes.len() && bytes[p] != b'\n' && bytes[p] != b'\r' {
            match bytes[p] {
                b' ' | b'\t' => p += 1,
                _ => return false,
            }
        }
        true
    }

    /// Consume a newline sequence (CRLF or LF) and update state
    #[inline]
    fn consume_newline(&mut self) {
        if self.position >= self.input.len() {
            return;
        }
        match self.input_bytes[self.position] {
            b'\r' => {
                self.position += 1;
                if self.position < self.input.len() && self.input_bytes[self.position] == b'\n' {
                    self.position += 1;
                }
            }
            b'\n' => self.advance(),
            _ => return, // not at a newline
        }
        self.after_newline = true;
        self.line_start_offset = self.position;
    }

    /// Find the end of the current line, returning both raw end and visible end (without trailing CR)
    #[inline]
    fn find_line_end(bytes: &[u8], start: usize) -> (usize, usize) {
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
            end += 1;
        }
        // Visible end strips trailing \r if followed by \n
        let visible_end = if end > start && end > 0 && bytes[end.saturating_sub(1)] == b'\r' {
            end - 1
        } else {
            end
        };
        (end, visible_end)
    }

    /// Get the next token from the input
    pub fn next_token(&mut self) -> Option<Token> {
        // Normalize file start (BOM) once
        if self.position == 0 {
            self.normalize_file_start();
        }

        // Loop to avoid recursion when processing heredocs
        loop {
            // Handle format body parsing if we're in that mode
            if matches!(self.mode, LexerMode::InFormatBody) {
                return self.parse_format_body();
            }

            // Handle data section parsing if we're in that mode
            if matches!(self.mode, LexerMode::InDataSection) {
                return self.parse_data_body();
            }

            // Check if we're inside a heredoc body BEFORE skipping whitespace
            let mut found_terminator = false;
            if !self.pending_heredocs.is_empty() {
                // Clone what we need to avoid holding a borrow
                let (body_start, label, allow_indent) =
                    if let Some(spec) = self.pending_heredocs.first() {
                        if spec.body_start > 0
                            && self.position >= spec.body_start
                            && self.position < self.input.len()
                        {
                            (spec.body_start, spec.label.clone(), spec.allow_indent)
                        } else {
                            // Not in a heredoc body yet or at EOF
                            (0, empty_arc(), false)
                        }
                    } else {
                        (0, empty_arc(), false)
                    };

                if body_start > 0 {
                    // We're inside a heredoc body - scan for the terminator

                    // Scan line by line looking for the terminator
                    while self.position < self.input.len() {
                        // Timeout protection (Issue #443)
                        if self.start_time.elapsed().as_millis() > HEREDOC_TIMEOUT_MS as u128 {
                            self.pending_heredocs.remove(0);
                            self.position = self.input.len();
                            return Some(Token {
                                token_type: TokenType::Error(Arc::from("Heredoc parsing timeout")),
                                text: Arc::from(&self.input[body_start..]),
                                start: body_start,
                                end: self.input.len(),
                            });
                        }

                        // Budget cap for huge bodies - optimized check
                        if self.position - body_start > MAX_HEREDOC_BYTES {
                            // Remove the pending heredoc to avoid infinite loop
                            self.pending_heredocs.remove(0);
                            self.position = self.input.len();
                            return Some(Token {
                                token_type: TokenType::UnknownRest,
                                text: Arc::from(&self.input[body_start..]),
                                start: body_start,
                                end: self.input.len(),
                            });
                        }

                        // Skip to start of next line if not at line start
                        // Exception: if we're at body_start exactly, we're at the heredoc body start
                        if !self.after_newline && self.position != body_start {
                            while self.position < self.input.len()
                                && self.input_bytes[self.position] != b'\n'
                                && self.input_bytes[self.position] != b'\r'
                            {
                                self.advance();
                            }
                            self.consume_newline();
                            continue;
                        }

                        // We're at line start - check if this line is the terminator
                        let line_start = self.position;
                        let (line_end, line_visible_end) =
                            Self::find_line_end(self.input_bytes, self.position);
                        let line = &self.input[line_start..line_visible_end];
                        // Strip trailing spaces/tabs (Perl allows them)
                        let trimmed_end = line.trim_end_matches([' ', '\t']);

                        // Check if this line is the terminator
                        let is_terminator = if allow_indent {
                            // Allow any leading spaces/tabs before the label
                            let mut p = 0;
                            while p < trimmed_end.len() {
                                let b = trimmed_end.as_bytes()[p];
                                if b == b' ' || b == b'\t' {
                                    p += 1;
                                } else {
                                    break;
                                }
                            }
                            trimmed_end[p..] == *label
                        } else {
                            // Must start at column 0 (no leading whitespace)
                            // The terminator is just the label (already trimmed trailing whitespace)
                            trimmed_end == &*label
                        };

                        if is_terminator {
                            // Found the terminator!
                            self.pending_heredocs.remove(0);
                            found_terminator = true;

                            // Consume past the terminator line
                            self.position = line_end;
                            self.consume_newline();

                            // Set body_start for the next pending heredoc (if any)
                            if let Some(next) = self.pending_heredocs.first_mut()
                                && next.body_start == 0
                            {
                                next.body_start = self.position;
                            }

                            // Only emit HeredocBody if requested (for folding)
                            if self.emit_heredoc_body_tokens {
                                return Some(Token {
                                    token_type: TokenType::HeredocBody(empty_arc()),
                                    text: empty_arc(),
                                    start: body_start,
                                    end: line_start,
                                });
                            }
                            // Otherwise, continue the outer loop to get the next real token (avoiding recursion)
                            break; // Break inner while loop, continue outer loop
                        }

                        // Not the terminator, continue to next line
                        self.position = line_end;
                        self.consume_newline();
                    }

                    // If we didn't find a terminator, we reached EOF - emit error token
                    if !found_terminator {
                        // Remove the pending heredoc to avoid infinite loop
                        self.pending_heredocs.remove(0);
                        self.position = self.input.len();
                        return Some(Token {
                            token_type: TokenType::UnknownRest,
                            text: Arc::from(&self.input[body_start..]),
                            start: body_start,
                            end: self.input.len(),
                        });
                    }
                }

                // If we found a terminator, continue outer loop to get next token
                if found_terminator {
                    continue; // Continue outer loop to get next token
                }
            }

            self.skip_whitespace_and_comments()?;

            // Check again if we're now in a heredoc body (might have been set during skip_whitespace)
            if !self.pending_heredocs.is_empty()
                && let Some(spec) = self.pending_heredocs.first()
                && spec.body_start > 0
                && self.position >= spec.body_start
                && self.position < self.input.len()
            {
                continue; // Go back to top of loop to process heredoc
            }

            // If we reach EOF with pending heredocs, clear them and emit EOF
            if self.position >= self.input.len() && !self.pending_heredocs.is_empty() {
                self.pending_heredocs.clear();
            }

            if self.position >= self.input.len() {
                if self.eof_emitted {
                    return None; // Stop the stream
                }
                self.eof_emitted = true;
                return Some(Token {
                    token_type: TokenType::EOF,
                    text: empty_arc(),
                    start: self.position,
                    end: self.position,
                });
            }

            let start = self.position;

            // Check for special tokens first
            if let Some(token) = self.try_heredoc() {
                return Some(token);
            }

            if let Some(token) = self.try_string() {
                return Some(token);
            }

            if let Some(token) = self.try_variable() {
                return Some(token);
            }

            if let Some(token) = self.try_number() {
                return Some(token);
            }

            if let Some(token) = self.try_identifier_or_keyword() {
                return Some(token);
            }

            // If we're expecting a delimiter for a quote operator, only try delimiter
            if matches!(self.mode, LexerMode::ExpectDelimiter) && self.current_quote_op.is_some() {
                if let Some(token) = self.try_delimiter() {
                    return Some(token);
                }
                // Do NOT fall through to try_operator / try_punct / etc.
                // Clear state first so we don't spin
                self.mode = LexerMode::ExpectOperator;
                self.current_quote_op = None;
                continue;
            }

            if let Some(token) = self.try_operator() {
                return Some(token);
            }

            if let Some(token) = self.try_delimiter() {
                return Some(token);
            }

            // If nothing else matches, return an error token
            let ch = self.current_char()?;
            self.advance();

            // Optimize error token creation - avoid expensive formatting in hot path
            let text = if ch.is_ascii() {
                // Fast path for ASCII characters
                Arc::from(&self.input[start..self.position])
            } else {
                // Slower path for Unicode
                Arc::from(ch.to_string())
            };

            return Some(Token {
                token_type: TokenType::Error(Arc::from("Unexpected character")),
                text,
                start,
                end: self.position,
            });
        } // End of loop
    }

    /// Budget guard to prevent infinite loops and timeouts (Issue #422)
    ///
    /// **Purpose**: Protect against pathological input that could cause:
    /// - Infinite loops in regex/heredoc parsing
    /// - Excessive memory consumption
    /// - LSP server hangs
    ///
    /// **Limits**:
    /// - `MAX_REGEX_BYTES` (64KB): Maximum bytes in a single regex literal
    /// - `MAX_DELIM_NEST` (128): Maximum delimiter nesting depth
    ///
    /// **Graceful Degradation**:
    /// - Budget exceeded → emit `UnknownRest` token
    /// - Jump to EOF to prevent further parsing of problematic region
    /// - LSP client can emit soft diagnostic about truncation
    /// - All previously parsed symbols remain valid
    ///
    /// **Performance**:
    /// - Fast path: inlined subtraction + comparison (~1-2 CPU cycles)
    /// - Slow path: Only triggered on pathological input
    /// - Amortized cost: O(1) per token
    #[allow(clippy::inline_always)] // Performance critical in lexer hot path
    #[inline(always)]
    fn budget_guard(&mut self, start: usize, depth: usize) -> Option<Token> {
        // Fast path: most calls won't hit limits
        let bytes_consumed = self.position - start;
        if bytes_consumed <= MAX_REGEX_BYTES && depth <= MAX_DELIM_NEST {
            return None;
        }

        // Slow path: budget exceeded - graceful degradation
        // Note: In production LSP, this event could be logged/metered for monitoring
        #[cfg(debug_assertions)]
        {
            eprintln!(
                "Budget exceeded: bytes={}, depth={}, at position={}",
                bytes_consumed, depth, self.position
            );
        }

        self.position = self.input.len();
        Some(Token {
            token_type: TokenType::UnknownRest,
            text: Arc::from(""),
            start,
            end: self.position,
        })
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&mut self) -> Option<Token> {
        let saved_pos = self.position;
        let saved_mode = self.mode;
        let saved_prototype = self.in_prototype;
        let saved_depth = self.prototype_depth;
        let saved_after_newline = self.after_newline;

        let token = self.next_token();

        self.position = saved_pos;
        self.mode = saved_mode;
        self.in_prototype = saved_prototype;
        self.prototype_depth = saved_depth;
        self.after_newline = saved_after_newline;

        token
    }

    /// Get all remaining tokens
    pub fn collect_tokens(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            if token.token_type == TokenType::EOF {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    /// Reset the lexer to the beginning
    pub fn reset(&mut self) {
        self.position = 0;
        self.mode = LexerMode::ExpectTerm;
        self.delimiter_stack.clear();
        self.in_prototype = false;
        self.prototype_depth = 0;
        self.after_newline = true;
        self.pending_heredocs.clear();
        self.line_start_offset = 0;
    }

    /// Switch lexer to format body parsing mode
    pub fn enter_format_mode(&mut self) {
        self.mode = LexerMode::InFormatBody;
    }

    // Internal helper methods

    #[allow(clippy::inline_always)] // Performance critical in lexer hot path
    #[inline(always)]
    fn current_char(&self) -> Option<char> {
        if self.position < self.input_bytes.len() {
            // For ASCII, direct access is safe
            let byte = unsafe { *self.input_bytes.get_unchecked(self.position) };
            if byte < 128 {
                Some(byte as char)
            } else {
                // For non-ASCII, fall back to proper UTF-8 parsing
                self.input.get(self.position..).and_then(|s| s.chars().next())
            }
        } else {
            None
        }
    }

    #[inline(always)]
    fn peek_char(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input_bytes.len() {
            // For ASCII, direct access is safe
            let byte = unsafe { *self.input_bytes.get_unchecked(pos) };
            if byte < 128 {
                Some(byte as char)
            } else {
                // For non-ASCII, use chars iterator
                self.input.get(self.position..).and_then(|s| s.chars().nth(offset))
            }
        } else {
            None
        }
    }

    #[allow(clippy::inline_always)] // Performance critical in lexer hot path
    #[inline(always)]
    fn advance(&mut self) {
        if self.position < self.input_bytes.len() {
            let byte = unsafe { *self.input_bytes.get_unchecked(self.position) };
            if byte < 128 {
                // ASCII fast path
                self.position += 1;
            } else if let Some(ch) = self.input.get(self.position..).and_then(|s| s.chars().next())
            {
                self.position += ch.len_utf8();
            }
        }
    }

    /// Fast byte-level check for ASCII characters
    #[inline]
    fn peek_byte(&self, offset: usize) -> Option<u8> {
        let pos = self.position + offset;
        if pos < self.input_bytes.len() { Some(self.input_bytes[pos]) } else { None }
    }

    /// Check if the next bytes match a pattern (ASCII only)
    #[inline]
    fn matches_bytes(&self, pattern: &[u8]) -> bool {
        let end = self.position + pattern.len();
        if end <= self.input_bytes.len() {
            &self.input_bytes[self.position..end] == pattern
        } else {
            false
        }
    }

    #[inline]
    fn skip_whitespace_and_comments(&mut self) -> Option<()> {
        // Don't reset after_newline if we're at the start of a line
        if self.position > 0 && self.position != self.line_start_offset {
            self.after_newline = false;
        }

        while self.position < self.input_bytes.len() {
            let byte = unsafe { *self.input_bytes.get_unchecked(self.position) };
            match byte {
                // Fast path for ASCII whitespace - batch process
                b' ' => {
                    // Batch skip spaces for better cache efficiency
                    let start = self.position;
                    while self.position < self.input_bytes.len()
                        && unsafe { *self.input_bytes.get_unchecked(self.position) } == b' '
                    {
                        self.position += 1;
                    }
                    // Continue outer loop if we processed any spaces
                    if self.position > start {
                        // Loop naturally continues to next iteration
                    }
                }
                b'\t' => {
                    // Batch skip tabs
                    let start = self.position;
                    while self.position < self.input_bytes.len()
                        && unsafe { *self.input_bytes.get_unchecked(self.position) } == b'\t'
                    {
                        self.position += 1;
                    }
                    if self.position > start {
                        // Loop naturally continues to next iteration
                    }
                }
                b'\r' | b'\n' => {
                    self.consume_newline();

                    // Set body_start for the FIRST pending heredoc that needs it (FIFO)
                    // Only check if we have pending heredocs to avoid unnecessary work
                    if !self.pending_heredocs.is_empty() {
                        for spec in &mut self.pending_heredocs {
                            if spec.body_start == 0 {
                                spec.body_start = self.position;
                                break; // Only set for the first unresolved heredoc
                            }
                        }
                    }
                }
                b'#' => {
                    // In ExpectDelimiter mode, '#' is a delimiter, not a comment
                    if matches!(self.mode, LexerMode::ExpectDelimiter) {
                        break;
                    }

                    // Skip line comment using memchr for fast newline search
                    self.position += 1; // Skip # directly

                    // Use memchr to find newline quickly
                    if let Some(newline_offset) =
                        memchr::memchr(b'\n', &self.input_bytes[self.position..])
                    {
                        self.position += newline_offset;
                    } else {
                        // No newline found, skip to end
                        self.position = self.input_bytes.len();
                    }
                }
                _ => {
                    // For non-ASCII whitespace, use char check only when needed
                    if byte >= 128
                        && let Some(ch) = self.current_char()
                        && ch.is_whitespace()
                    {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
        }
        Some(())
    }

    fn try_heredoc(&mut self) -> Option<Token> {
        // Check for heredoc start
        if self.peek_byte(0) != Some(b'<') || self.peek_byte(1) != Some(b'<') {
            return None;
        }

        let start = self.position;
        let mut text = String::from("<<");
        self.position += 2; // Skip <<

        // Check for indented heredoc (~)
        let allow_indent = if self.current_char() == Some('~') {
            text.push('~');
            self.advance();
            true
        } else {
            false
        };

        // Skip whitespace
        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Optional backslash disables interpolation, treat like single-quoted label
        let backslashed = if self.current_char() == Some('\\') {
            text.push('\\');
            self.advance();
            true
        } else {
            false
        };

        // Parse delimiter
        let delimiter = if self.position < self.input.len() {
            match self.current_char() {
                Some('"') if !backslashed => {
                    // Double-quoted delimiter
                    text.push('"');
                    self.advance();
                    let mut delim = String::new();
                    while self.position < self.input.len() {
                        if let Some(ch) = self.current_char() {
                            if ch == '"' {
                                text.push('"');
                                self.advance();
                                break;
                            }
                            delim.push(ch);
                            text.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    delim
                }
                Some('\'') if !backslashed => {
                    // Single-quoted delimiter
                    text.push('\'');
                    self.advance();
                    let mut delim = String::new();
                    while self.position < self.input.len() {
                        if let Some(ch) = self.current_char() {
                            if ch == '\'' {
                                text.push('\'');
                                self.advance();
                                break;
                            }
                            delim.push(ch);
                            text.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    delim
                }
                Some('`') if !backslashed => {
                    // Backtick delimiter
                    text.push('`');
                    self.advance();
                    let mut delim = String::new();
                    while self.position < self.input.len() {
                        if let Some(ch) = self.current_char() {
                            if ch == '`' {
                                text.push('`');
                                self.advance();
                                break;
                            }
                            delim.push(ch);
                            text.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    delim
                }
                Some(c) if is_perl_identifier_start(c) => {
                    // Bare word delimiter
                    let mut delim = String::new();
                    while self.position < self.input.len() {
                        if let Some(c) = self.current_char() {
                            if is_perl_identifier_continue(c) {
                                delim.push(c);
                                text.push(c);
                                self.advance();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    delim
                }
                _ => {
                    // Not a valid heredoc delimiter - reset position and return None
                    // This allows << to be parsed as bitshift operator (e.g., 1 << 2)
                    self.position = start;
                    return None;
                }
            }
        } else {
            // No delimiter found - reset position and return None
            self.position = start;
            return None;
        };

        // For now, return a placeholder token
        // The actual heredoc body would be parsed later when we encounter it
        self.mode = LexerMode::ExpectOperator;

        // Recursion depth limit (Issue #443)
        if self.pending_heredocs.len() >= MAX_HEREDOC_DEPTH {
            return Some(Token {
                token_type: TokenType::Error(Arc::from("Heredoc nesting too deep")),
                text: Arc::from(text),
                start,
                end: self.position,
            });
        }

        // Queue the heredoc spec with its label
        self.pending_heredocs.push(HeredocSpec {
            label: Arc::from(delimiter.as_str()),
            body_start: 0, // Will be set when we see the newline after this line
            allow_indent,
        });

        Some(Token {
            token_type: TokenType::HeredocStart,
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    fn try_string(&mut self) -> Option<Token> {
        let start = self.position;
        let quote = self.current_char()?;

        match quote {
            '"' => self.parse_double_quoted_string(start),
            '\'' => self.parse_single_quoted_string(start),
            '`' => self.parse_backtick_string(start),
            'q' if self.peek_char(1) == Some('{') => self.parse_q_string(start),
            _ => None,
        }
    }

    #[inline]
    fn try_number(&mut self) -> Option<Token> {
        let start = self.position;

        // Fast byte check for digits - optimized bounds checking
        let bytes = self.input_bytes;
        if self.position >= bytes.len()
            || !unsafe { bytes.get_unchecked(self.position) }.is_ascii_digit()
        {
            return None;
        }

        // Consume initial digits - unrolled for better performance
        let mut pos = self.position;
        while pos < bytes.len() {
            let byte = unsafe { *bytes.get_unchecked(pos) };
            if byte.is_ascii_digit() || byte == b'_' {
                pos += 1;
            } else {
                break;
            }
        }
        self.position = pos;

        // Check for decimal point - optimized with single bounds check
        if pos < bytes.len() && unsafe { *bytes.get_unchecked(pos) } == b'.' {
            // Peek ahead to see what follows the dot
            let has_following_digit = pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_digit();

            // Optimized dot consumption logic
            let should_consume_dot = has_following_digit || {
                pos + 1 >= bytes.len() || {
                    // Use bitwise operations for faster character classification
                    let next_byte = bytes[pos + 1];
                    // Whitespace, delimiters, operators - optimized check
                    next_byte <= b' '
                        || matches!(
                            next_byte,
                            b';' | b','
                                | b')'
                                | b'}'
                                | b']'
                                | b'+'
                                | b'-'
                                | b'*'
                                | b'/'
                                | b'%'
                                | b'='
                                | b'<'
                                | b'>'
                                | b'!'
                                | b'&'
                                | b'|'
                                | b'^'
                                | b'~'
                                | b'e'
                                | b'E'
                        )
                }
            };

            if should_consume_dot {
                pos += 1; // consume the dot
                // Consume fractional digits - batch processing
                while pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'_') {
                    pos += 1;
                }
                self.position = pos;
            }
        }

        // Check for exponent - optimized
        if pos < bytes.len() && (bytes[pos] == b'e' || bytes[pos] == b'E') {
            let exp_start = pos;
            pos += 1; // consume 'e' or 'E'

            // Check for optional sign
            if pos < bytes.len() && (bytes[pos] == b'+' || bytes[pos] == b'-') {
                pos += 1;
            }

            // Must have at least one digit after exponent
            let digit_start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }

            // If no digits after exponent, backtrack
            if pos == digit_start {
                pos = exp_start;
            }

            self.position = pos;
        }

        // Avoid string slicing for common number cases - use Arc::from directly on slice
        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;

        Some(Token {
            token_type: TokenType::Number(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    fn parse_decimal_number(&mut self, start: usize) -> Option<Token> {
        // We're at the dot, consume it
        self.advance();

        // Parse the fractional part
        while self.position < self.input_bytes.len() {
            let byte = self.input_bytes[self.position];
            match byte {
                b'0'..=b'9' | b'_' => self.position += 1,
                b'e' | b'E' => {
                    // Handle scientific notation
                    self.advance();
                    if self.position < self.input_bytes.len() {
                        let next = self.input_bytes[self.position];
                        if next == b'+' || next == b'-' {
                            self.advance();
                        }
                    }
                    // Parse exponent digits
                    while self.position < self.input_bytes.len()
                        && self.input_bytes[self.position].is_ascii_digit()
                    {
                        self.position += 1;
                    }
                    break;
                }
                _ => break,
            }
        }

        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;

        Some(Token {
            token_type: TokenType::Number(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    fn try_variable(&mut self) -> Option<Token> {
        let start = self.position;
        let sigil = self.current_char()?;

        match sigil {
            '$' | '@' | '%' | '*' => {
                // In ExpectOperator mode, treat % and * as operators rather than sigils
                if self.mode == LexerMode::ExpectOperator && matches!(sigil, '*' | '%') {
                    return None;
                }
                self.advance();

                // Special case: After ->, sigils followed by { or [ should be tokenized separately
                // This is for postfix dereference like ->@*, ->%{}, ->@[]
                // We need to be careful with Unicode - check if we have enough bytes and valid char boundaries
                let check_arrow = self.position >= 3
                    && self.position.saturating_sub(1) <= self.input.len()
                    && self.input.is_char_boundary(self.position.saturating_sub(3))
                    && self.input.is_char_boundary(self.position.saturating_sub(1));

                if check_arrow
                    && {
                        let saved = self.position;
                        self.position -= 3;
                        let arrow = self.matches_bytes(b"->");
                        self.position = saved;
                        arrow
                    }
                    && matches!(self.current_char(), Some('{' | '[' | '*'))
                {
                    // Just return the sigil
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;

                    return Some(Token {
                        token_type: TokenType::Identifier(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }

                // Check for $# (array length operator)
                if sigil == '$' && self.current_char() == Some('#') {
                    self.advance(); // consume #
                    // Now parse the array name
                    while let Some(ch) = self.current_char() {
                        if is_perl_identifier_continue(ch) {
                            self.advance();
                        } else if ch == ':' && self.peek_char(1) == Some(':') {
                            // Package-qualified array name
                            self.advance();
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;

                    return Some(Token {
                        token_type: TokenType::Identifier(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }

                // Check for special cases like ${^MATCH} or ${::{foo}} or *{$glob}
                if self.current_char() == Some('{') {
                    // Peek ahead to decide if we should consume the brace
                    let next_char = self.peek_char(1);

                    // Check if this is a dereference like @{$ref} or @{[...]}
                    // If the next char suggests dereference, don't consume the brace
                    if sigil != '*'
                        && matches!(
                            next_char,
                            Some('$' | '@' | '%' | '*' | '&' | '[' | ' ' | '\t' | '\n' | '\r')
                        )
                    {
                        // This is a dereference, don't consume the brace
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectOperator;

                        return Some(Token {
                            token_type: TokenType::Identifier(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        });
                    }

                    self.advance(); // consume {

                    // Handle special variables with caret
                    if self.current_char() == Some('^') {
                        self.advance(); // consume ^
                        // Parse the special variable name
                        while let Some(ch) = self.current_char() {
                            if ch == '}' {
                                self.advance(); // consume }
                                break;
                            } else if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    // Handle stash access like $::{foo}
                    else if self.current_char() == Some(':') && self.peek_char(1) == Some(':') {
                        self.advance(); // consume first :
                        self.advance(); // consume second :
                        // Skip optional { and }
                        if self.current_char() == Some('{') {
                            self.advance();
                        }
                        // Parse the name
                        while let Some(ch) = self.current_char() {
                            if ch == '}' {
                                self.advance();
                                if self.current_char() == Some('}') {
                                    self.advance(); // consume closing } of ${...}
                                }
                                break;
                            } else if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    // Regular braced variable like ${foo} or glob like *{$glob}
                    else {
                        // Check if this is a dereference like ${$ref} or @{$ref} or @{[...]}
                        // If the next char is a sigil or other expression starter, we should stop here and let the parser handle it
                        // EXCEPT for globs - *{$glob} should be parsed as one token
                        // Also check for empty braces or EOF - in these cases we should split the tokens
                        if sigil != '*'
                            && (matches!(
                                self.current_char(),
                                Some(
                                    '$' | '@'
                                        | '%'
                                        | '*'
                                        | '&'
                                        | '['
                                        | ' '
                                        | '\t'
                                        | '\n'
                                        | '\r'
                                        | '}'
                                )
                            ) || self.current_char().is_none())
                        {
                            // This is a dereference or empty/invalid brace, backtrack
                            self.position = start + 1; // Just past the sigil
                            let text = &self.input[start..self.position];
                            self.mode = LexerMode::ExpectOperator;

                            return Some(Token {
                                token_type: TokenType::Identifier(Arc::from(text)),
                                text: Arc::from(text),
                                start,
                                end: self.position,
                            });
                        }

                        // For glob access, we need to consume everything inside braces
                        if sigil == '*' {
                            let mut brace_depth: usize = 1;
                            while let Some(ch) = self.current_char() {
                                if ch == '{' {
                                    brace_depth += 1;
                                } else if ch == '}' {
                                    brace_depth = brace_depth.saturating_sub(1);
                                    if brace_depth == 0 {
                                        self.advance(); // consume final }
                                        break;
                                    }
                                }
                                self.advance();
                            }
                        } else {
                            // Regular variable
                            while let Some(ch) = self.current_char() {
                                if ch == '}' {
                                    self.advance(); // consume }
                                    break;
                                } else if is_perl_identifier_continue(ch) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                // Parse regular variable name
                else if let Some(ch) = self.current_char() {
                    if is_perl_identifier_start(ch) {
                        while let Some(ch) = self.current_char() {
                            if is_perl_identifier_continue(ch) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        // Handle package-qualified segments like Foo::bar
                        while self.current_char() == Some(':') && self.peek_char(1) == Some(':') {
                            self.advance();
                            self.advance();
                            while let Some(ch) = self.current_char() {
                                if is_perl_identifier_continue(ch) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    // Handle special punctuation variables
                    else if sigil == '$'
                        && matches!(
                            ch,
                            '?' | '!'
                                | '@'
                                | '&'
                                | '`'
                                | '\''
                                | '.'
                                | '/'
                                | '\\'
                                | '|'
                                | '+'
                                | '-'
                                | '['
                                | ']'
                                | '$'
                        )
                    {
                        self.advance(); // consume the special character
                    }
                    // Handle special array/hash punctuation variables
                    else if (sigil == '@' || sigil == '%') && matches!(ch, '+' | '-') {
                        self.advance(); // consume the + or -
                    }
                }

                let text = &self.input[start..self.position];
                self.mode = LexerMode::ExpectOperator;

                Some(Token {
                    token_type: TokenType::Identifier(Arc::from(text)),
                    text: Arc::from(text),
                    start,
                    end: self.position,
                })
            }
            _ => None,
        }
    }

    /// Return next non-space char without consuming.
    fn peek_nonspace(&self) -> Option<char> {
        let mut i = self.position;
        while i < self.input.len() {
            let c = self.input.get(i..).and_then(|s| s.chars().next())?;
            if c.is_whitespace() {
                i += c.len_utf8();
                continue;
            }
            return Some(c);
        }
        None
    }

    /// Is `c` a valid quote-like delimiter? (non-alnum, including paired)
    fn is_quote_delim(c: char) -> bool {
        // Quote delimiters are punctuation, but not whitespace or control characters
        !c.is_ascii_alphanumeric() && !c.is_whitespace() && !c.is_control()
    }

    #[inline]
    fn try_identifier_or_keyword(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.current_char()?;

        if is_perl_identifier_start(ch) {
            // Special case: substitution/transliteration with single-quote delimiter
            // The single quote is considered an identifier continuation, so we need to
            // detect these operators before consuming it as part of an identifier.
            if ch == 's' && self.peek_char(1) == Some('\'') {
                self.advance(); // consume 's'
                return self.parse_substitution(start);
            } else if ch == 'y' && self.peek_char(1) == Some('\'') {
                self.advance(); // consume 'y'
                return self.parse_transliteration(start);
            } else if ch == 't' && self.peek_char(1) == Some('r') && self.peek_char(2) == Some('\'')
            {
                self.advance(); // consume 't'
                self.advance(); // consume 'r'
                return self.parse_transliteration(start);
            }

            while let Some(ch) = self.current_char() {
                if is_perl_identifier_continue(ch) {
                    self.advance();
                } else {
                    break;
                }
            }
            // Handle package-qualified identifiers like Foo::bar
            while self.current_char() == Some(':') && self.peek_char(1) == Some(':') {
                // consume '::'
                self.advance();
                self.advance();

                // consume following identifier segment if present
                if let Some(ch) = self.current_char()
                    && is_perl_identifier_start(ch)
                {
                    self.advance();
                    while let Some(ch) = self.current_char() {
                        if is_perl_identifier_continue(ch) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }

            let text = &self.input[start..self.position];

            // Check for __DATA__ and __END__ markers using exact match
            // Only recognize these in code channel, not inside data/format sections or heredocs
            let in_code_channel =
                !matches!(self.mode, LexerMode::InDataSection | LexerMode::InFormatBody)
                    && self.pending_heredocs.is_empty();

            let marker = if in_code_channel {
                if text == "__DATA__" {
                    Some("__DATA__")
                } else if text == "__END__" {
                    Some("__END__")
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(marker_text) = marker {
                // These must be at the beginning of a line
                // Use the after_newline flag to determine if we're at line start
                if self.after_newline {
                    // Check if rest of line is only whitespace
                    // Only treat as data marker if line has no trailing junk
                    if Self::trailing_ws_only(self.input_bytes, self.position) {
                        // Consume the rest of the line (the marker line)
                        while self.position < self.input.len()
                            && self.input_bytes[self.position] != b'\n'
                        {
                            self.advance();
                        }
                        if self.position < self.input.len()
                            && self.input_bytes[self.position] == b'\n'
                        {
                            self.advance();
                        }

                        // Switch to data section mode
                        self.mode = LexerMode::InDataSection;

                        return Some(Token {
                            token_type: TokenType::DataMarker(Arc::from(marker_text)),
                            text: Arc::from(marker_text),
                            start,
                            end: self.position,
                        });
                    }
                }
            }

            // Check for substitution/transliteration operators
            #[allow(clippy::collapsible_if)]
            if matches!(text, "s" | "tr" | "y") {
                if let Some(next) = self.current_char() {
                    // Check if followed by a delimiter
                    if matches!(
                        next,
                        '/' | '|'
                            | '\''
                            | '{'
                            | '['
                            | '('
                            | '<'
                            | '!'
                            | '#'
                            | '@'
                            | '$'
                            | '%'
                            | '^'
                            | '&'
                            | '*'
                            | '+'
                            | '='
                            | '~'
                            | '`'
                    ) {
                        match text {
                            "s" => {
                                return self.parse_substitution(start);
                            }
                            "tr" | "y" => {
                                return self.parse_transliteration(start);
                            }
                            unexpected => {
                                // Return diagnostic token instead of panicking
                                return Some(Token {
                                    token_type: TokenType::Error(Arc::from(format!(
                                        "Unexpected substitution operator '{}': expected 's', 'tr', or 'y' at position {}",
                                        unexpected, start
                                    ))),
                                    text: Arc::from(unexpected),
                                    start,
                                    end: self.position,
                                });
                            }
                        }
                    }
                }
            }

            let token_type = if is_keyword(text) {
                // Check for special keywords that affect lexer mode
                match text {
                    "if" | "unless" | "while" | "until" | "for" | "foreach" => {
                        self.mode = LexerMode::ExpectTerm;
                    }
                    "sub" => {
                        self.in_prototype = true;
                    }
                    // Quote operators expect a delimiter next (must be immediately adjacent)
                    op if quote_handler::is_quote_operator(op) => {
                        // For regex operators like 'm', 's', 'tr', 'y', delimiter must be immediately adjacent
                        // For quote operators like 'q', 'qq', 'qw', 'qr', 'qx', we allow whitespace
                        let next_char = if matches!(op, "m" | "s" | "tr" | "y") {
                            self.current_char() // Must be immediately adjacent
                        } else {
                            self.peek_nonspace() // Can skip whitespace
                        };

                        if let Some(next) = next_char {
                            if Self::is_quote_delim(next) {
                                self.mode = LexerMode::ExpectDelimiter;
                                self.current_quote_op = Some(quote_handler::QuoteOperatorInfo {
                                    operator: op.to_string(),
                                    delimiter: '\0', // Will be set when we see the delimiter
                                    start_pos: start,
                                });

                                // Don't return a keyword token - continue to parse the delimiter
                                // Skip any whitespace between operator and delimiter
                                while let Some(ch) = self.current_char() {
                                    if ch.is_whitespace() {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }

                                // Get the delimiter
                                #[allow(clippy::collapsible_if)]
                                if let Some(delim) = self.current_char() {
                                    if !delim.is_alphanumeric() {
                                        self.advance();
                                        if let Some(ref mut info) = self.current_quote_op {
                                            info.delimiter = delim;
                                        }
                                        // Parse the quote operator content and return the complete token
                                        return self.parse_quote_operator(delim);
                                    }
                                }
                            } else {
                                // Not a quote operator here → treat as IDENTIFIER
                                self.current_quote_op = None;
                                self.mode = LexerMode::ExpectOperator;
                                return Some(Token {
                                    token_type: TokenType::Identifier(Arc::from(text)),
                                    start,
                                    end: self.position,
                                    text: Arc::from(text),
                                });
                            }
                        } else {
                            // End-of-input after the word → also treat as IDENTIFIER
                            self.current_quote_op = None;
                            self.mode = LexerMode::ExpectOperator;
                            return Some(Token {
                                token_type: TokenType::Identifier(Arc::from(text)),
                                start,
                                end: self.position,
                                text: Arc::from(text),
                            });
                        }
                        // If we get here but haven't returned, something went wrong
                        // Fall through to treat as identifier
                        self.current_quote_op = None;
                        self.mode = LexerMode::ExpectOperator;
                        return Some(Token {
                            token_type: TokenType::Identifier(Arc::from(text)),
                            start,
                            end: self.position,
                            text: Arc::from(text),
                        });
                    }
                    // Format declarations need special handling
                    "format" => {
                        // We'll need to check for the = after the format name
                        // For now, just mark that we saw format
                    }
                    _ => {}
                }
                TokenType::Keyword(Arc::from(text))
            } else {
                self.mode = LexerMode::ExpectOperator;
                TokenType::Identifier(Arc::from(text))
            };

            Some(Token { token_type, text: Arc::from(text), start, end: self.position })
        } else {
            None
        }
    }

    /// Parse data section body - consumes everything to EOF
    fn parse_data_body(&mut self) -> Option<Token> {
        if self.position >= self.input.len() {
            // Already at EOF
            self.mode = LexerMode::ExpectTerm;
            return Some(Token {
                token_type: TokenType::EOF,
                text: Arc::from(""),
                start: self.position,
                end: self.position,
            });
        }

        let start = self.position;
        // Consume everything to EOF
        let body = &self.input[self.position..];
        self.position = self.input.len();

        // Reset mode for next parse (though we're at EOF)
        self.mode = LexerMode::ExpectTerm;

        Some(Token {
            token_type: TokenType::DataBody(Arc::from(body)),
            text: Arc::from(body),
            start,
            end: self.position,
        })
    }

    /// Parse format body - consumes until a line with just a dot
    fn parse_format_body(&mut self) -> Option<Token> {
        let start = self.position;
        let mut body = String::new();
        let mut line_start = true;

        while self.position < self.input.len() {
            // Check if we're at the start of a line and the next char is a dot
            if line_start && self.current_char() == Some('.') {
                // Check if this line contains only a dot
                let mut peek_pos = self.position + 1;
                let mut found_terminator = true;

                // Skip any trailing whitespace on the dot line
                while peek_pos < self.input.len() {
                    match self.input_bytes[peek_pos] {
                        b' ' | b'\t' | b'\r' => peek_pos += 1,
                        b'\n' => break,
                        _ => {
                            found_terminator = false;
                            break;
                        }
                    }
                }

                if found_terminator {
                    // We found the terminating dot, consume it
                    self.position = peek_pos;
                    if self.position < self.input.len() && self.input_bytes[self.position] == b'\n'
                    {
                        self.position += 1;
                    }

                    // Switch back to normal mode
                    self.mode = LexerMode::ExpectTerm;

                    return Some(Token {
                        token_type: TokenType::FormatBody(Arc::from(body.clone())),
                        text: Arc::from(body),
                        start,
                        end: self.position,
                    });
                }
            }

            // Not a terminator, consume the character
            match self.current_char() {
                Some(ch) => {
                    body.push(ch);
                    self.advance();

                    // Track if we're at the start of a line
                    line_start = ch == '\n';
                }
                None => {
                    // Reached EOF without finding terminator
                    break;
                }
            }
        }

        // If we reach here, we didn't find a terminator
        self.mode = LexerMode::ExpectTerm;
        Some(Token {
            token_type: TokenType::Error(Arc::from("Unterminated format body")),
            text: Arc::from(body),
            start,
            end: self.position,
        })
    }

    fn try_operator(&mut self) -> Option<Token> {
        // Skip operator parsing if we're expecting a delimiter for a quote operator
        if matches!(self.mode, LexerMode::ExpectDelimiter) && self.current_quote_op.is_some() {
            return None;
        }

        let start = self.position;
        let ch = self.current_char()?;

        // ═══════════════════════════════════════════════════════════════════════
        // SLASH DISAMBIGUATION STRATEGY (Issue #422)
        // ═══════════════════════════════════════════════════════════════════════
        //
        // Perl's `/` character is ambiguous:
        //   - Division operator: `$x / 2`
        //   - Regex delimiter: `/pattern/`
        //   - Defined-or operator: `$x // $y`
        //
        // **Disambiguation Strategy (Context-Aware Heuristics):**
        //
        // 1. **Mode-Based Decision (Primary)**:
        //    - `LexerMode::ExpectTerm` → `/` starts a regex
        //      Examples: `if (/pattern/)`, `=~ /test/`, `( /regex/`
        //    - `LexerMode::ExpectOperator` → `/` is division or `//`
        //      Examples: `$x / 2`, `$x // $y`, `) / 3`
        //
        // 2. **Context Heuristics (Secondary - Implicit in Mode)**:
        //    Mode is set based on previous token:
        //    - After identifier/number/closing paren → ExpectOperator → division
        //    - After operator/keyword/opening paren → ExpectTerm → regex
        //
        // 3. **Timeout Protection**:
        //    - Regex parsing has budget guard: MAX_REGEX_BYTES (64KB)
        //    - Budget exceeded → emit UnknownRest token (graceful degradation)
        //    - See `parse_regex()` and `budget_guard()` for implementation
        //
        // 4. **Performance Characteristics**:
        //    - Single-pass: O(1) decision based on mode flag
        //    - No backtracking: Mode updated after each token
        //    - Optimized: Byte-level operations for common cases
        //
        // **Metrics & Monitoring**:
        //    - Budget exceeded events tracked via UnknownRest token emission
        //    - LSP diagnostics generated for truncated regexes
        //    - Test coverage: lexer_slash_timeout_tests.rs (21 test cases)
        //
        // ═══════════════════════════════════════════════════════════════════════

        if ch == '/' {
            if self.mode == LexerMode::ExpectTerm {
                // Mode indicates we're expecting a term → `/` starts a regex
                // Examples: `if (/pattern/)`, `=~ /test/`, `while (/match/)`
                return self.parse_regex(start);
            } else {
                // Mode indicates we're expecting an operator → `/` is division or `//`
                // Examples: `$x / 2`, `$x // $y`, `10 / 3`
                self.advance();
                // Check for // or //= using byte-level operations for speed
                if self.position < self.input_bytes.len() && self.input_bytes[self.position] == b'/'
                {
                    self.position += 1; // consume second / directly
                    if self.position < self.input_bytes.len()
                        && self.input_bytes[self.position] == b'='
                    {
                        self.position += 1; // consume = directly
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectTerm;
                        return Some(Token {
                            token_type: TokenType::Operator(Arc::from(text)),
                            text: Arc::from(text),
                            start,
                            end: self.position,
                        });
                    } else {
                        // Use cached string for common "//" operator
                        self.mode = LexerMode::ExpectTerm;
                        return Some(Token {
                            token_type: TokenType::Operator(Arc::from("//")),
                            text: Arc::from("//"),
                            start,
                            end: self.position,
                        });
                    }
                } else {
                    // Use cached string for common "/" division
                    self.mode = LexerMode::ExpectTerm;
                    return Some(Token {
                        token_type: TokenType::Division,
                        text: Arc::from("/"),
                        start,
                        end: self.position,
                    });
                }
            }
        }

        // Handle other operators - simplified
        match ch {
            '.' => {
                // Check if it's a decimal number like .5
                if self.peek_char(1).is_some_and(|c| c.is_ascii_digit()) {
                    return self.parse_decimal_number(start);
                }
                self.advance();
                // Check for compound operators
                #[allow(clippy::collapsible_if)]
                if let Some(next) = self.current_char() {
                    if is_compound_operator(ch, next) {
                        self.advance();

                        // Check for three-character operators like **=, <<=, >>=
                        if self.position < self.input.len() {
                            let third = self.current_char();
                            // Check for three-character operators
                            if matches!(
                                (ch, next, third),
                                ('*', '*', Some('='))
                                    | ('<', '<', Some('='))
                                    | ('>', '>', Some('='))
                                    | ('&', '&', Some('='))
                                    | ('|', '|', Some('='))
                                    | ('/', '/', Some('='))
                            ) {
                                self.advance(); // consume the =
                            } else if ch == '<' && next == '=' && third == Some('>') {
                                self.advance(); // consume the >
                            // Special case: <=> spaceship operator
                            } else if ch == '.' && next == '.' && third == Some('.') {
                                self.advance(); // consume the third .
                            }
                        }
                    }
                }
            }
            '+' | '-' | '*' | '%' | '&' | '|' | '^' | '~' | '!' | '=' | '<' | '>' | ':' | '?'
            | '\\' => {
                self.advance();
                // Check for compound operators
                #[allow(clippy::collapsible_if)]
                if let Some(next) = self.current_char() {
                    if is_compound_operator(ch, next) {
                        self.advance();

                        // Check for three-character operators like **=, <<=, >>=
                        if self.position < self.input.len() {
                            let third = self.current_char();
                            // Check for three-character operators
                            if matches!(
                                (ch, next, third),
                                ('*', '*', Some('='))
                                    | ('<', '<', Some('='))
                                    | ('>', '>', Some('='))
                                    | ('&', '&', Some('='))
                                    | ('|', '|', Some('='))
                                    | ('/', '/', Some('='))
                            ) {
                                self.advance(); // consume the =
                            } else if ch == '<' && next == '=' && third == Some('>') {
                                self.advance(); // consume the >
                                // Special case: <=> spaceship operator
                            }
                        }
                    }
                }
            }
            _ => return None,
        }

        let text = &self.input[start..self.position];
        // Postfix ++ and -- complete a term expression, so next token is an operator
        // (e.g., "$x++ / 2" → / is division, not regex)
        if (text == "++" || text == "--") && self.mode == LexerMode::ExpectOperator {
            // Postfix: stay in ExpectOperator
        } else {
            self.mode = LexerMode::ExpectTerm;
        }

        Some(Token {
            token_type: TokenType::Operator(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    fn try_delimiter(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.current_char()?;

        // If we're expecting a delimiter for a quote operator, handle it specially
        if matches!(self.mode, LexerMode::ExpectDelimiter) && self.current_quote_op.is_some() {
            // Accept any non-alphanumeric character as a delimiter
            if !ch.is_alphanumeric() && !ch.is_whitespace() {
                self.advance();
                if let Some(ref mut info) = self.current_quote_op {
                    info.delimiter = ch;
                }
                // Now parse the quote operator content
                return self.parse_quote_operator(ch);
            }
        }

        match ch {
            '(' => {
                // Check if this is a quote operator delimiter
                if matches!(self.mode, LexerMode::ExpectDelimiter)
                    && self.current_quote_op.is_some()
                {
                    self.advance();
                    if let Some(ref mut info) = self.current_quote_op {
                        info.delimiter = ch;
                    }
                    return self.parse_quote_operator(ch);
                }

                self.advance();
                if self.in_prototype {
                    self.prototype_depth += 1;
                }
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::LeftParen,
                    text: Arc::from("("),
                    start,
                    end: self.position,
                })
            }
            ')' => {
                self.advance();
                if self.in_prototype && self.prototype_depth > 0 {
                    self.prototype_depth -= 1;
                    if self.prototype_depth == 0 {
                        self.in_prototype = false;
                    }
                }
                self.mode = LexerMode::ExpectOperator;
                Some(Token {
                    token_type: TokenType::RightParen,
                    text: Arc::from(")"),
                    start,
                    end: self.position,
                })
            }
            ';' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::Semicolon,
                    text: Arc::from(";"),
                    start,
                    end: self.position,
                })
            }
            ',' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::Comma,
                    text: Arc::from(","),
                    start,
                    end: self.position,
                })
            }
            '[' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::LeftBracket,
                    text: Arc::from("["),
                    start,
                    end: self.position,
                })
            }
            ']' => {
                self.advance();
                self.mode = LexerMode::ExpectOperator;
                Some(Token {
                    token_type: TokenType::RightBracket,
                    text: Arc::from("]"),
                    start,
                    end: self.position,
                })
            }
            '{' => {
                self.advance();
                self.mode = LexerMode::ExpectTerm;
                Some(Token {
                    token_type: TokenType::LeftBrace,
                    text: Arc::from("{"),
                    start,
                    end: self.position,
                })
            }
            '}' => {
                self.advance();
                self.mode = LexerMode::ExpectOperator;
                Some(Token {
                    token_type: TokenType::RightBrace,
                    text: Arc::from("}"),
                    start,
                    end: self.position,
                })
            }
            '#' => {
                // Only treat as delimiter in ExpectDelimiter mode
                if matches!(self.mode, LexerMode::ExpectDelimiter) {
                    self.advance();
                    // Reset mode after consuming delimiter
                    self.mode = LexerMode::ExpectTerm;
                    Some(Token {
                        token_type: TokenType::Operator(Arc::from("#")),
                        text: Arc::from("#"),
                        start,
                        end: self.position,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn parse_double_quoted_string(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening quote
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut last_pos = self.position;

        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    self.advance();
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal)));
                    }

                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;

                    return Some(Token {
                        token_type: if parts.is_empty() {
                            TokenType::StringLiteral
                        } else {
                            TokenType::InterpolatedString(parts)
                        },
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if let Some(escaped) = self.current_char() {
                        // Optimize by reserving space to avoid frequent reallocations
                        if current_literal.capacity() == 0 {
                            current_literal.reserve(32);
                        }
                        current_literal.push('\\');
                        current_literal.push(escaped);
                        self.advance();
                    }
                }
                '$' if self.config.parse_interpolation => {
                    // Handle variable interpolation - avoid unnecessary clone
                    if !current_literal.is_empty() {
                        parts.push(StringPart::Literal(Arc::from(current_literal)));
                        current_literal = String::new(); // Clear without cloning
                    }

                    // Parse variable - optimized using byte-level checks where possible
                    self.advance();
                    let var_start = self.position;

                    // Fast path for ASCII identifier continuation
                    while self.position < self.input_bytes.len() {
                        let byte = self.input_bytes[self.position];
                        if byte.is_ascii_alphanumeric() || byte == b'_' {
                            self.position += 1;
                        } else if byte >= 128 {
                            // Only use UTF-8 parsing for non-ASCII
                            if let Some(ch) = self.current_char() {
                                if is_perl_identifier_continue(ch) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if self.position > var_start {
                        let var_name = &self.input[var_start - 1..self.position];
                        parts.push(StringPart::Variable(Arc::from(var_name)));
                    }
                }
                _ => {
                    // Optimize string building with better capacity management
                    if current_literal.capacity() == 0 {
                        current_literal.reserve(32);
                    }
                    current_literal.push(ch);
                    self.advance();
                }
            }

            // Safety check: ensure we're making progress
            if self.position == last_pos {
                break;
            }
            last_pos = self.position;
        }

        // Unterminated string - return error token consuming rest of input
        let end = self.input.len();
        self.position = end;

        Some(Token {
            token_type: TokenType::Error(Arc::from("unterminated string")),
            text: Arc::from(&self.input[start..end]),
            start,
            end,
        })
    }

    fn parse_single_quoted_string(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening quote

        let mut last_pos = self.position;

        while let Some(ch) = self.current_char() {
            match ch {
                '\'' => {
                    self.advance();
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;

                    return Some(Token {
                        token_type: TokenType::StringLiteral,
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if self.current_char() == Some('\'') || self.current_char() == Some('\\') {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }

            // Safety check: ensure we're making progress
            if self.position == last_pos {
                break;
            }
            last_pos = self.position;
        }

        // Unterminated string - return error token consuming rest of input
        let end = self.input.len();
        self.position = end;

        Some(Token {
            token_type: TokenType::Error(Arc::from("unterminated string")),
            text: Arc::from(&self.input[start..end]),
            start,
            end,
        })
    }

    fn parse_backtick_string(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening backtick

        let mut last_pos = self.position;

        while let Some(ch) = self.current_char() {
            match ch {
                '`' => {
                    self.advance();
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;

                    return Some(Token {
                        token_type: TokenType::QuoteCommand,
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }

            // Safety check: ensure we're making progress
            if self.position == last_pos {
                break;
            }
            last_pos = self.position;
        }

        // Unterminated string - return error token consuming rest of input
        let end = self.input.len();
        self.position = end;

        Some(Token {
            token_type: TokenType::Error(Arc::from("unterminated string")),
            text: Arc::from(&self.input[start..end]),
            start,
            end,
        })
    }

    fn parse_q_string(&mut self, _start: usize) -> Option<Token> {
        // Simplified q-string parsing
        None
    }

    /// Returns the closing delimiter for paired delimiters, or the same character for non-paired.
    /// This helper makes delimiter pairing explicit and avoids unreachable code paths.
    fn paired_closing(delim: char) -> char {
        match delim {
            '{' => '}',
            '[' => ']',
            '(' => ')',
            '<' => '>',
            _ => delim, // non-paired delimiters use the same character
        }
    }

    fn parse_substitution(&mut self, start: usize) -> Option<Token> {
        // We've already consumed 's'
        let delimiter = self.current_char()?;
        self.advance(); // Skip delimiter

        // Parse pattern
        let mut depth = 1;
        let is_paired = matches!(delimiter, '{' | '[' | '(' | '<');
        let closing = Self::paired_closing(delimiter);

        while let Some(ch) = self.current_char() {
            // Check budget
            if let Some(token) = self.budget_guard(start, depth) {
                return Some(token);
            }

            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }

        // Parse replacement - may use different delimiter for paired patterns (e.g., s[foo]{bar})
        // MUT_002 fix: Detect the actual replacement delimiter instead of assuming same as pattern
        // Note: Pattern scanning is complete at this point; we use a separate repl_depth for replacement
        let (repl_delimiter, repl_closing, repl_is_paired) = if is_paired {
            // Skip whitespace between pattern and replacement for paired delimiters
            while let Some(ch) = self.current_char() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Detect replacement delimiter - may be different from pattern delimiter
            if let Some(repl_delim) = self.current_char() {
                if matches!(repl_delim, '{' | '[' | '(' | '<') {
                    let repl_close = Self::paired_closing(repl_delim);
                    self.advance();
                    (repl_delim, repl_close, true)
                } else {
                    // Non-paired replacement after paired pattern (unusual but valid)
                    self.advance();
                    (repl_delim, repl_delim, false)
                }
            } else {
                // End of input - return what we have
                (delimiter, closing, is_paired)
            }
        } else {
            // Non-paired delimiter - replacement uses same delimiter
            (delimiter, closing, false)
        };

        // Use separate depth counter for replacement to avoid confusion with pattern depth
        let mut repl_depth: usize = 1;
        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == repl_delimiter && repl_is_paired => {
                    repl_depth += 1;
                    self.advance();
                }
                _ if ch == repl_closing => {
                    self.advance();
                    if repl_is_paired {
                        repl_depth = repl_depth.saturating_sub(1);
                        if repl_depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }

        // Parse modifiers - include all alphanumeric for proper validation in parser (MUT_005 fix)
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_alphanumeric() {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;

        Some(Token {
            token_type: TokenType::Substitution,
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    fn parse_transliteration(&mut self, start: usize) -> Option<Token> {
        // We've already consumed 'tr' or 'y'
        let delimiter = self.current_char()?;
        self.advance(); // Skip delimiter

        // Parse search list
        let mut depth = 1;
        let is_paired = matches!(delimiter, '{' | '[' | '(' | '<');
        let closing = Self::paired_closing(delimiter);

        while let Some(ch) = self.current_char() {
            // Check budget
            if let Some(token) = self.budget_guard(start, depth) {
                return Some(token);
            }

            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }

        // Parse replacement list - same delimiter handling
        if is_paired {
            // Skip whitespace between search and replace for paired delimiters
            while let Some(ch) = self.current_char() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Expect opening delimiter for replacement
            if self.current_char() == Some(delimiter) {
                self.advance();
                depth = 1;
            }
        }

        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ if ch == delimiter && is_paired => {
                    depth += 1;
                    self.advance();
                }
                _ if ch == closing => {
                    self.advance();
                    if is_paired {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => self.advance(),
            }
        }

        // Parse modifiers - include all alphanumeric for proper validation in parser (MUT_005 fix)
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_alphanumeric() {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.input[start..self.position];
        self.mode = LexerMode::ExpectOperator;

        Some(Token {
            token_type: TokenType::Transliteration,
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    /// Read content between delimiters
    fn read_delimited_body(&mut self, delim: char) -> String {
        let paired = quote_handler::paired_close(delim);
        let close = paired.unwrap_or(delim);
        let mut body = String::new();
        let mut depth = i32::from(paired.is_some());

        while let Some(ch) = self.current_char() {
            if ch == '\\' {
                body.push(ch);
                self.advance();
                if let Some(next) = self.current_char() {
                    body.push(next);
                    self.advance();
                }
                continue;
            }

            if paired.is_some() && ch == delim {
                body.push(ch);
                self.advance();
                depth += 1;
                continue;
            }

            if ch == close {
                if paired.is_some() {
                    depth -= 1;
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                    body.push(ch);
                    self.advance();
                } else {
                    self.advance();
                    break;
                }
                continue;
            }

            body.push(ch);
            self.advance();
        }

        body
    }

    /// Parse a quote operator after we've seen the delimiter
    fn parse_quote_operator(&mut self, delimiter: char) -> Option<Token> {
        let info = self.current_quote_op.as_ref()?;
        let start = info.start_pos;
        let operator = info.operator.clone();

        // Parse based on operator type
        match operator.as_str() {
            "s" => {
                // Substitution: two bodies
                let _pattern = self.read_delimited_body(delimiter);

                // For paired delimiters, skip whitespace between bodies
                if quote_handler::paired_close(delimiter).is_some() {
                    while let Some(ch) = self.current_char() {
                        if ch.is_whitespace() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Expect same delimiter for replacement
                    if self.current_char() == Some(delimiter) {
                        self.advance();
                    }
                }

                let _replacement = self.read_delimited_body(delimiter);

                // Parse modifiers
                self.parse_regex_modifiers(&quote_handler::S_SPEC);
            }
            "tr" | "y" => {
                // Transliteration: two bodies
                let _from = self.read_delimited_body(delimiter);

                // For paired delimiters, skip whitespace between bodies
                if quote_handler::paired_close(delimiter).is_some() {
                    while let Some(ch) = self.current_char() {
                        if ch.is_whitespace() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Expect same delimiter for replacement
                    if self.current_char() == Some(delimiter) {
                        self.advance();
                    }
                }

                let _to = self.read_delimited_body(delimiter);

                // Parse modifiers
                self.parse_regex_modifiers(&quote_handler::TR_SPEC);
            }
            "qr" => {
                let _pattern = self.read_delimited_body(delimiter);
                self.parse_regex_modifiers(&quote_handler::QR_SPEC);
            }
            "m" => {
                let _pattern = self.read_delimited_body(delimiter);
                self.parse_regex_modifiers(&quote_handler::M_SPEC);
            }
            _ => {
                // q, qq, qw, qx - no modifiers
                let _body = self.read_delimited_body(delimiter);
            }
        }

        let text = &self.input[start..self.position];
        let token_type = quote_handler::get_quote_token_type(&operator);

        self.mode = LexerMode::ExpectOperator;
        self.current_quote_op = None;

        Some(Token { token_type, text: Arc::from(text), start, end: self.position })
    }

    /// Parse regex modifiers according to the given spec
    ///
    /// This function includes ALL characters that could be intended as modifiers,
    /// including invalid ones. This allows the parser to properly reject invalid
    /// modifiers with a clear error message, rather than leaving them as separate
    /// tokens that could be confusingly parsed.
    fn parse_regex_modifiers(&mut self, _spec: &quote_handler::ModSpec) {
        // Consume all alphanumeric characters that could be intended as modifiers
        // The parser will validate and reject invalid ones
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_alphanumeric() {
                self.advance();
            } else {
                break;
            }
        }
        // Note: We no longer validate here - the parser will validate and provide
        // clear error messages for invalid modifiers (MUT_005 fix)
    }

    /// Parse a regex literal starting with `/`
    ///
    /// **Timeout Protection (Issue #422)**:
    /// - Budget guard prevents infinite loops on pathological input
    /// - MAX_REGEX_BYTES limit (64KB) ensures bounded execution time
    /// - Graceful degradation: emit UnknownRest token if budget exceeded
    ///
    /// **Performance**:
    /// - Single-pass scanning with escape handling
    /// - Budget check per iteration (amortized O(1) via inline fast path)
    /// - Typical regex: <10μs, Large regex (64KB): ~1ms
    fn parse_regex(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening /

        while let Some(ch) = self.current_char() {
            // Budget guard: prevent timeout on pathological input (Issue #422)
            // If exceeded, returns UnknownRest token for graceful degradation
            if let Some(token) = self.budget_guard(start, 0) {
                return Some(token);
            }

            match ch {
                '/' => {
                    self.advance();
                    // Parse flags - include all alphanumeric for proper validation in parser (MUT_005 fix)
                    while let Some(ch) = self.current_char() {
                        if ch.is_ascii_alphanumeric() {
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;

                    return Some(Token {
                        token_type: TokenType::RegexMatch,
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                '\\' => {
                    // Handle escape sequences: consume backslash + next char
                    self.advance();
                    if self.current_char().is_some() {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }
        }

        // Unterminated regex - EOF reached before closing /
        // Parser will emit diagnostic for unterminated literal
        None
    }
}

// Pre-computed keyword hash for fast lookup
static KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();

// Pre-allocated empty Arc to avoid repeated allocations
static EMPTY_ARC: OnceLock<Arc<str>> = OnceLock::new();

#[inline(always)]
fn empty_arc() -> Arc<str> {
    EMPTY_ARC.get_or_init(|| Arc::from("")).clone()
}

#[inline(always)]
fn is_keyword(word: &str) -> bool {
    let keywords = KEYWORDS.get_or_init(|| {
        [
            // Single char keywords
            "q",
            "m",
            "s",
            "y",
            // Two char keywords
            "if",
            "do",
            "my",
            "or",
            "qq",
            "qw",
            "qr",
            "qx",
            "tr",
            // Three char keywords
            "sub",
            "our",
            "use",
            "and",
            "not",
            "xor",
            "die",
            "say",
            "for",
            "try",
            "END",
            "cmp",
            // Four char keywords
            "else",
            "when",
            "next",
            "last",
            "redo",
            "goto",
            "eval",
            "warn",
            "INIT",
            // Five char keywords
            "elsif",
            "while",
            "until",
            "local",
            "state",
            "given",
            "break",
            "print",
            "catch",
            "BEGIN",
            "CHECK",
            "class",
            "undef",
            // Six char keywords
            "unless",
            "return",
            "method",
            "format",
            // Seven char keywords
            "require",
            "package",
            "default",
            "foreach",
            "finally",
            // Eight char keywords
            "continue",
            // Nine char keywords
            "UNITCHECK",
        ]
        .into_iter()
        .collect()
    });

    // Fast length-based rejection for most cases
    match word.len() {
        1..=9 => keywords.contains(word),
        _ => false,
    }
}

/// Fast lookup table for compound operator second characters
const COMPOUND_SECOND_CHARS: &[u8] = b"=<>&|+->.~*";

#[inline]
fn is_compound_operator(first: char, second: char) -> bool {
    // Optimized compound operator lookup using perfect hashing for common cases
    // Convert to bytes for faster comparison (most operators are ASCII)
    if first.is_ascii() && second.is_ascii() {
        let first_byte = first as u8;
        let second_byte = second as u8;

        if !COMPOUND_SECOND_CHARS.contains(&second_byte) {
            return false;
        }

        // Use lookup table approach for maximum performance
        match (first_byte, second_byte) {
            // Assignment operators
            (b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'.', b'=') => true,

            // Comparison operators
            (b'<' | b'>' | b'=' | b'!', b'=') => true,

            // Pattern operators
            (b'=' | b'!', b'~') => true,

            // Increment/decrement
            (b'+', b'+') | (b'-', b'-') => true,

            // Logical operators
            (b'&', b'&') | (b'|', b'|') => true,

            // Shift operators
            (b'<', b'<') | (b'>', b'>') => true,

            // Other compound operators
            (b'*', b'*')
            | (b'/', b'/')
            | (b'-' | b'=', b'>')
            | (b'.', b'.')
            | (b'~', b'~')
            | (b':', b':') => true,

            _ => false,
        }
    } else {
        // Fallback for non-ASCII (should be rare)
        matches!(
            (first, second),
            ('+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '.' | '<' | '>' | '=' | '!', '=')
                | ('=' | '!' | '~', '~')
                | ('+', '+')
                | ('-', '-' | '>')
                | ('&', '&')
                | ('|', '|')
                | ('<', '<')
                | ('>' | '=', '>')
                | ('*', '*')
                | ('/', '/')
                | ('.', '.')
                | (':', ':')
        )
    }
}

// Checkpoint support for incremental parsing
impl Checkpointable for PerlLexer<'_> {
    fn checkpoint(&self) -> LexerCheckpoint {
        use checkpoint::CheckpointContext;

        // Determine the checkpoint context based on current state
        let context = if matches!(self.mode, LexerMode::InFormatBody) {
            CheckpointContext::Format {
                start_position: self.position.saturating_sub(100), // Approximate
            }
        } else if !self.delimiter_stack.is_empty() {
            // We're in some kind of quote-like construct
            CheckpointContext::QuoteLike {
                operator: String::new(), // Would need to track this
                delimiter: self.delimiter_stack.last().copied().unwrap_or('\0'),
                is_paired: true,
            }
        } else {
            CheckpointContext::Normal
        };

        LexerCheckpoint {
            position: self.position,
            mode: self.mode,
            delimiter_stack: self.delimiter_stack.clone(),
            in_prototype: self.in_prototype,
            prototype_depth: self.prototype_depth,
            current_pos: self.current_pos,
            context,
        }
    }

    fn restore(&mut self, checkpoint: &LexerCheckpoint) {
        self.position = checkpoint.position;
        self.mode = checkpoint.mode;
        self.delimiter_stack.clone_from(&checkpoint.delimiter_stack);
        self.in_prototype = checkpoint.in_prototype;
        self.prototype_depth = checkpoint.prototype_depth;
        self.current_pos = checkpoint.current_pos;

        // Handle special contexts
        use checkpoint::CheckpointContext;
        if let CheckpointContext::Format { .. } = &checkpoint.context {
            // Ensure we're in format body mode
            if !matches!(self.mode, LexerMode::InFormatBody) {
                self.mode = LexerMode::InFormatBody;
            }
        }
    }

    fn can_restore(&self, checkpoint: &LexerCheckpoint) -> bool {
        // Can restore if the position is valid for our input
        checkpoint.position <= self.input.len()
    }
}

#[cfg(test)]
mod test_format_debug;

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_basic_tokens() -> TestResult {
        let mut lexer = PerlLexer::new("my $x = 42;");

        let token = lexer.next_token().ok_or("Expected keyword token")?;
        assert_eq!(token.token_type, TokenType::Keyword(Arc::from("my")));

        let token = lexer.next_token().ok_or("Expected identifier token")?;
        assert!(matches!(token.token_type, TokenType::Identifier(_)));

        let token = lexer.next_token().ok_or("Expected operator token")?;
        assert!(matches!(token.token_type, TokenType::Operator(_)));

        let token = lexer.next_token().ok_or("Expected number token")?;
        assert!(matches!(token.token_type, TokenType::Number(_)));

        let token = lexer.next_token().ok_or("Expected semicolon token")?;
        assert_eq!(token.token_type, TokenType::Semicolon);
        Ok(())
    }

    #[test]
    fn test_slash_disambiguation() -> TestResult {
        // Division
        let mut lexer = PerlLexer::new("10 / 2");
        lexer.next_token(); // 10
        let token = lexer.next_token().ok_or("Expected division token")?;
        assert_eq!(token.token_type, TokenType::Division);

        // Regex
        let mut lexer = PerlLexer::new("if (/pattern/)");
        lexer.next_token(); // if
        lexer.next_token(); // (
        let token = lexer.next_token().ok_or("Expected regex token")?;
        assert_eq!(token.token_type, TokenType::RegexMatch);
        Ok(())
    }

    #[test]
    fn test_percent_and_double_sigil_disambiguation() -> TestResult {
        // Hash variable
        let mut lexer = PerlLexer::new("%hash");
        let token = lexer.next_token().ok_or("Expected hash identifier token")?;
        assert!(
            matches!(token.token_type, TokenType::Identifier(ref id) if id.as_ref() == "%hash")
        );

        // Modulo operator
        let mut lexer = PerlLexer::new("10 % 3");
        lexer.next_token(); // 10
        let token = lexer.next_token().ok_or("Expected modulo operator token")?;
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "%"));
        Ok(())
    }

    #[test]
    fn test_defined_or_and_exponent() -> TestResult {
        // Defined-or operator
        let mut lexer = PerlLexer::new("$a // $b");
        lexer.next_token(); // $a
        let token = lexer.next_token().ok_or("Expected defined-or operator token")?;
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "//"));

        // Regex after =~ should still parse
        let mut lexer = PerlLexer::new("$x =~ //");
        lexer.next_token(); // $x
        lexer.next_token(); // =~
        let token = lexer.next_token().ok_or("Expected regex token")?;
        assert_eq!(token.token_type, TokenType::RegexMatch);

        // Exponent operator
        let mut lexer = PerlLexer::new("2 ** 3");
        lexer.next_token(); // 2
        let token = lexer.next_token().ok_or("Expected exponent operator token")?;
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "**"));
        Ok(())
    }
}
