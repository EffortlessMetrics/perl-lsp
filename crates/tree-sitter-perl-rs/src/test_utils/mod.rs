//! Test utilities for the Perl parser
//!
//! This module provides utilities for testing the parser, including
//! test data generation, validation functions, and comparison tools.

use crate::error::ParseResult;
use tree_sitter::{Parser, Tree};

/// Test utilities for Perl parsing
pub struct TestUtils;

impl TestUtils {
    /// Parse Perl code and return the tree
    pub fn parse_perl_code(code: &str) -> ParseResult<Tree> {
        let mut parser = Parser::new();
        parser.set_language(&crate::language()).map_err(|_| crate::error::ParseError::LanguageLoadFailed)?;
        parser.parse(code, None).ok_or(crate::error::ParseError::ParseFailed)
    }

    /// Validate that a tree has no error nodes
    pub fn validate_tree_no_errors(tree: &Tree) -> ParseResult<()> {
        let mut has_errors = false;
        Self::collect_errors(&tree.root_node(), &mut has_errors);

        if has_errors {
            Err(crate::error::ParseError::scanner_error_simple("Tree contains error nodes"))
        } else {
            Ok(())
        }
    }

    /// Collect error nodes from a tree
    fn collect_errors(node: &tree_sitter::Node, has_errors: &mut bool) {
        if node.kind() == "ERROR" {
            *has_errors = true;
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::collect_errors(&child, has_errors);
            }
        }
    }

    /// Convert a tree to S-expression format
    pub fn tree_to_sexp(tree: &Tree) -> String {
        tree.root_node().to_sexp()
    }

    /// Compare two trees and return differences
    pub fn compare_trees(tree1: &Tree, tree2: &Tree) -> Vec<String> {
        let sexp1 = Self::tree_to_sexp(tree1);
        let sexp2 = Self::tree_to_sexp(tree2);

        if sexp1 == sexp2 {
            vec![]
        } else {
            vec![format!("Trees differ:\nExpected: {}\nActual: {}", sexp1, sexp2)]
        }
    }

    /// Generate test data for property-based testing
    pub fn generate_test_data() -> Vec<String> {
        vec![
            "my $var = 42;".to_string(),
            "print 'Hello, World!';".to_string(),
            "sub foo { return 1; }".to_string(),
            "if ($x) { $y = 1; }".to_string(),
            "for my $i (1..10) { print $i; }".to_string(),
            "my @array = (1, 2, 3);".to_string(),
            "my %hash = (key => 'value');".to_string(),
            "# This is a comment\nmy $var = 1;".to_string(),
            r#"my $str = "Hello\nWorld";"#.to_string(),
            "my $regex = qr/pattern/;".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;

    #[test]
    fn test_parse_perl_code() {
        let result = TestUtils::parse_perl_code("my $var = 42;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tree_no_errors() {
        let tree = must(TestUtils::parse_perl_code("my $var = 42;"));
        let result = TestUtils::validate_tree_no_errors(&tree);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tree_to_sexp() {
        let tree = must(TestUtils::parse_perl_code("my $var = 42;"));
        let sexp = TestUtils::tree_to_sexp(&tree);
        assert!(sexp.contains("source_file"));
    }
}
