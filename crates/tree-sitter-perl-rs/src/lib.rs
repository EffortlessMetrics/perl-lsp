//! Pure Rust Perl Parser - A modern Pest-based parser with tree-sitter compatibility
//!
//! This crate provides a Pure Rust parser for Perl 5, built with the Pest parser
//! generator. It outputs tree-sitter compatible S-expressions and requires no C
//! dependencies, making it ideal for cross-platform Perl tooling.
//!
//! ## Features
//!
//! - **pure-rust**: Pure Rust Pest-based parser (canonical implementation)
//! - **v2-pest-microcrate**: Route v2 Pest modules through `perl-parser-pest` (default path)
//! - **test-utils**: Testing utilities and benchmarking tools
//! - **c-scanner**: Legacy C implementation (for benchmarking only)
//!
//! ## Usage
//!
//! ```rust
//! use tree_sitter_perl::PureRustPerlParser;
//!
//! // Create parser instance
//! let mut parser = PureRustPerlParser::new();
//!
//! // Parse Perl code
//! let code = r#"
//!     sub hello {
//!         my $name = shift;
//!         print "Hello, $name!\n";
//!     }
//! "#;
//!
//! // Get AST and convert to S-expression
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let ast = parser.parse(code)?;
//! let sexp = parser.to_sexp(&ast);
//! println!("{}", sexp);
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The parser uses a three-stage pipeline:
//! 1. **Pest Parsing**: PEG grammar processes input into parse tree
//! 2. **AST Building**: Type-safe AST construction with position tracking
//! 3. **S-Expression Output**: Tree-sitter compatible format generation
//!
//! See [ARCHITECTURE.md](https://github.com/EffortlessMetrics/tree-sitter-perl/blob/main/ARCHITECTURE.md) for details.

pub mod error;
pub mod scanner;
pub mod unicode;

// These modules depend on perl-lexer, so only compile when not using pure-rust standalone
#[cfg(not(feature = "pure-rust-standalone"))]
pub mod ast;
#[cfg(not(feature = "pure-rust-standalone"))]
pub mod minimal_parser;
#[cfg(not(feature = "pure-rust-standalone"))]
pub mod parser;
#[cfg(not(feature = "pure-rust-standalone"))]
pub mod parser_v2;
#[cfg(not(feature = "pure-rust-standalone"))]
pub mod token_compat;
#[cfg(not(feature = "pure-rust-standalone"))]
pub mod working_parser;

#[cfg(feature = "pure-rust")]
pub mod benchmark_parser;
#[cfg(feature = "pure-rust-standalone")]
pub mod pest_only;
#[cfg(all(feature = "pure-rust", feature = "v2-pest-microcrate"))]
#[path = "pure_rust_parser_bridge.rs"]
pub mod pure_rust_parser;
#[cfg(all(feature = "pure-rust", not(feature = "v2-pest-microcrate")))]
pub mod pure_rust_parser;

#[cfg(all(feature = "pure-rust", not(feature = "pure-rust-standalone")))]
pub use ast::{Node, NodeKind, SourceLocation};
#[cfg(all(feature = "pure-rust", not(feature = "pure-rust-standalone")))]
pub use parser::Parser;
#[cfg(all(feature = "pure-rust", not(feature = "pure-rust-standalone")))]
pub use parser_v2::ParserV2;

// ── Re-exports from perl-ts-heredoc-analysis ──
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::anti_pattern_detector;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::context_sensitive;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::dynamic_delimiter_recovery;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::encoding_aware_lexer;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::runtime_heredoc_handler;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::statement_tracker;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_analysis::string_utils;

// ── Re-exports from perl-ts-logos-lexer ──
#[cfg(feature = "token-parser")]
pub use perl_ts_logos_lexer::simple_token;
#[cfg(feature = "token-parser")]
pub use perl_ts_logos_lexer::token_ast;
#[cfg(feature = "token-parser")]
pub use perl_ts_logos_lexer::context_lexer_simple;
pub use perl_ts_logos_lexer::regex_parser;
#[cfg(feature = "token-parser")]
pub use perl_ts_logos_lexer::context_lexer_v2;
#[cfg(feature = "token-parser")]
pub use perl_ts_logos_lexer::simple_parser;
#[cfg(feature = "token-parser")]
pub use perl_ts_logos_lexer::simple_parser_v2;

// ── Re-exports from perl-ts-heredoc-parser ──
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_parser::perl_lexer;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_parser::heredoc_recovery;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_parser::heredoc_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_parser::enhanced_heredoc_lexer;
#[cfg(feature = "pure-rust")]
pub use perl_ts_heredoc_parser::lexer_adapter;

// ── Re-exports from perl-ts-partial-ast ──
#[cfg(feature = "pure-rust")]
pub use perl_ts_partial_ast::partial_parse_ast;
#[cfg(feature = "pure-rust")]
pub use perl_ts_partial_ast::understanding_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_partial_ast::phase_aware_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_partial_ast::edge_case_handler;
#[cfg(feature = "pure-rust")]
pub use perl_ts_partial_ast::tree_sitter_adapter;

// ── Re-exports from perl-ts-advanced-parsers ──
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::context_aware_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::full_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::enhanced_full_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::streaming_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::error_recovery;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::incremental_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::lsp_server;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::stateful_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::enhanced_parser;
#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::disambiguated_parser;
#[cfg(all(feature = "pure-rust", not(feature = "v2-pest-microcrate")))]
pub use perl_ts_advanced_parsers::iterative_parser;

// ── Test modules (not moved) ──
#[cfg(all(feature = "token-parser", test))]
pub mod test_token_parser;
#[cfg(all(feature = "token-parser", test))]
pub mod test_debug;
#[cfg(all(feature = "token-parser", test))]
pub mod demo_token_parser;

// Re-export the main parser and types for convenience
#[cfg(feature = "pure-rust-standalone")]
pub use pest_only::PestOnlyParser as PureRustParser; // Clean Pest-only parser for benchmarks
#[cfg(feature = "pure-rust")]
pub use pure_rust_parser::PureRustPerlParser;
#[cfg(all(feature = "pure-rust", not(feature = "pure-rust-standalone")))]
pub use pure_rust_parser::PureRustPerlParser as PureRustParser; // Original for non-benchmark use
#[cfg(feature = "pure-rust")]
pub use pure_rust_parser::{AstNode, PerlParser};

#[cfg(all(feature = "pure-rust", test))]
pub mod parser_benchmark;

#[cfg(all(test, feature = "pure-rust"))]
mod pure_rust_parser_tests;

#[cfg(all(test, feature = "pure-rust"))]
mod fuzz_tests;

#[cfg(all(test, feature = "pure-rust"))]
mod test_slash;

#[cfg(all(feature = "pure-rust", feature = "v2-pest-microcrate"))]
#[path = "pratt_parser_bridge.rs"]
pub mod pratt_parser;
#[cfg(all(feature = "pure-rust", not(feature = "v2-pest-microcrate")))]
pub mod pratt_parser;

#[cfg(all(feature = "pure-rust", feature = "v2-pest-microcrate"))]
#[path = "sexp_formatter_bridge.rs"]
pub mod sexp_formatter;
#[cfg(all(feature = "pure-rust", not(feature = "v2-pest-microcrate")))]
pub mod sexp_formatter;

// #[cfg(feature = "pure-rust")]
// pub mod language_binding;

#[cfg(any(feature = "pure-rust", feature = "test-utils"))]
pub mod comparison_harness;

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "pure-rust"))]
mod test_format;
#[cfg(all(test, feature = "pure-rust"))]
mod test_format_order;
#[cfg(all(test, feature = "pure-rust"))]
mod test_statement_debug;

#[cfg(feature = "c-parser")]
use tree_sitter::Language;

// External C functions from the generated parser
#[cfg(feature = "c-parser")]
unsafe extern "C" {
    fn tree_sitter_perl() -> *const tree_sitter::ffi::TSLanguage;
}

/// Get the tree-sitter language for Perl
#[cfg(feature = "c-parser")]
pub fn language() -> Language {
    unsafe { Language::from_raw(tree_sitter_perl()) }
}

/// Create a new tree-sitter parser instance
#[cfg(feature = "c-parser")]
pub fn create_ts_parser() -> Result<tree_sitter::Parser, error::ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language()).map_err(|_| error::ParseError::LanguageLoadFailed)?;
    Ok(parser)
}

/// Parse Perl source code
#[cfg(feature = "c-parser")]
pub fn parse(source: &str) -> Result<tree_sitter::Tree, error::ParseError> {
    let mut parser = create_ts_parser()?;
    parser.parse(source, None).ok_or(error::ParseError::ParseFailed)
}

/// Parse Perl source code with existing tree
#[cfg(feature = "c-parser")]
pub fn parse_with_tree(
    source: &str,
    old_tree: Option<&tree_sitter::Tree>,
) -> Result<tree_sitter::Tree, error::ParseError> {
    let mut parser = create_ts_parser()?;
    parser.parse(source, old_tree).ok_or(error::ParseError::ParseFailed)
}

// Rule is available as a type inside pure_rust_parser module when using PerlParser

#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::enhanced_parser::EnhancedPerlParser;

#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::full_parser::FullPerlParser;

#[cfg(feature = "pure-rust")]
pub use perl_ts_advanced_parsers::enhanced_full_parser::EnhancedFullParser;

#[cfg(all(test, feature = "c-parser"))]
mod test {
    use super::*;

    #[test]
    fn test_language_loading() {
        let lang = language();
        // Language is valid if we can get its version
        assert!(lang.abi_version() > 0);
    }

    #[test]
    fn test_basic_parsing() {
        let source = "my $var = 42;";
        let result = parse(source);
        assert!(result.is_ok());

        if let Ok(tree) = result {
            let root = tree.root_node();
            assert_eq!(root.kind(), "source_file");
        }
    }

    #[test]
    fn test_parser_creation() {
        let mut parser = tree_sitter::Parser::new();
        let _ = parser.set_language(&language());
        // Test that parser has a language set
        assert!(parser.language().is_some());
    }
}
