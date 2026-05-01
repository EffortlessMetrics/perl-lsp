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

pub mod api;
pub mod builtins;
pub mod checkpoint;
pub mod config;
pub mod error;
mod heredoc;
mod lexer;
pub mod keywords;
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
pub use limits::MAX_REGEX_PARSE_STEPS;
pub use mode::LexerMode;
pub use perl_position_tracking::Position;
pub use token::{StringPart, Token, TokenType};

pub use lexer::PerlLexer;


#[cfg(test)]
mod test_format_debug;
