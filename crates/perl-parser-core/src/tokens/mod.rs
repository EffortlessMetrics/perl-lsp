//! Token stream and trivia utilities for parser workflows.
//!
//! This module re-exports tokenizer helpers used during the Parse → Index → Analyze
//! stages to power LSP features such as diagnostics, completion, and navigation.
//!
//! # Examples
//!
//! ```rust
//! use perl_parser_core::tokens::token_stream::TokenStream;
//!
//! let mut stream = TokenStream::new("my $x = 1;");
//! let _ = stream.peek();
//! ```

/// Token stream adapters used during the Parse stage for LSP workflows.
pub mod token_stream;
/// Token wrapper utilities for preserving original lexemes and trivia.
pub use perl_tokenizer::token_wrapper;
/// Trivia tokens (whitespace/comments) used for formatting and diagnostics.
pub use perl_tokenizer::trivia;
/// Trivia parser helpers for preserving formatting context.
pub use perl_tokenizer::trivia_parser;
