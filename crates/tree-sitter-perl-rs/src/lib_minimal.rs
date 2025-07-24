//! Minimal lib.rs for pure Rust Pest parser benchmarking
//! 
//! This file provides just the essentials needed for benchmarking
//! without the perl-lexer dependencies.

#![cfg(feature = "pure-rust")]

pub mod error;
pub mod unicode;
pub mod pure_rust_parser;
pub mod pratt_parser;

// Re-export the main parser for benchmarks
pub use pure_rust_parser::PureRustPerlParser;
pub use pure_rust_parser::PureRustPerlParser as PureRustParser;
pub use pure_rust_parser::{AstNode, PerlParser};

// Export Rule enum from the parser
pub use crate::pure_rust_parser::Rule;