//! Composed parser experiments for Perl
//!
//! This crate provides advanced parsing capabilities including full parsers
//! with heredoc support, streaming parsers, error recovery, incremental
//! parsing, and an experimental LSP server implementation.

pub mod context_aware_parser;
pub mod disambiguated_parser;
pub mod enhanced_full_parser;
pub mod enhanced_parser;
pub mod error_recovery;
pub mod full_parser;
pub mod incremental_parser;
pub mod iterative_parser;
pub mod lsp_server;
pub mod stateful_parser;
pub mod streaming_parser;
