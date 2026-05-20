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
//! ```rust
//! use perl_lexer::{PerlLexer, TokenType};
//!
//! let mut lexer = PerlLexer::new("my $x = 42;");
//! let tokens = lexer.collect_tokens();
//!
//! // First token is the keyword `my`
//! assert!(matches!(&tokens[0].token_type, TokenType::Keyword(k) if &**k == "my"));
//! // Tokens include variables, operators, literals, and EOF
//! assert!(matches!(&tokens.last().map(|t| &t.token_type), Some(TokenType::EOF)));
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
//! - **MAX_REGEX_PARSE_STEPS**: 32K maximum scan iterations for regex literals
//!
//! When limits are exceeded, the lexer emits an `UnknownRest` token preserving
//! all previously parsed symbols, allowing continued analysis.
//!
//! # Integration with perl-parser
//!
//! The lexer is designed to work seamlessly with `perl_parser_core::Parser`.
//! You rarely need to use the lexer directly -- the parser creates and manages
//! a `PerlLexer` instance internally:
//!
//! ```rust,ignore
//! use perl_parser_core::Parser;
//!
//! let code = r#"sub hello { print "Hello, world!\n"; }"#;
//! let mut parser = Parser::new(code);
//! let ast = parser.parse().expect("should parse");
//! ```

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

use std::sync::Arc;

pub mod api;
pub mod builtins;
pub mod checkpoint;
pub mod config;
pub mod error;
mod heredoc;
pub mod keywords;
mod lexer;
pub mod limits;
pub mod mode;
mod quote_handler;
pub mod token;
pub mod tokenizer;
mod unicode;

pub use api::*;
pub use checkpoint::{CheckpointCache, Checkpointable, LexerCheckpoint};
pub use config::LexerConfig;
pub use error::{LexerError, Result};
pub use lexer::PerlLexer;
pub use limits::MAX_REGEX_PARSE_STEPS;
pub use mode::LexerMode;
pub use perl_position_tracking::Position;
pub use token::{StringPart, Token, TokenType};

use unicode::{is_perl_identifier_continue, is_perl_identifier_start};

use crate::heredoc::HeredocSpec;
use crate::lexer::helpers::{
    empty_arc, is_builtin_function, is_compound_operator, is_keyword_fast, is_quote_op_word_prefix,
    truncate_preview,
};
use crate::limits::{
    HEREDOC_TIMEOUT_MS, MAX_DELIM_NEST, MAX_HEREDOC_BYTES, MAX_HEREDOC_DEPTH, MAX_REGEX_BYTES,
};

impl<'a> PerlLexer<'a> {
    /// Create a new lexer that emits `HeredocBody` tokens (for LSP folding)
    pub fn with_body_tokens(input: &'a str) -> Self {
        let mut lexer = Self::new(input);
        lexer.emit_heredoc_body_tokens = true;
        lexer
    }

    /// Set the lexer mode (for resetting state at statement boundaries)
    pub fn set_mode(&mut self, mode: LexerMode) {
        self.mode = mode;
    }

    /// Advance the lexer and return the next token.
    ///
    /// Returns `None` only after an `EOF` token has already been emitted.
    /// The final meaningful call returns `Some(Token { token_type: TokenType::EOF, .. })`.
    pub fn next_token(&mut self) -> Option<Token> {
        // Normalize file start (BOM) once
        if self.position == 0 {
            self.normalize_file_start();
        }
        self.normalize_char_boundary();

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
                        let line_end = Self::find_line_end(self.input_bytes, self.position);
                        let line = &self.input[line_start..line_end];
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

            if let Some(token) = self.try_vstring() {
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
                // Unicode path without intermediate heap allocation
                let mut buf = [0_u8; 4];
                Arc::from(ch.encode_utf8(&mut buf))
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
        #[cfg(debug_assertions)]
        {
            tracing::debug!(
                bytes_consumed,
                depth,
                position = self.position,
                "Lexer budget exceeded"
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

    /// Peek at the next token without consuming it.
    ///
    /// Saves and restores the full lexer state so the next call to
    /// [`next_token`](Self::next_token) returns the same token.
    pub fn peek_token(&mut self) -> Option<Token> {
        let saved_pos = self.position;
        let saved_mode = self.mode;
        let saved_delimiter_stack = self.delimiter_stack.clone();
        let saved_prototype = self.in_prototype;
        let saved_depth = self.prototype_depth;
        let saved_after_sub = self.after_sub;
        let saved_after_arrow = self.after_arrow;
        let saved_hash_brace_depth = self.hash_brace_depth;
        let saved_after_var_subscript = self.after_var_subscript;
        let saved_paren_depth = self.paren_depth;
        let saved_current_pos = self.current_pos;
        let saved_after_newline = self.after_newline;
        let saved_pending_heredocs = self.pending_heredocs.clone();
        let saved_line_start_offset = self.line_start_offset;
        let saved_current_quote_op = self.current_quote_op.clone();
        let saved_eof_emitted = self.eof_emitted;
        let saved_start_time = self.start_time;

        let token = self.next_token();

        self.position = saved_pos;
        self.mode = saved_mode;
        self.delimiter_stack = saved_delimiter_stack;
        self.in_prototype = saved_prototype;
        self.prototype_depth = saved_depth;
        self.after_sub = saved_after_sub;
        self.after_arrow = saved_after_arrow;
        self.hash_brace_depth = saved_hash_brace_depth;
        self.after_var_subscript = saved_after_var_subscript;
        self.paren_depth = saved_paren_depth;
        self.current_pos = saved_current_pos;
        self.after_newline = saved_after_newline;
        self.pending_heredocs = saved_pending_heredocs;
        self.line_start_offset = saved_line_start_offset;
        self.current_quote_op = saved_current_quote_op;
        self.eof_emitted = saved_eof_emitted;
        self.start_time = saved_start_time;

        token
    }

    /// Consume all remaining tokens and return them as a vector.
    ///
    /// The returned vector always ends with an `EOF` token.
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

    /// Reset the lexer to the beginning of the input.
    ///
    /// Clears all internal state (mode, delimiter stack, heredoc queue, etc.)
    /// so the lexer can re-tokenize the same source from scratch.
    pub fn reset(&mut self) {
        self.position = 0;
        self.mode = LexerMode::ExpectTerm;
        self.delimiter_stack.clear();
        self.in_prototype = false;
        self.prototype_depth = 0;
        self.after_sub = false;
        self.after_arrow = false;
        self.hash_brace_depth = 0;
        self.after_var_subscript = false;
        self.paren_depth = 0;
        self.current_pos = Position::start();
        self.after_newline = true;
        self.pending_heredocs.clear();
        self.line_start_offset = 0;
        self.current_quote_op = None;
        self.eof_emitted = false;
        self.start_time = std::time::Instant::now();
    }

    /// Switch the lexer into format-body parsing mode.
    ///
    /// In this mode the lexer consumes input verbatim until it encounters a
    /// line containing only `.` (the Perl format terminator).
    pub fn enter_format_mode(&mut self) {
        self.mode = LexerMode::InFormatBody;
    }

    // Token-specific parsing methods

    #[inline]
    fn skip_whitespace_and_comments(&mut self) -> Option<()> {
        // Don't reset after_newline if we're at the start of a line
        if self.position > 0 && self.position != self.line_start_offset {
            self.after_newline = false;
        }

        while self.position < self.input_bytes.len() {
            let byte = Self::byte_at(self.input_bytes, self.position);
            match byte {
                // Fast path for ASCII whitespace - batch process
                b' ' => {
                    // Batch skip spaces for better cache efficiency
                    let start = self.position;
                    while self.position < self.input_bytes.len()
                        && Self::byte_at(self.input_bytes, self.position) == b' '
                    {
                        self.position += 1;
                    }
                    // Continue outer loop if we processed any spaces
                    if self.position > start {
                        // Loop naturally continues to next iteration
                    }
                }
                b'\t' | 0x0B | 0x0C => {
                    // Batch skip horizontal tab, vertical tab, and form feed.
                    // Perl treats these as whitespace separators.
                    let start = self.position;
                    while self.position < self.input_bytes.len()
                        && matches!(
                            Self::byte_at(self.input_bytes, self.position),
                            b'\t' | 0x0B | 0x0C
                        )
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

                    // Use memchr2 to find CR/LF line endings quickly (supports LF, CRLF, and CR)
                    if let Some(newline_offset) =
                        memchr::memchr2(b'\n', b'\r', &self.input_bytes[self.position..])
                    {
                        self.position += newline_offset;
                    } else {
                        // No newline found, skip to end
                        self.position = self.input_bytes.len();
                    }
                }
                b'=' if self.position == 0
                    || (self.position > 0
                        && matches!(self.input_bytes[self.position - 1], b'\n' | b'\r')) =>
                {
                    // Check if this starts a POD section (=pod, =head, =over, etc.)
                    // Use byte-safe checks — avoid slicing &str at arbitrary byte positions
                    let remaining = &self.input_bytes[self.position..];
                    if remaining.starts_with(b"=pod")
                        || remaining.starts_with(b"=head")
                        || remaining.starts_with(b"=over")
                        || remaining.starts_with(b"=item")
                        || remaining.starts_with(b"=back")
                        || remaining.starts_with(b"=begin")
                        || remaining.starts_with(b"=end")
                        || remaining.starts_with(b"=for")
                        || remaining.starts_with(b"=encoding")
                    {
                        // Scan forward for \n=cut (end of POD block)
                        let search_start = self.position;
                        let mut found_cut = false;
                        let bytes = self.input_bytes;
                        let mut i = search_start;
                        while i < bytes.len() {
                            // Look for =cut at the start of a line
                            if (i == 0 || matches!(bytes[i - 1], b'\n' | b'\r'))
                                && bytes[i..].starts_with(b"=cut")
                            {
                                i += 4; // Skip "=cut"
                                // Skip rest of the =cut line
                                while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                                    i += 1;
                                }
                                // Consume one line ending sequence if present
                                if i < bytes.len() && bytes[i] == b'\r' {
                                    i += 1;
                                    if i < bytes.len() && bytes[i] == b'\n' {
                                        i += 1;
                                    }
                                } else if i < bytes.len() && bytes[i] == b'\n' {
                                    i += 1;
                                }
                                self.position = i;
                                found_cut = true;
                                break;
                            }
                            i += 1;
                        }
                        if !found_cut {
                            // POD extends to end of file
                            self.position = bytes.len();
                        }
                        continue;
                    }
                    // Not a POD directive - regular '=' token
                    break;
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
        // `<<` is the left-shift operator, not a heredoc, when we are inside
        // a parenthesized expression and have just finished a term.
        // E.g. `(1<<index(...))` — the `1` sets ExpectOperator and paren_depth > 0,
        // so `<<index` must be the bitshift operator, not a heredoc start.
        //
        // We must NOT fire the guard at statement level (paren_depth == 0) because
        // `print $fh <<END` is valid Perl: `$fh` sets ExpectOperator but `<<END`
        // is a heredoc.  The depth check distinguishes the two cases.
        if self.mode == LexerMode::ExpectOperator && self.paren_depth > 0 {
            return None;
        }

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
                Some('"') if !backslashed => self.parse_quoted_heredoc_delimiter('"', &mut text)?,
                Some('\'') if !backslashed => {
                    self.parse_quoted_heredoc_delimiter('\'', &mut text)?
                }
                Some('`') if !backslashed => self.parse_quoted_heredoc_delimiter('`', &mut text)?,
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
        if self.position >= bytes.len() || !Self::byte_at(bytes, self.position).is_ascii_digit() {
            return None;
        }

        // Check for hex (0x), binary (0b), or octal (0o) prefixes
        let mut pos = self.position;
        if Self::byte_at(bytes, pos) == b'0' && pos + 1 < bytes.len() {
            let prefix_byte = bytes[pos + 1];
            if prefix_byte == b'x' || prefix_byte == b'X' {
                // Hexadecimal: 0x[0-9a-fA-F_]+
                pos += 2; // consume '0x'
                let digit_start = pos;
                let mut saw_digit = false;
                while pos < bytes.len() && (bytes[pos].is_ascii_hexdigit() || bytes[pos] == b'_') {
                    saw_digit |= bytes[pos].is_ascii_hexdigit();
                    pos += 1;
                }
                if pos > digit_start && saw_digit {
                    self.position = pos;
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    return Some(Token {
                        token_type: TokenType::Number(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                // No hex digits after 0x - fall through to parse '0' as decimal
            } else if prefix_byte == b'b' || prefix_byte == b'B' {
                // Binary: 0b[01_]+
                pos += 2; // consume '0b'
                let digit_start = pos;
                let mut saw_digit = false;
                while pos < bytes.len()
                    && (bytes[pos] == b'0' || bytes[pos] == b'1' || bytes[pos] == b'_')
                {
                    saw_digit |= bytes[pos] == b'0' || bytes[pos] == b'1';
                    pos += 1;
                }
                if pos > digit_start && saw_digit {
                    self.position = pos;
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    return Some(Token {
                        token_type: TokenType::Number(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                // No binary digits after 0b - fall through to parse '0' as decimal
            } else if prefix_byte == b'o' || prefix_byte == b'O' {
                // Octal (explicit): 0o[0-7_]+
                pos += 2; // consume '0o'
                let digit_start = pos;
                let mut saw_digit = false;
                while pos < bytes.len()
                    && ((bytes[pos] >= b'0' && bytes[pos] <= b'7') || bytes[pos] == b'_')
                {
                    saw_digit |= (b'0'..=b'7').contains(&bytes[pos]);
                    pos += 1;
                }
                if pos > digit_start && saw_digit {
                    self.position = pos;
                    let text = &self.input[start..self.position];
                    self.mode = LexerMode::ExpectOperator;
                    return Some(Token {
                        token_type: TokenType::Number(Arc::from(text)),
                        text: Arc::from(text),
                        start,
                        end: self.position,
                    });
                }
                // No octal digits after 0o - fall through to parse '0' as decimal
            }
        }

        // Consume initial digits - unrolled for better performance
        pos = self.position;
        while pos < bytes.len() {
            let byte = Self::byte_at(bytes, pos);
            if byte.is_ascii_digit() || byte == b'_' {
                pos += 1;
            } else {
                break;
            }
        }
        self.position = pos;

        // Check for decimal point - optimized with single bounds check
        if pos < bytes.len() && Self::byte_at(bytes, pos) == b'.' {
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

            // Must have at least one digit after exponent (underscores allowed between digits)
            let mut saw_digit = false;
            while pos < bytes.len() {
                let byte = bytes[pos];
                if byte.is_ascii_digit() {
                    saw_digit = true;
                    pos += 1;
                } else if byte == b'_' {
                    pos += 1;
                } else {
                    break;
                }
            }

            // If no digits after exponent, backtrack
            if !saw_digit {
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
                    // Handle scientific notation.
                    // Save the position of 'e'/'E' so we can backtrack here if
                    // no digits follow the exponent marker (with or without sign).
                    let e_pos = self.position;
                    self.advance();
                    if self.position < self.input_bytes.len() {
                        let next = self.input_bytes[self.position];
                        if next == b'+' || next == b'-' {
                            self.advance();
                        }
                    }
                    // Parse exponent digits (underscores allowed between digits)
                    let mut saw_digit = false;
                    while self.position < self.input_bytes.len() {
                        let byte = self.input_bytes[self.position];
                        if byte.is_ascii_digit() {
                            saw_digit = true;
                            self.position += 1;
                        } else if byte == b'_' {
                            self.position += 1;
                        } else {
                            break;
                        }
                    }

                    // No digits after exponent marker — backtrack to just before
                    // 'e'/'E' so the caller sees it as a separate token.
                    // Using e_pos (not exponent_start-1) avoids including 'e' in
                    // the number slice when a sign character was consumed.
                    if !saw_digit {
                        self.position = e_pos;
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
                    // $#foo is a complete variable token; a following `{` is a subscript.
                    self.after_var_subscript = true;

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
                    // If the next char suggests dereference, don't consume the brace.
                    // For @ and % sigils, identifiers inside braces are also derefs
                    // (e.g. @{Foo::Bar::baz} or %{Some::Hash}).
                    let is_deref = sigil != '*'
                        && (matches!(
                            next_char,
                            Some('$' | '@' | '%' | '*' | '&' | '[' | ' ' | '\t' | '\n' | '\r',)
                        ) || (matches!(sigil, '@' | '%')
                            && next_char.is_some_and(is_perl_identifier_start)));
                    if is_deref {
                        // This is a dereference, don't consume the brace
                        let text = &self.input[start..self.position];
                        self.mode = LexerMode::ExpectOperator;
                        // A standalone sigil token before `{` starts a dereference
                        // sequence (e.g. `${$ref}` / `@{$aref}` / `%{$href}` / `&{$cref}`).
                        // Mark it as subscript-capable so `{` increments brace depth
                        // and the closing `}` can enable chained `{...}` subscripts.
                        // (Broader form than master's `$|@|%` filter — `*` is already
                        // excluded by the `is_deref` guard above and `&` deref also
                        // benefits from chained-subscript handling.)
                        self.after_var_subscript = true;

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
                            && !self.current_char().is_some_and(is_perl_identifier_start)
                        {
                            // This is a dereference or empty/invalid brace, backtrack
                            self.position = start + 1; // Just past the sigil
                            let text = &self.input[start..self.position];
                            self.mode = LexerMode::ExpectOperator;
                            // Same as above: sigil-only token means a dereference opener.
                            self.after_var_subscript = true;

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
                    // Handle $^Letter (e.g. $^W, $^O, $^X) and bare $^ (format_top_name)
                    // Not inside prototypes where ^ is a literal prototype char
                    else if sigil == '$' && ch == '^' && !self.in_prototype {
                        self.advance(); // consume ^
                        // $^Letter: consume the single uppercase letter
                        if let Some(letter) = self.current_char()
                            && letter.is_ascii_uppercase()
                        {
                            self.advance();
                        }
                        // bare $^ (no uppercase letter follows): format_top_name — stop here
                    }
                    // Handle special punctuation variables
                    // Not inside prototypes where ; and , are literal prototype chars
                    else if sigil == '$'
                        && !self.in_prototype
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
                                | '~'
                                | '='
                                | '%'
                                | ','
                                | '"'
                                | ';'
                                | '>'
                                | '<'
                                | ')'
                                | '(' // $( = real group ID of this process
                        )
                    {
                        self.advance(); // consume the special character
                    }
                    // $$ is the PID special variable, but only when it is not immediately
                    // followed by an identifier-start character. $$var is scalar dereference
                    // of $var, so keep the second $ for the next token.
                    else if sigil == '$' && ch == '$' {
                        if !self.peek_char(1).is_some_and(is_perl_identifier_start) {
                            self.advance(); // consume the second $ for bare $$ PID
                        }
                    }
                    // Handle special array/hash punctuation variables
                    else if (sigil == '@' || sigil == '%') && matches!(ch, '+' | '-') {
                        self.advance(); // consume the + or -
                    }
                }

                let text = &self.input[start..self.position];
                self.mode = LexerMode::ExpectOperator;
                // A complete $foo, @foo, %foo token can be followed by a hash/slice
                // subscript `{`. Set the flag so the `{` handler knows to increment
                // hash_brace_depth. Glob tokens (*foo) are excluded: they don't take
                // hash subscripts in the same way.
                self.after_var_subscript = matches!(sigil, '$' | '@' | '%');

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

    /// Return the next non-space char and the char immediately following it (without consuming).
    /// Used to detect quote-operator delimiters while distinguishing `=>` (fat-arrow autoquote)
    /// from `=` used as a plain delimiter.
    fn peek_nonspace_and_following(&self) -> (Option<char>, Option<char>) {
        let mut i = self.position;
        while i < self.input.len() {
            let c = match self.input.get(i..).and_then(|s| s.chars().next()) {
                Some(c) => c,
                None => return (None, None),
            };
            if c.is_whitespace() {
                i += c.len_utf8();
                continue;
            }
            // Found non-space at position i; peek the next char after it
            let j = i + c.len_utf8();
            let following = self.input.get(j..).and_then(|s| s.chars().next());
            return (Some(c), following);
        }
        (None, None)
    }

    /// Is `c` a valid quote-like delimiter? (non-alnum, including paired)
    fn is_quote_delim(c: char) -> bool {
        // Perl allows any non-alphanumeric, non-whitespace character as delimiter,
        // including control characters (e.g. s\x07pattern\x07replacement\x07).
        !c.is_ascii_alphanumeric() && !c.is_whitespace()
    }

    #[inline]
    fn immediately_follows_sigil_prefix(&self, start: usize) -> bool {
        start > 0
            && matches!(
                Self::byte_at(self.input_bytes, start.saturating_sub(1)),
                b'$' | b'@' | b'%' | b'&' | b'*'
            )
    }

    /// Try to parse a v-string (version string) like `v5.26.0` or `v5.10`.
    ///
    /// A v-string starts with `v` followed by one or more digits, then optionally
    /// `.` followed by digits, repeated. The `v` prefix distinguishes these from
    /// normal identifiers. Examples: `v5.26.0`, `v5.10`, `v1.2.3.4`.
    #[inline]
    fn try_vstring(&mut self) -> Option<Token> {
        let start = self.position;
        let bytes = self.input_bytes;

        // Must start with 'v' followed by at least one digit
        if start >= bytes.len() || bytes[start] != b'v' {
            return None;
        }

        let next_pos = start + 1;
        if next_pos >= bytes.len() || !bytes[next_pos].is_ascii_digit() {
            return None;
        }

        // We have `v` followed by a digit — scan the rest of the v-string.
        // Pattern: v DIGITS (.DIGITS)*
        let mut pos = next_pos;

        // Consume leading digits
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }

        // Consume optional `.DIGITS` segments (require at least one digit after dot)
        while pos < bytes.len() && bytes[pos] == b'.' {
            let dot_pos = pos;
            pos += 1; // skip '.'

            if pos >= bytes.len() || !bytes[pos].is_ascii_digit() {
                // Dot not followed by digit — not part of the v-string
                pos = dot_pos;
                break;
            }

            // Consume digits after the dot
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
        }

        // Make sure the v-string isn't followed by identifier-continuation characters
        // (e.g. `v5x` should remain an identifier, not a v-string `v5` + `x`)
        if pos < bytes.len() {
            let next_byte = bytes[pos];
            if next_byte == b'_' || next_byte.is_ascii_alphabetic() {
                return None;
            }
            // Also check for non-ASCII identifier continuations
            if next_byte >= 128
                && let Some(ch) = self.input.get(pos..).and_then(|s| s.chars().next())
                && is_perl_identifier_continue(ch)
            {
                return None;
            }
        }

        // `v5` (no dots) is a valid Perl v-string meaning chr(5).
        let text = &self.input[start..pos];

        self.position = pos;
        self.mode = LexerMode::ExpectOperator;

        Some(Token {
            token_type: TokenType::Version(Arc::from(text)),
            text: Arc::from(text),
            start,
            end: self.position,
        })
    }

    #[inline]
    fn apostrophe_starts_legacy_package_segment(&self, position: usize) -> bool {
        let next_position = position + '\''.len_utf8();
        self.input
            .get(next_position..)
            .and_then(|suffix| suffix.chars().next())
            .is_some_and(is_perl_identifier_start)
    }

    #[inline]
    fn try_identifier_or_keyword(&mut self) -> Option<Token> {
        let start = self.position;
        let ch = self.current_char()?;
        let bytes = self.input_bytes;
        let len = bytes.len();

        if is_perl_identifier_start(ch) {
            // Special case: substitution/transliteration with single-quote delimiter
            // The single quote is considered an identifier continuation, so we need to
            // detect these operators before consuming it as part of an identifier.
            let follows_sigil_prefix = self.immediately_follows_sigil_prefix(start);
            if !follows_sigil_prefix
                && !self.after_arrow
                && self.hash_brace_depth == 0
                && ch == 's'
                && self.peek_char(1) == Some('\'')
            {
                self.advance(); // consume 's'
                return self.parse_substitution(start);
            } else if !follows_sigil_prefix
                && !self.after_arrow
                && self.hash_brace_depth == 0
                && ch == 'y'
                && self.peek_char(1) == Some('\'')
            {
                self.advance(); // consume 'y'
                return self.parse_transliteration(start);
            } else if !follows_sigil_prefix
                && !self.after_arrow
                && self.hash_brace_depth == 0
                && ch == 't'
                && self.peek_char(1) == Some('r')
                && self.peek_char(2) == Some('\'')
            {
                self.advance(); // consume 't'
                self.advance(); // consume 'r'
                return self.parse_transliteration(start);
            }

            // Fast ASCII path for identifier continuation.
            while self.position < len {
                let byte = bytes[self.position];
                if byte == b'\'' {
                    if is_quote_op_word_prefix(&bytes[start..self.position])
                        || !self.apostrophe_starts_legacy_package_segment(self.position)
                    {
                        // Keep apostrophe for quote/string parsing in cases like q'...'
                        // and split' ', while still accepting Foo'Bar package spelling.
                        break;
                    }
                    self.position += 1;
                    continue;
                }

                if byte.is_ascii_alphanumeric() || byte == b'_' {
                    self.position += 1;
                    continue;
                }

                if byte < 128 {
                    break;
                }

                if let Some(ch) = self.current_char()
                    && is_perl_identifier_continue(ch)
                {
                    self.advance();
                    continue;
                }
                break;
            }
            // Handle package-qualified identifiers like Foo::bar.
            while self.config.max_lookahead >= 1
                && self.position + 1 < len
                && bytes[self.position] == b':'
                && bytes[self.position + 1] == b':'
            {
                self.position += 2; // consume '::'

                // consume following identifier segment if present
                let Some(ch) = self.current_char() else {
                    break;
                };
                if !is_perl_identifier_start(ch) {
                    break;
                }
                self.advance();
                while self.position < len {
                    let byte = bytes[self.position];
                    if byte == b'\'' {
                        if !self.apostrophe_starts_legacy_package_segment(self.position) {
                            break;
                        }
                        self.position += 1;
                        continue;
                    }

                    if byte.is_ascii_alphanumeric() || byte == b'_' {
                        self.position += 1;
                        continue;
                    }
                    if byte < 128 {
                        break;
                    }
                    if let Some(ch) = self.current_char()
                        && is_perl_identifier_continue(ch)
                    {
                        self.advance();
                        continue;
                    }
                    break;
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
                            && self.input_bytes[self.position] != b'\r'
                        {
                            self.advance();
                        }
                        self.consume_newline();

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
            // Skip if after '->'  -- these are method names, not operators.
            #[allow(clippy::collapsible_if)]
            if !self.after_sub
                && !self.after_arrow
                && !follows_sigil_prefix
                && self.hash_brace_depth == 0
                && matches!(text, "s" | "tr" | "y")
            {
                let immediate = self.current_char();
                let (candidate, char_after_next, has_whitespace) =
                    if immediate.is_some_and(|c| c.is_whitespace()) {
                        let (nc, ca) = self.peek_nonspace_and_following();
                        (nc, ca, true)
                    } else {
                        let following = immediate.and_then(|c| {
                            let j = self.position + c.len_utf8();
                            self.input.get(j..).and_then(|s| s.chars().next())
                        });
                        (immediate, following, false)
                    };

                if let Some(next) = candidate {
                    // `s => 1` should remain a fat-arrow hash key, not quote op.
                    let is_fat_arrow = next == '=' && char_after_next == Some('>');
                    let is_filetest_s = text == "s"
                        && self.input.get(..start).is_some_and(|prefix| prefix.ends_with('-'));
                    let is_paired_delim = matches!(next, '{' | '[' | '(' | '<');
                    let is_quote_char = matches!(next, '\'' | '"') && text != "s";
                    let transliteration_allows_whitespace = text == "tr" || text == "y";
                    let substitution_disallows_whitespace = text == "s" && has_whitespace;
                    let is_valid_delim = Self::is_quote_delim(next)
                        && !is_fat_arrow
                        && !is_filetest_s
                        && !substitution_disallows_whitespace
                        && (!has_whitespace
                            || is_paired_delim
                            || is_quote_char
                            || transliteration_allows_whitespace);

                    if is_valid_delim {
                        match text {
                            "s" => return self.parse_substitution(start),
                            "tr" | "y" => return self.parse_transliteration(start),
                            unexpected => {
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

            let token_type = if is_keyword_fast(text) {
                // Check for special keywords that affect lexer mode
                match text {
                    "if" | "unless" | "while" | "until" | "for" | "foreach" | "grep" | "map"
                    | "sort" | "split" | "and" | "or" | "xor" | "not"
                    // These keywords introduce an expression, so a following `/` is a
                    // regex, not division.  `return /re/`, `die /re/`, `warn /re/`,
                    // `do /file/`, and `eval /re/` are all valid Perl.
                    | "return" | "die" | "warn" | "do" | "eval" => {
                        self.mode = LexerMode::ExpectTerm;
                    }
                    "sub" => {
                        self.after_sub = true;
                        self.mode = LexerMode::ExpectTerm;
                    }
                    // Quote operators expect a delimiter next.
                    // Skip if after '->' -- these are method names, not operators.
                    // Inside hash subscript braces, regex-like operators stay bareword
                    // keys (`@h{m, s}`), but q-family operators can still introduce real
                    // quote expressions in slices (`@h{qw/a b/}`).
                    op if !self.after_sub
                        && !self.after_arrow
                        && !follows_sigil_prefix
                        && quote_handler::is_quote_operator(op)
                        && (self.hash_brace_depth == 0
                            || matches!(op, "q" | "qq" | "qw" | "qr" | "qx")) =>
                    {
                        // Perl allows whitespace between a quote-like operator and its delimiter,
                        // but ONLY for paired delimiters (s { ... } { ... }g).
                        // For non-paired delimiters (s/foo/bar/, s,foo,bar,), the delimiter
                        // must be immediately adjacent — otherwise `s $foo` would wrongly
                        // treat `$` as a delimiter instead of being a bareword `s` followed
                        // by a scalar variable.
                        //
                        // Strategy:
                        //   1. Check the immediately-adjacent char first (no whitespace skip).
                        //      If it is a valid delimiter → any non-alnum, non-whitespace char.
                        //   2. If the adjacent char is whitespace, peek past it.
                        //      Only accept PAIRED delimiters ({, [, (, <) in that case.
                        let immediate = self.current_char();
                        let (candidate, char_after_next, has_whitespace) =
                            if immediate.is_some_and(|c| c.is_whitespace()) {
                                // There is whitespace — peek past it
                                let (nc, ca) = self.peek_nonspace_and_following();
                                (nc, ca, true)
                            } else {
                                // No whitespace — use immediate char
                                let following = immediate.and_then(|c| {
                                    let j = self.position + c.len_utf8();
                                    self.input.get(j..).and_then(|s| s.chars().next())
                                });
                                (immediate, following, false)
                            };

                        if let Some(next) = candidate {
                            // Fat-arrow autoquoting: `s => value` — `=` followed by `>` is '=>',
                            // not a valid substitution delimiter. Treat as identifier.
                            let is_fat_arrow = next == '=' && char_after_next == Some('>');
                            let is_filetest_s =
                                op == "s" && self.input.get(..start).is_some_and(|prefix| {
                                    prefix.ends_with('-')
                                });

                            // When whitespace precedes the delimiter, only unambiguous
                            // delimiters are accepted:
                            //   - Paired delimiters ({, [, (, <) are always safe.
                            //   - ' and " are safe for all operators EXCEPT `s` — `-s 'filename'`
                            //     is a valid file-size filetest and must not be treated as a
                            //     substitution start. All other operators (qw, q, qq, qr, qx, m,
                            //     tr, y) have no corresponding file-test operator.
                            //   - / is safe for non-substitution quote operators; `qw /a b/` and
                            //     `m /re/` are common, while `s /foo/bar/` remains ambiguous with
                            //     the file-size test shape and stays rejected here.
                            //   - Non-paired, non-quote chars ($, @, ,, etc.) remain rejected.
                            let is_paired_delim = matches!(next, '{' | '[' | '(' | '<');
                            let is_quote_char = matches!(next, '\'' | '"') && op != "s";
                            let is_spaced_slash_delim = next == '/' && op != "s";
                            let is_hash_subscript_bare_key_boundary =
                                self.hash_brace_depth > 0 && matches!(next, ',' | '}');
                            let is_valid_delim = Self::is_quote_delim(next)
                                && !is_fat_arrow
                                && !is_filetest_s
                                && !is_hash_subscript_bare_key_boundary
                                && (!has_whitespace
                                    || is_paired_delim
                                    || is_quote_char
                                    || is_spaced_slash_delim);

                            if is_valid_delim {
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
                    _ if is_builtin_function(text) => {
                        // Bare builtins are term-introducing in Perl.
                        self.mode = LexerMode::ExpectTerm;
                    }
                    _ => {
                        self.mode = LexerMode::ExpectOperator;
                    }
                }
                TokenType::Keyword(Arc::from(text))
            } else {
                // Mirror parser bare-builtin handling so `/` after builtins like
                // `join` or `print` is lexed as a regex term, not division.
                if is_builtin_function(text) {
                    self.mode = LexerMode::ExpectTerm;
                } else {
                    self.mode = LexerMode::ExpectOperator;
                }
                TokenType::Identifier(Arc::from(text))
            };

            self.after_arrow = false;
            // A keyword/identifier is not a variable; `{` after it is a block opener.
            self.after_var_subscript = false;
            // hash_brace_depth is managed by { and } handlers, not cleared per-token
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
        // 3. **Budget Protection**:
        //    - Regex parsing has a parse-step budget and byte budget
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
                if self.peek_byte(0) == Some(b'/') {
                    self.position += 1; // consume second / directly
                    if self.peek_byte(0) == Some(b'=') {
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
                } else if self.position < self.input_bytes.len()
                    && self.input_bytes[self.position] == b'='
                {
                    // /= division-assign operator
                    self.position += 1; // consume =
                    self.mode = LexerMode::ExpectTerm;
                    return Some(Token {
                        token_type: TokenType::Operator(Arc::from("/=")),
                        text: Arc::from("/="),
                        start,
                        end: self.position,
                    });
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
                // Check if it's a decimal number like .5 -- but only when we
                // expect a term.  In operator position `.5` is concatenation
                // of the bareword/number on the left with the number `5`.
                if self.mode != LexerMode::ExpectOperator
                    && self.peek_char(1).is_some_and(|c| c.is_ascii_digit())
                {
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
        // Operator ends prototype window (e.g. `:` for attributes)
        self.after_sub = false;
        // Track whether this operator is '->' for method name disambiguation
        self.after_arrow = text == "->";
        // Any operator token ends the "just saw a variable" window; `{` after
        // an operator is not a hash subscript (e.g. `foo() {`, `+ {`, etc.).
        self.after_var_subscript = false;
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
                if self.after_sub {
                    // Promote after_sub to in_prototype now that we see '('
                    self.in_prototype = true;
                    self.after_sub = false;
                    self.prototype_depth = 1;
                } else if self.in_prototype {
                    self.prototype_depth += 1;
                }
                self.paren_depth += 1;
                self.after_var_subscript = false;
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
                self.after_arrow = false;
                self.paren_depth = self.paren_depth.saturating_sub(1);
                // A closing paren ends any var-subscript context: `if ($var)` should
                // NOT leave after_var_subscript set, otherwise the following `{` would
                // incorrectly increment hash_brace_depth and suppress regex operators
                // inside the block body (issue #2844).
                self.after_var_subscript = false;
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
                // Semicolon ends prototype window (forward declaration)
                self.after_sub = false;
                // Semicolon is a statement boundary — any pending method-call chain is over.
                self.after_arrow = false;
                self.after_var_subscript = false;
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
                self.after_var_subscript = false;
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
                self.after_var_subscript = false;
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
                // A closing `]` from an array subscript leaves us in a state where
                // a `{` immediately following is a hash subscript — e.g. `$arr[$i]{key}`.
                // Set after_var_subscript so the `{` handler recognises it as such.
                // This mirrors the `}` handler's behavior when closing a hash subscript.
                self.after_var_subscript = true;
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
                // Opening brace ends prototype window — no prototype follows
                self.after_sub = false;
                // `{` is a hash/slice subscript opener only when it immediately follows
                // a variable token ($x, @x, %x) — tracked by `after_var_subscript`.
                // This is narrower than the old `mode == ExpectOperator` check, which
                // incorrectly incremented depth for block-opening braces after `sub foo`,
                // `if (cond)`, `else`, `while (cond)`, etc., causing quote-op suppression
                // inside those block bodies and breaking m//, s///, qr//, tr/// etc.
                if self.after_var_subscript {
                    self.hash_brace_depth = self.hash_brace_depth.saturating_add(1);
                }
                self.after_var_subscript = false;
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
                self.after_arrow = false;
                // Decrement hash subscript brace depth only if we were inside one.
                // If depth > 0, this closes a hash subscript; enable chained subscripts
                // like $h{a}{b} by setting after_var_subscript so the next `{` is
                // recognized as another subscript opener.
                if self.hash_brace_depth > 0 {
                    self.hash_brace_depth -= 1;
                    // The subscript value is now the "variable" for a chained subscript.
                    self.after_var_subscript = true;
                } else {
                    // Block-close `}` — no subscript follows
                    self.after_var_subscript = false;
                }
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

                    let part_start = self.position;
                    self.advance();
                    match self.current_char() {
                        Some('{') => {
                            let _ = self.consume_balanced_segment_in_string('{', '}', '"');
                            parts.push(StringPart::Expression(Arc::from(
                                &self.input[part_start..self.position],
                            )));
                        }
                        Some(ch) if is_perl_identifier_start(ch) => {
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
                                let var_name = &self.input[part_start..self.position];
                                parts.push(StringPart::Variable(Arc::from(var_name)));

                                if self.matches_bytes(b"->") {
                                    let tail_start = self.position;
                                    self.advance();
                                    self.advance();

                                    match self.current_char() {
                                        Some('[') => {
                                            let _ = self
                                                .consume_balanced_segment_in_string('[', ']', '"');
                                            parts.push(StringPart::MethodCall(Arc::from(
                                                &self.input[tail_start..self.position],
                                            )));
                                        }
                                        Some('{') => {
                                            let _ = self
                                                .consume_balanced_segment_in_string('{', '}', '"');
                                            parts.push(StringPart::MethodCall(Arc::from(
                                                &self.input[tail_start..self.position],
                                            )));
                                        }
                                        Some('(') => {
                                            let _ = self
                                                .consume_balanced_segment_in_string('(', ')', '"');
                                            parts.push(StringPart::MethodCall(Arc::from(
                                                &self.input[tail_start..self.position],
                                            )));
                                        }
                                        Some(ch) if is_perl_identifier_start(ch) => {
                                            while self.position < self.input_bytes.len() {
                                                let byte = self.input_bytes[self.position];
                                                if byte.is_ascii_alphanumeric() || byte == b'_' {
                                                    self.position += 1;
                                                } else if byte >= 128 {
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
                                            if self.current_char() == Some('(') {
                                                let _ = self.consume_balanced_segment_in_string(
                                                    '(', ')', '"',
                                                );
                                            }
                                            parts.push(StringPart::MethodCall(Arc::from(
                                                &self.input[tail_start..self.position],
                                            )));
                                        }
                                        _ => {
                                            parts.push(StringPart::MethodCall(Arc::from(
                                                &self.input[tail_start..self.position],
                                            )));
                                        }
                                    }
                                } else if self.current_char() == Some('[') {
                                    let tail_start = self.position;
                                    let _ = self.consume_balanced_segment_in_string('[', ']', '"');
                                    parts.push(StringPart::ArraySlice(Arc::from(
                                        &self.input[tail_start..self.position],
                                    )));
                                } else if self.current_char() == Some('{') {
                                    let tail_start = self.position;
                                    let _ = self.consume_balanced_segment_in_string('{', '}', '"');
                                    parts.push(StringPart::Expression(Arc::from(
                                        &self.input[tail_start..self.position],
                                    )));
                                }
                            }
                        }
                        _ => {}
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

        Some(self.unterminated_string_error(start))
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

        Some(self.unterminated_string_error(start))
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

        Some(self.unterminated_string_error(start))
    }

    fn parse_q_string(&mut self, _start: usize) -> Option<Token> {
        // Simplified q-string parsing
        None
    }

    #[inline]
    fn unterminated_string_error(&mut self, start: usize) -> Token {
        // Consume to EOF so the caller receives a single terminal error token.
        let end = self.input.len();
        self.position = end;

        Token {
            token_type: TokenType::Error(Arc::from("unterminated string")),
            text: Arc::from(&self.input[start..end]),
            start,
            end,
        }
    }

    fn parse_substitution(&mut self, start: usize) -> Option<Token> {
        // We've already consumed 's'
        let delimiter = self.current_char()?;
        self.advance(); // Skip delimiter
        self.parse_substitution_with_delimiter(start, delimiter)
    }

    fn parse_substitution_with_delimiter(
        &mut self,
        start: usize,
        delimiter: char,
    ) -> Option<Token> {
        let (_pattern, pattern_closed) = self.read_delimited_body(delimiter);
        let replacement_closed;

        let pattern_is_paired = quote_handler::paired_close(delimiter).is_some();
        if pattern_is_paired {
            self.skip_paired_substitution_replacement_gap();

            if let Some(repl_delim) = self.current_char()
                && Self::is_quote_delim(repl_delim)
            {
                self.advance();
                let (_replacement, closed) = self.read_substitution_replacement_body(repl_delim);
                replacement_closed = closed;
            } else {
                replacement_closed = false;
            }
        } else {
            let (_replacement, closed) = self.read_substitution_replacement_body(delimiter);
            replacement_closed = closed;
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

        let token_type = if pattern_closed && replacement_closed {
            TokenType::Substitution
        } else {
            TokenType::Error(Arc::from(format!(
                "unclosed quote-like operator 's' delimiter '{}'",
                delimiter
            )))
        };

        Some(Token { token_type, text: Arc::from(text), start, end: self.position })
    }

    fn skip_paired_substitution_replacement_gap(&mut self) {
        let mut comment_eligible = false;
        loop {
            let mut saw_whitespace = false;
            while self.current_char().is_some_and(char::is_whitespace) {
                self.advance();
                saw_whitespace = true;
            }
            comment_eligible |= saw_whitespace;

            if comment_eligible && self.current_char() == Some('#') {
                while let Some(ch) = self.current_char() {
                    self.advance();
                    if matches!(ch, '\n' | '\r') {
                        break;
                    }
                }
                comment_eligible = true;
                continue;
            }

            break;
        }
    }

    fn read_substitution_replacement_body(&mut self, delim: char) -> (String, bool) {
        if quote_handler::paired_close(delim).is_some() {
            return self.read_delimited_body(delim);
        }

        self.read_unpaired_substitution_replacement_body(delim)
    }

    fn read_unpaired_substitution_replacement_body(&mut self, delim: char) -> (String, bool) {
        let mut body = String::new();
        let mut escaped = false;

        while let Some(ch) = self.current_char() {
            if escaped {
                body.push(ch);
                self.advance();
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    body.push(ch);
                    self.advance();
                    escaped = true;
                }
                '"' | '\'' if ch != delim => {
                    if let Some((string_end, true)) =
                        self.scan_inner_string_for_delimiter(self.position, ch, delim)
                    {
                        if let Some(string_text) = self.input.get(self.position..string_end) {
                            body.push_str(string_text);
                            self.position = string_end;
                        } else {
                            body.push(ch);
                            self.advance();
                        }
                    } else {
                        body.push(ch);
                        self.advance();
                    }
                }
                c if c == delim => {
                    self.advance();
                    return (body, true);
                }
                _ => {
                    body.push(ch);
                    self.advance();
                }
            }
        }

        (body, false)
    }

    fn scan_inner_string_for_delimiter(
        &self,
        start: usize,
        quote: char,
        delim: char,
    ) -> Option<(usize, bool)> {
        if Self::is_word_apostrophe(self.input, start, quote) {
            return None;
        }
        // Adjacent quotes are literal replacement text (for example s/"/""/g),
        // not a string literal to skip while hunting for the replacement delimiter.
        if self.input.get(..start).and_then(|text| text.chars().next_back()) == Some(quote) {
            return None;
        }
        let mut pos = start.checked_add(quote.len_utf8())?;
        let expression_quote = Self::can_start_replacement_expression_quote(self.input, start);
        if !expression_quote && self.input.get(pos..).is_some_and(|text| text.starts_with(delim)) {
            return None;
        }
        if self.input.get(pos..).is_some_and(|text| text.starts_with(quote)) {
            return None;
        }
        let mut escaped = false;
        let mut contains_delim = false;

        while let Some(ch) = self.input.get(pos..).and_then(|text| text.chars().next()) {
            if matches!(ch, '\n' | '\r') {
                return None;
            }
            if !expression_quote && matches!(ch, ';' | '#') {
                return None;
            }

            if escaped {
                if ch == delim {
                    contains_delim = true;
                }
                pos += ch.len_utf8();
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    pos += ch.len_utf8();
                    escaped = true;
                }
                c if c == quote => {
                    return Some((pos + ch.len_utf8(), contains_delim));
                }
                c if c == delim => {
                    contains_delim = true;
                    pos += ch.len_utf8();
                }
                _ => {
                    pos += ch.len_utf8();
                }
            }
        }

        None
    }

    // Only skip delimiter-bearing inner strings in positions that look like
    // replacement expressions; literal replacement quotes still let the next
    // delimiter close the substitution.
    fn can_start_replacement_expression_quote(input: &str, pos: usize) -> bool {
        input
            .get(..pos)
            .and_then(|text| text.chars().rev().find(|ch| !ch.is_whitespace()))
            .is_some_and(|ch| {
                matches!(
                    ch,
                    '(' | '['
                        | '{'
                        | ','
                        | '='
                        | ':'
                        | '?'
                        | '!'
                        | '~'
                        | '+'
                        | '-'
                        | '*'
                        | '%'
                        | '&'
                        | '|'
                        | '^'
                        | '<'
                        | '>'
                )
            })
    }

    fn is_word_apostrophe(input: &str, pos: usize, quote: char) -> bool {
        quote == '\''
            && input
                .get(..pos)
                .and_then(|text| text.chars().next_back())
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }

    fn parse_transliteration(&mut self, start: usize) -> Option<Token> {
        // We've already consumed 'tr' or 'y'
        while self.current_char().is_some_and(char::is_whitespace) {
            self.advance();
        }

        let delimiter = self.current_char()?;
        self.advance(); // Skip delimiter
        self.parse_transliteration_with_delimiter(start, delimiter)
    }

    fn parse_transliteration_with_delimiter(
        &mut self,
        start: usize,
        delimiter: char,
    ) -> Option<Token> {
        let (_search, search_closed) = self.read_delimited_body(delimiter);
        let replacement_closed;

        let search_is_paired = quote_handler::paired_close(delimiter).is_some();
        if search_is_paired {
            while self.current_char().is_some_and(char::is_whitespace) {
                self.advance();
            }

            if let Some(repl_delim) = self.current_char()
                && Self::is_quote_delim(repl_delim)
            {
                self.advance();
                let (_replacement, closed) = self.read_delimited_body(repl_delim);
                replacement_closed = closed;
            } else {
                replacement_closed = false;
            }
        } else {
            let (_replacement, closed) = self.read_delimited_body(delimiter);
            replacement_closed = closed;
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

        let token_type = if search_closed && replacement_closed {
            TokenType::Transliteration
        } else {
            TokenType::Error(Arc::from(format!(
                "unclosed quote-like operator '{}' delimiter '{}'",
                if self.input[start..].starts_with("tr") { "tr" } else { "y" },
                delimiter
            )))
        };

        Some(Token { token_type, text: Arc::from(text), start, end: self.position })
    }

    /// Read content between delimiters.
    ///
    /// Returns `(body, closed)` where `closed` is `true` if the closing
    /// delimiter was found before EOF, and `false` if EOF was reached first.
    fn read_delimited_body(&mut self, delim: char) -> (String, bool) {
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
                        return (body, true);
                    }
                    body.push(ch);
                    self.advance();
                } else {
                    self.advance();
                    return (body, true);
                }
                continue;
            }

            body.push(ch);
            self.advance();
        }

        // EOF reached without finding the closing delimiter
        (body, false)
    }

    /// Parse a quote operator after we've seen the delimiter
    fn parse_quote_operator(&mut self, delimiter: char) -> Option<Token> {
        let info = self.current_quote_op.as_ref()?;
        let start = info.start_pos;
        let operator = info.operator.clone();

        // Clear the quote-op context eagerly so any early-return path (s/tr/y delegations
        // below) does not leave a stale reference behind. The post-match cleanup at the
        // bottom of this function would otherwise be skipped for those operators.
        self.current_quote_op = None;

        // Parse based on operator type; track whether all delimiters were closed.
        let closed = match operator.as_str() {
            "s" => {
                return self.parse_substitution_with_delimiter(start, delimiter);
            }
            "tr" | "y" => {
                return self.parse_transliteration_with_delimiter(start, delimiter);
            }
            "qr" => {
                let (_pattern, body_closed) = self.read_delimited_body(delimiter);
                self.parse_regex_modifiers(&quote_handler::QR_SPEC);
                body_closed
            }
            "m" => {
                let (_pattern, body_closed) = self.read_delimited_body(delimiter);
                self.parse_regex_modifiers(&quote_handler::M_SPEC);
                body_closed
            }
            _ => {
                // q, qq, qw, qx - no modifiers
                let (_body, body_closed) = self.read_delimited_body(delimiter);
                body_closed
            }
        };

        let text = &self.input[start..self.position];

        self.mode = LexerMode::ExpectOperator;

        if !closed {
            // EOF reached before finding the closing delimiter — emit an error
            // token so the parser's recovery mechanism records a diagnostic.
            return Some(Token {
                token_type: TokenType::Error(Arc::from(format!(
                    "unclosed {} delimiter '{}'",
                    operator, delimiter
                ))),
                text: Arc::from(text),
                start,
                end: self.position,
            });
        }

        let token_type = quote_handler::get_quote_token_type(&operator);
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
    /// **Budget Protection (Issue #422)**:
    /// - Budget guards prevent runaway scanning on pathological input
    /// - `MAX_REGEX_PARSE_STEPS` bounds literal scanning before the byte budget
    /// - `MAX_REGEX_BYTES` bounds total bytes consumed in a single regex literal
    /// - Graceful degradation: emit UnknownRest token if budget exceeded
    ///
    /// **Performance**:
    /// - Single-pass scanning with escape handling
    /// - Budget check per iteration (amortized O(1) via inline fast path)
    /// - Typical regex: <10μs, Large regex (64KB): ~1ms
    fn parse_regex(&mut self, start: usize) -> Option<Token> {
        self.advance(); // Skip opening /

        let mut regex_parse_steps: usize = 0;
        let mut in_character_class = false;

        while let Some(ch) = self.current_char() {
            regex_parse_steps += 1;
            if regex_parse_steps > MAX_REGEX_PARSE_STEPS {
                #[cfg(debug_assertions)]
                {
                    let text = &self.input[start..self.position];
                    let preview = truncate_preview(text, 50);
                    tracing::debug!(
                        limit = MAX_REGEX_PARSE_STEPS,
                        pattern_preview = %preview,
                        "Regex parse step budget exceeded"
                    );
                }
                self.position = self.input.len();
                return Some(Token {
                    token_type: TokenType::UnknownRest,
                    text: empty_arc(),
                    start,
                    end: self.position,
                });
            }

            // Budget guard: prevent timeout on pathological input (Issue #422)
            // If exceeded, returns UnknownRest token for graceful degradation
            if let Some(token) = self.budget_guard(start, 0) {
                return Some(token);
            }

            match ch {
                '/' if !in_character_class => {
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
                '[' => {
                    in_character_class = true;
                    self.advance();
                }
                ']' if in_character_class => {
                    in_character_class = false;
                    self.advance();
                }
                _ => self.advance(),
            }
        }

        // Unterminated regex - EOF reached before closing /
        // Parser will emit diagnostic for unterminated literal
        None
    }
}

// Checkpoint support for incremental parsing

mod checkpoint_impl;

#[cfg(test)]
mod test_format_debug;
#[cfg(test)]
mod tests;
