//! Minimal module that exports only the Pest-based parser
//! This avoids all perl-lexer dependencies for benchmarking

pub use crate::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};

/// Benchmark-friendly wrapper that provides the expected interface
pub struct PestOnlyParser {
    inner: PureRustPerlParser,
}

impl PestOnlyParser {
    pub fn new() -> Self {
        Self { inner: PureRustPerlParser::new() }
    }

    // Provide immutable parse method for benchmarks
    pub fn parse(&self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        // Create a new parser instance each time since parse requires &mut self
        let mut parser = PureRustPerlParser::new();
        parser.parse(input)
    }
}

impl Default for PestOnlyParser {
    fn default() -> Self {
        Self::new()
    }
}
