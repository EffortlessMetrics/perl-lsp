//! Token utilities bridging raw lexer output to parser consumption.
//!
//! This module consolidates the AST-agnostic slice of the former
//! `perl-tokenizer` crate that has no dependency on `perl-error` or
//! `perl-ast-v2`:
//!
//! - [`token_wrapper`] — position-tracked wrappers over lexer tokens.
//! - [`util`] — [`__DATA__`/`__END__`](util) marker utilities.
//!
//! The buffered [`TokenStream`](perl_parser_core::tokens::token_stream::TokenStream)
//! lives in `perl-parser-core` because it uses `perl-error`'s `ParseError`
//! type, which would create a dependency cycle if it lived here. Trivia
//! preservation (comments/whitespace → AST) also lives in `perl-parser-core`,
//! since it depends on `perl-ast-v2`.

pub mod token_wrapper;
pub mod util;

pub use token_wrapper::{PositionTracker, TokenWithPosition};
