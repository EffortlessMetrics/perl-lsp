//! Simplified parser interface for benchmarking
//!
//! This module provides a clean interface to the Pest-based parser
//! without dependencies on perl-lexer.

use crate::pure_rust_parser::{AstNode, PureRustPerlParser};
use pest::Parser;

/// Benchmark-friendly wrapper for the Pure Rust Pest parser
/// This wrapper provides an immutable parse method for benchmarking
pub struct BenchmarkPureRustParser;

impl BenchmarkPureRustParser {
    /// Create a new parser instance
    pub fn new() -> Self {
        BenchmarkPureRustParser
    }

    /// Parse Perl code (immutable interface for benchmarking)
    pub fn parse(&self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        // Create a new mutable parser instance for each parse
        let mut parser = PureRustPerlParser::new();
        parser.parse(input)
    }
}

impl Default for BenchmarkPureRustParser {
    fn default() -> Self {
        Self::new()
    }
}
