//! Standalone heredoc analysis tools for Perl parsing
//!
//! This crate provides detection and analysis of problematic Perl patterns,
//! particularly around heredocs, including anti-pattern detection, dynamic
//! delimiter recovery, encoding-aware lexing, and statement tracking.

pub mod anti_pattern_detector;
pub mod context_sensitive;
pub mod dynamic_delimiter_recovery;
pub mod encoding_aware_lexer;
pub mod runtime_heredoc_handler;
pub mod string_utils;

pub use perl_ts_statement_tracker as statement_tracker;
