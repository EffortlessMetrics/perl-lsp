//! Tree-sitter Perl grammar with Rust-native scanner
//!
//! This crate provides a Perl grammar for tree-sitter with a high-performance
//! Rust-native scanner implementation. It supports both C and Rust scanner backends
//! for maximum compatibility and performance.
//!
//! ## Features
//!
//! - **rust-scanner** (default): Use the Rust-native scanner implementation
//! - **c-scanner**: Use the legacy C scanner implementation  
//! - **test-utils**: Include testing utilities and test data
//!
//! ## Usage
//!
//! ```rust
//! use tree_sitter_perl::{parse, language};
//! use tree_sitter::Parser;
//!
//! let mut parser = Parser::new();
//! parser.set_language(&language()).unwrap();
//!
//! let source_code = "my $var = 42; print 'Hello, World!';";
//! let tree = parser.parse(source_code, None).unwrap();
//!
//! println!("{}", tree.root_node().to_sexp());
//! ```

pub mod error;
pub mod scanner;
pub mod unicode;

#[cfg(feature = "pure-rust")]
pub mod pure_rust_parser;

#[cfg(feature = "pure-rust")]
pub mod iterative_parser;

#[cfg(all(feature = "pure-rust", test))]
pub mod parser_benchmark;

#[cfg(feature = "pure-rust")]
pub mod context_aware_parser;

#[cfg(feature = "pure-rust")]
pub mod runtime_heredoc_handler;

#[cfg(feature = "pure-rust")]
pub mod anti_pattern_detector;

#[cfg(feature = "pure-rust")]
pub mod partial_parse_ast;

#[cfg(feature = "pure-rust")]
pub mod understanding_parser;

#[cfg(feature = "pure-rust")]
pub mod phase_aware_parser;

#[cfg(feature = "pure-rust")]
pub mod dynamic_delimiter_recovery;

#[cfg(feature = "pure-rust")]
pub mod edge_case_handler;

#[cfg(all(test, feature = "pure-rust"))]
mod pure_rust_parser_tests;

#[cfg(all(test, feature = "pure-rust"))]
mod fuzz_tests;

#[cfg(feature = "pure-rust")]
pub mod stateful_parser;

#[cfg(feature = "pure-rust")]
pub mod context_sensitive;

#[cfg(feature = "pure-rust")]
pub mod enhanced_parser;

#[cfg(feature = "pure-rust")]
pub mod perl_lexer;

#[cfg(feature = "pure-rust")]
pub mod lexer_adapter;

#[cfg(feature = "pure-rust")]
pub mod disambiguated_parser;

#[cfg(all(test, feature = "pure-rust"))]
mod test_slash;

#[cfg(feature = "pure-rust")]
pub mod pratt_parser;

#[cfg(feature = "pure-rust")]
pub mod heredoc_parser;

#[cfg(feature = "pure-rust")]
pub mod full_parser;

#[cfg(feature = "pure-rust")]
mod statement_tracker;

#[cfg(any(feature = "pure-rust", feature = "test-utils"))]
pub mod comparison_harness;

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "pure-rust"))]
mod test_format;

use tree_sitter::{Language, Parser};

// External C functions from the generated parser
unsafe extern "C" {
    fn tree_sitter_perl() -> *const tree_sitter::ffi::TSLanguage;
}

/// Get the tree-sitter language for Perl
pub fn language() -> Language {
    unsafe { Language::from_raw(tree_sitter_perl()) }
}

/// Create a new parser instance
pub fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("Failed to set language");
    parser
}

/// Parse Perl source code
pub fn parse(source: &str) -> Result<tree_sitter::Tree, error::ParseError> {
    let mut parser = parser();
    parser
        .parse(source, None)
        .ok_or(error::ParseError::ParseFailed)
}

/// Parse Perl source code with existing tree
pub fn parse_with_tree(
    source: &str,
    old_tree: Option<&tree_sitter::Tree>,
) -> Result<tree_sitter::Tree, error::ParseError> {
    let mut parser = parser();
    parser
        .parse(source, old_tree)
        .ok_or(error::ParseError::ParseFailed)
}

#[cfg(feature = "pure-rust")]
pub use pure_rust_parser::{PureRustPerlParser, AstNode, PerlParser};
// Rule is available as a type inside pure_rust_parser module when using PerlParser

#[cfg(feature = "pure-rust")]
pub use enhanced_parser::EnhancedPerlParser;

#[cfg(feature = "pure-rust")]
pub use full_parser::FullPerlParser;

#[cfg(test)]
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

        let tree = result.unwrap();
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");
    }

    #[test]
    fn test_parser_creation() {
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        // Test that parser has a language set
        assert!(parser.language().is_some());
    }
}
