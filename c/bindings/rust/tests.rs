//! Comprehensive Rust-side test suite for tree-sitter-perl
//!
//! This module orchestrates all scanner, unicode, property, and integration tests.
//! It is designed to mirror the C-based test suite and ensure 100% input/output fidelity.

mod test_harness;
mod simple_test;
mod unit_scanner;
mod unit_unicode;
mod property_scanner;
mod integration_corpus;
mod integration_highlight;
mod performance;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness::parse_perl_code;

    #[test]
    fn test_basic_parsing_works() {
        let code = "print 'Hello, World!';";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse basic Perl code: {:?}", result);
    }

    #[test]
    fn test_empty_string_parses() {
        let code = "";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse empty string: {:?}", result);
    }

    #[test]
    fn test_variable_declaration_parses() {
        let code = "my $var = 42;";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse variable declaration: {:?}", result);
    }
} 