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

#[cfg(any(feature = "pure-rust", feature = "test-utils"))]
pub mod comparison_harness;

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(test)]
mod tests;

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

#[cfg(feature = "pure-rust")]
pub use pure_rust_parser::{PureRustPerlParser, AstNode};
