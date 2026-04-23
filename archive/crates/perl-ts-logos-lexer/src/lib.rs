//! Logos-based token parser for Perl — ARCHIVED EXPERIMENT
//!
//! This crate is retained as evidence of the logos-based tokenization approach
//! explored during the v1→v3 parser transition. It is not integrated into the
//! LSP stack. The compilable subset (simple_token, simple_parser_v2, context
//! lexers) correctly demonstrates slash disambiguation via DFA; the full
//! regex/heredoc/position-tracking parity needed for production use was not
//! completed.
//!
//! See: perl-lexer for the production tokenizer.

pub mod context_lexer_simple;
pub mod context_lexer_v2;
pub mod regex_parser;
pub mod simple_parser;
pub mod simple_parser_v2;
pub mod simple_token;
pub mod token_ast;
