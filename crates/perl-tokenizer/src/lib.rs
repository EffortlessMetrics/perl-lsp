//! Token stream and trivia utilities for the parser.
//!
//! Wraps the raw token output of `perl-lexer` into a position-aware
//! [`TokenStream`] and preserves whitespace/comment trivia via the
//! [`TriviaPreservingParser`]. Used by the parser and formatting provider
//! to maintain lossless round-trip fidelity.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]

pub mod token_stream;
pub mod token_wrapper;
pub mod trivia;
pub mod trivia_parser;
pub mod util;

pub use perl_token::{Token, TokenKind};
pub use token_stream::TokenStream;
pub use token_wrapper::TokenWithPosition;
pub use trivia::{Trivia, TriviaToken};
pub use trivia_parser::{TriviaParserContext, TriviaPreservingParser};
