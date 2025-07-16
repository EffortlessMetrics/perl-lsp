//! Tree-sitter Perl parser with C scanner implementation
//!
//! This crate provides the legacy C implementation of the tree-sitter Perl parser
//! for comparison and benchmarking purposes.

use tree_sitter::{Language, Parser};
use std::path::Path;

/// Get the tree-sitter Perl language
pub fn language() -> Language {
    unsafe { tree_sitter_perl() }
}

/// Create a new parser with C scanner
pub fn create_parser() -> Parser {
    let mut parser = Parser::new();
    parser.set_language(language()).unwrap();
    parser
}

/// Parse Perl code using the C scanner
pub fn parse_perl_code(code: &str) -> Result<tree_sitter::Tree, tree_sitter::ParseError> {
    let mut parser = create_parser();
    parser.parse(code, None)
}

/// Parse a Perl file using the C scanner
pub fn parse_perl_file<P: AsRef<Path>>(path: P) -> Result<tree_sitter::Tree, Box<dyn std::error::Error>> {
    let code = std::fs::read_to_string(path)?;
    Ok(parse_perl_code(&code)?)
}

/// Get the scanner configuration for the C implementation
pub fn get_scanner_config() -> &'static str {
    "c-scanner"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_loading() {
        let lang = language();
        assert_eq!(lang.node_kind_count(), 40); // Expected node types
    }

    #[test]
    fn test_basic_parsing() {
        let code = "my $var = 'hello';";
        let tree = parse_perl_code(code).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parser_creation() {
        let parser = create_parser();
        assert!(parser.language().is_some());
    }
}

// External C function declarations
extern "C" {
    fn tree_sitter_perl() -> Language;
} 