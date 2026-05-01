//! Context-aware Perl lexer with mode-based tokenization

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
pub mod keywords;
pub mod lexer;
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
