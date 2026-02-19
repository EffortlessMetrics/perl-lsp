//! Logos-based token parser for Perl
//!
//! This crate provides token definitions and lexer implementations built on
//! the Logos lexer generator, including context-aware lexing, regex parsing,
//! and simple recursive descent parsers.

pub mod context_lexer_simple;
pub mod context_lexer_v2;
pub mod regex_parser;
pub mod simple_parser;
pub mod simple_parser_v2;
pub mod simple_token;
pub mod token_ast;

#[cfg(feature = "logos-tokens")]
pub mod logos_lexer;
#[cfg(feature = "logos-tokens")]
pub mod token_parser;
