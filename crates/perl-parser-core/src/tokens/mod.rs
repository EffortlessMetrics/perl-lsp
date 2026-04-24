//! Token stream and trivia utilities for parser workflows.
//!
//! This module hosts the parser-facing token infrastructure that depends
//! on `perl-error` and `perl-ast-v2` (cannot live in `perl-lexer` without
//! creating a dependency cycle):
//!
//! - [`token_stream`] — buffered [`TokenStream`](token_stream::TokenStream)
//!   over the raw lexer with 3-token lookahead, trivia skipping, and
//!   statement-boundary mode resets.
//! - [`trivia`] — whitespace/comment/POD classification with
//!   [`Trivia`](trivia::Trivia) and [`TriviaLexer`](trivia::TriviaLexer).
//! - [`trivia_parser`] — trivia-preserving parser context and `format_with_trivia`.
//!
//! AST-agnostic utilities (token position wrapping, `__DATA__`/`__END__`
//! marker scanning) live in [`perl_lexer::tokenizer`] and are re-exported
//! here for discoverability.

/// Buffered token stream over the raw lexer (with trivia skipping).
pub mod token_stream;
/// Token wrapper utilities for preserving original lexemes.
pub use perl_lexer::tokenizer::token_wrapper;
/// Trivia tokens (whitespace/comments/POD) used for formatting and diagnostics.
pub mod trivia;
/// Trivia-preserving parser helpers for formatting context.
pub mod trivia_parser;
