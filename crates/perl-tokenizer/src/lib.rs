//! Token stream and trivia utilities for the parser.

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
