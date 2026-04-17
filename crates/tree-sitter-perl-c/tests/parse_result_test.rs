//! Tests for ParseResult and parse_perl_summary
//!
//! These tests define the expected behavior of the ParseResult struct
//! and parse_perl_summary convenience function.

use tree_sitter_perl_c::parse_perl_summary;

/// Test that parse_perl_summary returns correct fields for valid Perl code.
#[test]
fn test_parse_perl_summary_valid_code() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("my $x = 42;")?;

    // has_errors should be false for valid code
    assert!(!result.has_errors, "expected no errors for valid Perl code");

    // root_kind should be "source_file"
    assert_eq!(
        result.root_kind, "source_file",
        "expected root_kind to be 'source_file', got '{}'",
        result.root_kind
    );

    // grammar_node_kind_count should be a positive integer (grammar constant > 0)
    assert!(
        result.grammar_node_kind_count > 0,
        "expected grammar_node_kind_count > 0, got {}",
        result.grammar_node_kind_count
    );

    // sexp should start with "(source_file"
    assert!(
        result.sexp.starts_with("(source_file"),
        "expected sexp to start with '(source_file', got first 50 chars: '{}'",
        &result.sexp[..result.sexp.len().min(50)]
    );

    // tree escape hatch - root_node().kind() should equal root_kind
    assert_eq!(
        result.tree.root_node().kind(),
        result.root_kind,
        "tree.root_node().kind() should equal root_kind"
    );

    Ok(())
}

/// Test that parse_perl_summary returns has_errors=true for invalid Perl code.
#[test]
fn test_parse_perl_summary_invalid_code() -> Result<(), Box<dyn std::error::Error>> {
    // This is invalid Perl - missing RHS of assignment
    let result = parse_perl_summary("my $x = ;")?;

    // has_errors should be true for invalid code
    assert!(result.has_errors, "expected has_errors to be true for invalid Perl code");

    Ok(())
}

/// Test that the tree escape hatch works correctly.
#[test]
fn test_parse_perl_summary_tree_escape_hatch() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("sub foo { 42 }")?;

    // tree.root_node().kind() should equal "source_file"
    assert_eq!(
        result.tree.root_node().kind(),
        "source_file",
        "expected tree.root_node().kind() to be 'source_file', got '{}'",
        result.tree.root_node().kind()
    );

    // tree should be usable for further operations
    let root = result.tree.root_node();
    assert!(!root.has_error(), "expected root node to not have error for valid code");

    Ok(())
}

// =============================================================================
// Edge case tests — green-test-builder coverage beyond red tests
// =============================================================================

/// Test that parse_perl_summary handles empty string input.
/// Empty input is valid Perl (produces an empty source_file).
#[test]
fn test_parse_perl_summary_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("")?;

    // Empty input should have no errors
    assert!(!result.has_errors, "expected no errors for empty string");
    // root_kind should still be "source_file"
    assert_eq!(
        result.root_kind, "source_file",
        "expected root_kind to be 'source_file' for empty input, got '{}'",
        result.root_kind
    );
    // grammar_node_kind_count should still be positive
    assert!(
        result.grammar_node_kind_count > 0,
        "expected grammar_node_kind_count > 0 for empty input"
    );
    // sexp should be "(source_file)"
    assert_eq!(
        result.sexp, "(source_file)",
        "expected sexp to be '(source_file)' for empty input, got '{}'",
        result.sexp
    );

    Ok(())
}

/// Test that parse_perl_summary handles whitespace-only input.
/// Whitespace-only input is valid Perl.
#[test]
fn test_parse_perl_summary_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("   \n\t  \r\n")?;

    // Whitespace-only should have no errors
    assert!(!result.has_errors, "expected no errors for whitespace-only input");
    assert_eq!(
        result.root_kind, "source_file",
        "expected root_kind to be 'source_file' for whitespace-only input"
    );

    Ok(())
}

/// Test that parse_perl_summary handles multiple statements.
#[test]
fn test_parse_perl_summary_multiple_statements() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("my $x = 1; my $y = 2; my $z = 3;")?;

    assert!(!result.has_errors, "expected no errors for multiple statements");
    assert_eq!(result.root_kind, "source_file");
    // sexp should reflect multiple expression statements
    // Note: tree-sitter sexp shows structure (scalar, varname) not $x literal text
    assert!(
        result.sexp.contains("scalar"),
        "expected sexp to contain 'scalar' node for scalar variables, got first 100 chars: '{}'",
        &result.sexp[..result.sexp.len().min(100)]
    );
    assert!(
        result.sexp.contains("assignment_expression"),
        "expected sexp to contain 'assignment_expression'"
    );

    Ok(())
}

/// Test that parse_perl_summary handles code with Perl comments.
#[test]
fn test_parse_perl_summary_with_comments() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("# this is a comment\nmy $x = 42; # end\n")?;

    assert!(!result.has_errors, "expected no errors for code with comments");
    assert_eq!(result.root_kind, "source_file");
    // tree-sitter sexp shows structure, not $x literal
    assert!(
        result.sexp.contains("scalar"),
        "expected sexp to contain 'scalar' node even with comments present"
    );

    Ok(())
}

/// Test that parse_perl_summary handles package and use statements.
#[test]
fn test_parse_perl_summary_package_and_use() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("package My::Package;\nuse strict;\nuse warnings;\n1;\n")?;

    assert!(!result.has_errors, "expected no errors for package and use statements");
    assert_eq!(result.root_kind, "source_file");
    // The sexp should contain package and use statements
    assert!(result.sexp.contains("package"), "expected sexp to contain 'package'");

    Ok(())
}

/// Test that parse_perl_summary handles array and hash references.
#[test]
fn test_parse_perl_summary_complex_data_structures() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary(
        "my @arr = (1, 2, 3);\nmy %hash = (a => 1, b => 2);\nmy $ref = \\@arr;\n",
    )?;

    assert!(!result.has_errors, "expected no errors for complex data structures");
    assert_eq!(result.root_kind, "source_file");
    // sexp should contain array and hash node types (tree-sitter uses structural node types)
    assert!(
        result.sexp.contains("array"),
        "expected sexp to contain 'array' node type, got: {}",
        result.sexp
    );

    Ok(())
}

/// Test that grammar_node_kind_count is a consistent grammar-level constant.
/// It should be the same across all parse results.
#[test]
fn test_parse_perl_summary_grammar_node_kind_count_consistency()
-> Result<(), Box<dyn std::error::Error>> {
    let inputs = [
        "",
        "my $x = 42;",
        "sub foo { 42 }",
        "package My::Package;\nuse strict;\n1;\n",
        "# comment\nmy @arr = (1, 2, 3);",
    ];

    let counts: Vec<usize> = inputs
        .iter()
        .map(|code| parse_perl_summary(code).map(|r| r.grammar_node_kind_count))
        .collect::<Result<Vec<_>, _>>()?;

    // All counts should be identical (grammar-level constant)
    let first = counts[0];
    for (i, count) in counts.iter().enumerate() {
        assert_eq!(
            *count, first,
            "grammar_node_kind_count should be consistent across all parses; \
             input '{}' had {} but expected {}",
            inputs[i], count, first
        );
    }

    // And greater than 0
    assert!(first > 0, "grammar_node_kind_count should be > 0");

    Ok(())
}

/// Test that sexp field actually reflects the parsed content.
/// The sexp should contain nodes that correspond to the input code.
#[test]
fn test_parse_perl_summary_sexp_content_verification() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("my $scalar = 'hello';\n")?;

    // The sexp should contain string_literal node type
    // Note: tree-sitter sexp shows structure, not literal text content
    assert!(
        result.sexp.contains("string_literal"),
        "expected sexp to contain 'string_literal' node type, got: {}",
        result.sexp
    );

    // The sexp should contain scalar variable declaration
    assert!(
        result.sexp.contains("scalar"),
        "expected sexp to contain 'scalar' node type, got: {}",
        result.sexp
    );

    Ok(())
}

/// Test that tree escape hatch allows full tree-sitter API access.
/// Verify we can walk the tree and access child nodes.
#[test]
fn test_parse_perl_summary_tree_walk() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_perl_summary("my ($x, $y) = @_;\n")?;

    // Use the tree escape hatch to walk the tree
    let root = result.tree.root_node();
    assert_eq!(root.kind(), "source_file");

    // Root should have children (the my statement)
    let child_count = root.child_count();
    assert!(child_count > 0, "expected root node to have children, got {}", child_count);

    // We should be able to get a child by index
    if child_count > 0 {
        if let Some(first_child) = root.child(0) {
            // Child should have a kind
            assert!(!first_child.kind().is_empty(), "expected first child to have a kind");
        }
    }

    Ok(())
}
