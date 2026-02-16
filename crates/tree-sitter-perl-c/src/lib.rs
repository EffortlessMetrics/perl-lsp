//! Tree-sitter Perl parser with C scanner implementation
//!
//! This crate provides the legacy C implementation of the tree-sitter Perl parser
//! for comparison and benchmarking purposes.

// Suppress warnings from bindgen-generated code
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(dead_code)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::path::Path;
use tree_sitter::{Language, Parser};

/// Get the tree-sitter Perl language
pub fn language() -> Language {
    unsafe { tree_sitter_perl() }
}

/// Create a new parser with C scanner
pub fn try_create_parser() -> Result<Parser, tree_sitter::LanguageError> {
    let mut parser = Parser::new();
    parser.set_language(&language())?;
    Ok(parser)
}

/// Create a new parser with C scanner, failing silently if language cannot be set
/// (Legacy API for compatibility)
pub fn create_parser() -> Parser {
    let mut parser = Parser::new();
    let _ = parser.set_language(&language());
    parser
}

/// Parse Perl code using the C scanner
pub fn parse_perl_code(code: &str) -> Result<tree_sitter::Tree, Box<dyn std::error::Error>> {
    let mut parser = try_create_parser()?;
    match parser.parse(code, None) {
        Some(tree) => Ok(tree),
        None => Err("Failed to parse code".into()),
    }
}

/// Parse a Perl file using the C scanner
pub fn parse_perl_file<P: AsRef<Path>>(
    path: P,
) -> Result<tree_sitter::Tree, Box<dyn std::error::Error>> {
    let code = std::fs::read_to_string(path)?;
    parse_perl_code(&code)
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
        let count = lang.node_kind_count();
        println!("C implementation node kind count: {}", count);
        // Language is valid if we can get its node kind count
        assert!(count > 0);
    }

    #[test]
    fn test_basic_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $var = 'hello';";
        let tree = parse_perl_code(code)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parser_creation() {
        let parser = create_parser();
        assert!(parser.language().is_some());
    }
}

// External C function declarations
unsafe extern "C" {
    fn tree_sitter_perl() -> Language;
}
