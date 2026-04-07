//! Heredoc parsing pipeline for Perl
//!
//! This crate provides the core heredoc parsing infrastructure including
//! the Perl lexer with slash disambiguation, heredoc recovery, multi-phase
//! heredoc parsing, and lexer adapter for tree-sitter compatibility.

pub mod enhanced_heredoc_lexer;
pub mod heredoc_parser;
pub mod heredoc_recovery;
pub mod lexer_adapter;
pub mod perl_lexer;
