//! Test harness for tree-sitter-perl
//!
//! Provides utilities for parsing Perl code and comparing outputs
//! between C and Rust implementations.

use std::path::Path;
use tree_sitter::{Parser, Tree, Node};

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

/// Serialize tree to S-expression format (matches tree-sitter test format)
pub fn tree_to_sexp(tree: &Tree) -> String {
    let root_node = tree.root_node();
    node_to_sexp(&root_node)
}

/// Convert a node to S-expression format
fn node_to_sexp(node: &Node) -> String {
    let mut result = String::new();
    
    // Add node type
    result.push_str(&format!("({}", node.kind()));
    
    // Add named fields if any
    let mut field_names = Vec::new();
    for i in 0..node.child_count() {
        if let Some(field_name) = node.field_name_for_child(i.try_into().unwrap()) {
            field_names.push((i, field_name));
        }
    }
    
    // Process children
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        
        // Add field name if this child has one
        if let Some((_, field_name)) = field_names.iter().find(|(idx, _)| *idx == i) {
            result.push_str(&format!(" {}: ", field_name));
        } else if i > 0 {
            result.push(' ');
        }
        
        if child.is_named() {
            result.push_str(&node_to_sexp(&child));
        } else {
            // For anonymous nodes, just add the text
            result.push_str(&format!("({})", child.kind()));
        }
    }
    
    result.push(')');
    result
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

/// Compare S-expression outputs
pub fn compare_sexp(tree: &Tree, expected_sexp: &str) -> Result<(), String> {
    let actual_sexp = tree_to_sexp(tree);
    let expected_trimmed = expected_sexp.trim();
    let actual_trimmed = actual_sexp.trim();
    
    if actual_trimmed == expected_trimmed {
        Ok(())
    } else {
        Err(format!(
            "S-expression mismatch:\nExpected: {}\nActual: {}",
            expected_trimmed, actual_trimmed
        ))
    }
}

/// Parse corpus file and extract test cases with expected output
pub fn parse_corpus_test_cases(file_path: &Path) -> Result<Vec<CorpusTestCase>, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file {:?}: {}", file_path, e))?;
    
    let mut test_cases = Vec::new();
    let mut current_test: Option<CorpusTestCase> = None;
    let mut lines = content.lines().peekable();
    
    while let Some(line) = lines.next() {
        if line.starts_with("=") && line.ends_with("=") {
            let test_name = line.trim_matches('=').trim();
            
            // Skip separator lines that are just equals signs
            if test_name.is_empty() {
                continue;
            }
            
            // Start of new test case
            if let Some(test) = current_test.take() {
                test_cases.push(test);
            }
            
            current_test = Some(CorpusTestCase {
                name: test_name.to_string(),
                input: String::new(),
                expected_sexp: String::new(),
            });
        } else if line.starts_with("-") && line.ends_with("-") {
            // Separator between input and expected output
            // Continue reading expected output
        } else if current_test.is_some() {
            let test = current_test.as_mut().unwrap();
            if test.expected_sexp.is_empty() && !line.trim().is_empty() {
                // Still reading input
                if !test.input.is_empty() {
                    test.input.push('\n');
                }
                test.input.push_str(line);
            } else {
                // Reading expected output
                if !test.expected_sexp.is_empty() {
                    test.expected_sexp.push('\n');
                }
                test.expected_sexp.push_str(line);
            }
        }
    }
    
    // Add the last test case
    if let Some(test) = current_test.take() {
        test_cases.push(test);
    }
    
    Ok(test_cases)
}

/// Test case structure for corpus files
#[derive(Debug)]
pub struct CorpusTestCase {
    pub name: String,
    pub input: String,
    pub expected_sexp: String,
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

/// Test all corpus test cases with S-expression validation
pub fn test_corpus_file_with_sexp(file_path: &Path) -> Result<(), String> {
    let test_cases = parse_corpus_test_cases(file_path)?;
    let mut failures = Vec::new();
    
    for test_case in test_cases {
        match parse_perl_code(&test_case.input) {
            Ok(tree) => {
                if tree.root_node().kind() == "ERROR" {
                    failures.push(format!("Test '{}': Parse failed", test_case.name));
                } else if !test_case.expected_sexp.trim().is_empty() {
                    match compare_sexp(&tree, &test_case.expected_sexp) {
                        Ok(()) => (), // Success
                        Err(e) => failures.push(format!("Test '{}': {}", test_case.name, e)),
                    }
                }
            }
            Err(e) => failures.push(format!("Test '{}': Parse error: {}", test_case.name, e)),
        }
    }
    
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("Corpus file {:?} had {} failures:\n{}", 
                   file_path, failures.len(), failures.join("\n")))
    }
}

/// Validate that a tree has no error nodes
pub fn validate_tree_no_errors(tree: &Tree) -> Result<(), String> {
    let mut errors = Vec::new();
    collect_errors(&tree.root_node(), &mut errors);
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Tree contains {} error nodes: {:?}", errors.len(), errors))
    }
}

/// Collect all error nodes in a tree
fn collect_errors(node: &Node, errors: &mut Vec<String>) {
    if node.kind() == "ERROR" {
        errors.push(format!("ERROR at {}:{}", node.start_position().row, node.start_position().column));
    }
    
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_errors(&child, errors);
        }
    }
}

/// Round-trip test: parse → serialize → parse again
pub fn test_round_trip(code: &str) -> Result<(), String> {
    let tree1 = parse_perl_code(code)?;
    let _sexp = tree_to_sexp(&tree1);
    
    // Parse the S-expression back (this is a basic test)
    // In a real implementation, you'd want to deserialize the S-expression
    // For now, we just validate the original tree is valid
    validate_tree_no_errors(&tree1)?;
    
    Ok(())
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

    #[test]
    fn test_sexp_serialization() {
        let code = "print 'Hello';";
        let tree = parse_perl_code(code).unwrap();
        let sexp = tree_to_sexp(&tree);
        
        // Basic validation that we get a valid S-expression
        assert!(sexp.contains("source_file"), "S-expression should contain source_file: {}", sexp);
        // The function name might be represented as (function) in the AST
        assert!(sexp.contains("function") || sexp.contains("print"), "S-expression should contain function or print: {}", sexp);
    }

    #[test]
    fn test_round_trip_validation() {
        let code = "my $x = 42;";
        let result = test_round_trip(code);
        assert!(result.is_ok(), "Round-trip test failed: {:?}", result);
    }

    #[test]
    fn test_error_validation() {
        let code = "print 'Hello';"; // Valid code
        let tree = parse_perl_code(code).unwrap();
        let result = validate_tree_no_errors(&tree);
        assert!(result.is_ok(), "Valid code should have no errors: {:?}", result);
    }
} 