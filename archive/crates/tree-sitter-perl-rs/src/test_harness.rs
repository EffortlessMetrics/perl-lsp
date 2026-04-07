//! Test harness for tree-sitter-perl
//!
//! Provides utilities for parsing Perl code and comparing outputs
//! between C and Rust implementations.

use std::path::Path;
use tree_sitter::{Parser, Tree};

/// Parse Perl code using the current implementation (C scanner)
pub fn parse_perl_code(code: &str) -> Result<Tree, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::language())
        .map_err(|e| format!("Failed to set language: {:?}", e))?;
    
    parser
        .parse(code, None)
        .ok_or_else(|| "Failed to parse code".to_string())
}

/// Parse a corpus file and return the parse tree
pub fn parse_corpus_file(file_path: &Path) -> Result<Tree, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file {:?}: {}", file_path, e))?;
    
    parse_perl_code(&content)
}

/// Get the string representation of a parse tree
pub fn tree_to_string(tree: &Tree) -> String {
    let root_node = tree.root_node();
    format!("{:?}", root_node)
}

/// Compare two parse trees and return differences
pub fn compare_trees(tree1: &Tree, tree2: &Tree) -> Vec<String> {
    let str1 = tree_to_string(tree1);
    let str2 = tree_to_string(tree2);
    
    if str1 == str2 {
        vec![]
    } else {
        vec![format!("Trees differ:\nExpected: {}\nActual: {}", str1, str2)]
    }
}

/// Test that a corpus file parses successfully
pub fn test_corpus_file_parses(file_path: &Path) -> Result<(), String> {
    let tree = parse_corpus_file(file_path)?;
    
    // Basic validation: ensure we got a valid tree
    if tree.root_node().kind() == "ERROR" {
        return Err(format!("File {:?} failed to parse", file_path));
    }
    
    Ok(())
}

/// Test that a corpus file produces the expected output
pub fn test_corpus_file_matches_expected(file_path: &Path, expected_output: &str) -> Result<(), String> {
    let tree = parse_corpus_file(file_path)?;
    let actual_output = tree_to_string(&tree);
    
    if actual_output == expected_output {
        Ok(())
    } else {
        Err(format!(
            "Output mismatch for {:?}:\nExpected: {}\nActual: {}",
            file_path, expected_output, actual_output
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_perl() {
        let code = "print 'Hello, World!';";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse simple Perl code: {:?}", result);
    }

    #[test]
    fn test_parse_empty_string() {
        let code = "";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse empty string: {:?}", result);
    }

    #[test]
    fn test_parse_complex_expression() {
        let code = "my $result = $a + $b * $c;";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse complex expression: {:?}", result);
    }
} 