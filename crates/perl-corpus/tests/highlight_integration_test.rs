//! Highlight integration tests for perl-corpus
//!
//! This module tests the integration between the tree-sitter highlight test runner
//! and the perl-corpus test infrastructure, following architectural patterns from
//! the existing LSP test infrastructure.

use perl_parser::{Node, NodeKind, Parser};
use std::collections::HashMap;

/// Test the highlight test runner integration with perl-corpus test patterns
#[test]
fn test_highlight_runner_integration() {
    // Test basic Perl constructs that should highlight correctly
    let test_cases = vec![
        ("my $x = 42;", vec!["VariableDeclaration", "Variable", "number"]),
        ("\"hello world\"", vec!["string"]),
        ("1 + 2", vec!["number", "binary_+", "number"]),
    ];

    for (source, expected_kinds) in test_cases {
        let mut parser = Parser::new(source);
        use perl_tdd_support::must;
        let ast = must(parser.parse());

        let mut actual_kinds = HashMap::new();
        collect_node_kinds(&ast, &mut actual_kinds);

        // Verify that all expected node kinds are present
        for expected_kind in &expected_kinds {
            assert!(
                actual_kinds.contains_key(*expected_kind),
                "Expected node kind '{}' not found in AST. Available: {:?}",
                expected_kind,
                actual_kinds.keys().collect::<Vec<_>>()
            );
        }
    }
}

/// Test complex Perl constructs for highlight coverage
#[test]
fn test_complex_highlight_constructs() {
    let test_cases = vec![
        // Variable declarations with different sigils
        ("my $scalar = 'test';", "Variable"),
        ("our @array = (1, 2, 3);", "Variable"),
        ("my %hash = (key => 'value');", "Variable"),
        // Function calls
        ("print 'hello';", "FunctionCall"),
        ("chomp($line);", "FunctionCall"),
        // Use statements
        ("use strict;", "UseStatement"),
        ("use warnings;", "UseStatement"),
    ];

    for (source, expected_primary_kind) in test_cases {
        let mut parser = Parser::new(source);
        use perl_tdd_support::must;
        let ast = must(parser.parse());

        let mut actual_kinds = HashMap::new();
        collect_node_kinds(&ast, &mut actual_kinds);

        assert!(
            actual_kinds.contains_key(expected_primary_kind),
            "Expected primary kind '{}' not found in '{}'. Available: {:?}",
            expected_primary_kind,
            source,
            actual_kinds.keys().collect::<Vec<_>>()
        );
    }
}

/// Test error handling in highlight parsing
#[test]
fn test_highlight_error_handling() {
    // Test various edge cases that might cause parsing issues
    let edge_cases = vec![
        "",                         // Empty input
        "   ",                      // Whitespace only
        "#",                        // Comment only
        "# This is just a comment", // Comment with text
    ];

    for source in edge_cases {
        let mut parser = Parser::new(source);
        // These should parse without panicking, even if they produce minimal ASTs
        let result = parser.parse();
        assert!(
            result.is_ok() || source.trim().is_empty(),
            "Parser should handle edge case gracefully: '{}'",
            source
        );

        if let Ok(ast) = result {
            let mut actual_kinds = HashMap::new();
            collect_node_kinds(&ast, &mut actual_kinds);
            // Should at least have a Program node
            assert!(
                actual_kinds.contains_key("Program"),
                "Should always have Program node, got: {:?}",
                actual_kinds
            );
        }
    }
}

/// Test performance characteristics of highlight parsing
#[test]
fn test_highlight_performance() {
    use std::time::Instant;

    // Test performance with moderately sized Perl code (simplified for parser compatibility)
    let large_source = r#"
use strict;
use warnings;

my $count = 0;
my @items = ("one", "two", "three", "four", "five");
my %lookup = ("key1", "value1", "key2", "value2", "key3", "value3");

my $item = "test";
if ($count > 0) {
    print "Found: $item";
    $count = $count + 1;
} else {
    print "Not found: $item";
}

sub process_data {
    my $data = "test";
    my $result = "";
    return $result;
}

print "Processed items";
"#;

    let start = Instant::now();
    let mut parser = Parser::new(large_source);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    let mut actual_kinds = HashMap::new();
    collect_node_kinds(&ast, &mut actual_kinds);
    let duration = start.elapsed();

    // Should complete parsing and AST traversal quickly (< 100ms for this size)
    assert!(duration.as_millis() < 100, "Highlight parsing took too long: {:?}", duration);

    // Should find multiple node types in complex code
    assert!(
        actual_kinds.len() >= 5,
        "Should find multiple node types in complex code, got: {:?}",
        actual_kinds.len()
    );
}

/// Recursively collect all node kinds from the AST
/// This mirrors the implementation in the xtask highlight runner
fn collect_node_kinds(node: &Node, scopes: &mut HashMap<String, usize>) {
    let kind_name = match &node.kind {
        NodeKind::Program { .. } => "Program",
        NodeKind::ExpressionStatement { .. } => "ExpressionStatement",
        NodeKind::VariableDeclaration { .. } => "VariableDeclaration",
        NodeKind::Variable { .. } => "Variable",
        NodeKind::Binary { op, .. } => {
            // Return specific binary operator types
            match op.as_str() {
                "+" => "binary_+",
                "-" => "binary_-",
                "*" => "binary_*",
                "/" => "binary_/",
                "==" => "binary_==",
                "=~" => "binary_=~",
                "=>" => "binary_=>",
                _ => "binary_op",
            }
        }
        NodeKind::Number { .. } => "number",
        NodeKind::String { .. } => "string",
        NodeKind::Assignment { .. } => "Assignment",
        NodeKind::Subroutine { .. } => "SubDeclaration",
        NodeKind::Use { .. } => "UseStatement",
        NodeKind::FunctionCall { .. } => "FunctionCall",
        NodeKind::Heredoc { .. } => "HereDoc",
        _ => "other", // Fallback for other node types
    }
    .to_string();

    *scopes.entry(kind_name).or_insert(0) += 1;

    // Manually recurse into child nodes based on NodeKind
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                collect_node_kinds(stmt, scopes);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            collect_node_kinds(expression, scopes);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            collect_node_kinds(variable, scopes);
            if let Some(init) = initializer {
                collect_node_kinds(init, scopes);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            collect_node_kinds(left, scopes);
            collect_node_kinds(right, scopes);
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            collect_node_kinds(lhs, scopes);
            collect_node_kinds(rhs, scopes);
        }
        NodeKind::Subroutine { body, .. } => {
            collect_node_kinds(body, scopes);
        }
        NodeKind::Use { .. } => {
            // Note: module is a String, not a Node
        }
        NodeKind::FunctionCall { args, .. } => {
            // Note: name is a String, not a Node
            for arg in args {
                collect_node_kinds(arg, scopes);
            }
        }
        _ => {}
    }
}
