//! Hash slice integration tests (work-e5278c16)
//!
//! Integration tests that exercise the full parsing pipeline:
//! Source → Lexer → TokenStream → Parser → AST
//!
//! These tests focus on:
//! 1. Component handoffs - verifying data flows correctly between components
//! 2. Multi-component flows - complex expressions involving multiple operators
//! 3. Error propagation - how parse errors are reported through the system
//! 4. Full workflow - complete Perl statements containing hash slices
//!
//! Unlike unit tests that focus on individual functions, these tests verify
//! that the complete pipeline works end-to-end.

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_ast::Node;
use perl_parser_core::NodeKind;

// =============================================================================
// Integration Test 1: Full Pipeline - Source to AST
// Verify that hash slice source code flows correctly through the entire pipeline
// =============================================================================

/// Test: Full pipeline for simple hash slice
/// Flow: Source → Lexer → TokenStream → Parser → PostfixChain → AST
///
/// Verifies that a simple hash slice expression produces a clean AST with
/// the correct node structure.
#[test]
fn integration_simple_hash_slice_full_pipeline() {
    let source = r#"%hash{key1, key2}"#;
    let ast = parse(source);

    // Should have no error nodes anywhere in the tree
    assert_clean_parse(source);

    // Verify the structure: should be a Binary node with {} operator
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("binary_{}"),
        "Hash slice should produce binary_{{}} node, got: {}",
        sexp
    );
}

/// Test: Full pipeline for array hash-slice alias
/// Flow: Source → Lexer → TokenStream → Parser → PostfixChain → AST
///
/// Verifies that @hash{...} (Perl alias for hash slice) also works correctly.
#[test]
fn integration_at_hash_slice_full_pipeline() {
    let source = r#"@hash{key1, key2}"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("binary_{}"),
        "@hash slice should produce binary_{{}} node, got: {}",
        sexp
    );
}

/// Test: Full pipeline for complex hash slice from CPAN
/// Flow: Source → Lexer → TokenStream → Parser → PostfixChain → AST
///
/// This is the exact pattern that was failing before the fix:
/// `@ops_seen{ map split(/ /), values %ops }`
#[test]
fn integration_complex_hash_slice_from_corpus() {
    let source = r#"@ops_seen{ map split(/ /), values %ops }"#;
    let ast = parse(source);

    // This was producing unexpected_comma_expr errors before the fix
    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // The map expression should be inside the hash slice key
    assert!(
        sexp.contains("map"),
        "Complex hash slice should contain map expression, got: {}",
        sexp
    );
}

// =============================================================================
// Integration Test 2: Component Handoffs - Parser to Postfix Chain
// Verify that the postfix chain correctly handles the base expression and
// produces the correct node type
// =============================================================================

/// Test: Hash slice node structure verification
/// Verifies that the parser produces the expected Binary node with {} operator
#[test]
fn integration_hash_slice_node_structure() {
    let source = r#"%hash{key}"#;
    let ast = parse(source);

    // Walk the AST to find the binary node
    let found_binary = find_binary_node_with_op(&ast, "{}");
    assert!(found_binary.is_some(), "Should find binary {{}} node in AST for: {}", ast.to_sexp());

    let binary = found_binary.unwrap();
    // Left child should be a Variable with % sigil
    match &binary.kind {
        NodeKind::Binary { left, right, .. } => {
            match &left.kind {
                NodeKind::Variable { sigil, name } => {
                    assert_eq!(sigil, "%", "Left child should be % variable");
                    assert_eq!(name, "hash", "Variable name should be 'hash'");
                }
                _ => panic!("Left child should be Variable, got: {:?}", left.kind),
            }
            // Right child is the key (identifier)
            assert!(
                matches!(&right.kind, NodeKind::Identifier { .. }),
                "Right child should be identifier, got: {:?}",
                right.kind
            );
        }
        _ => panic!("Should be Binary node, got: {:?}", binary.kind),
    }
}

/// Test: @hash slice node structure (Perl alias)
/// Verifies that @hash{...} produces the same node structure as %hash{...}
#[test]
fn integration_at_hash_slice_node_structure() {
    let source = r#"@hash{key}"#;
    let ast = parse(source);

    let found_binary = find_binary_node_with_op(&ast, "{}");
    assert!(found_binary.is_some(), "Should find binary {{}} node in AST for: {}", ast.to_sexp());

    let binary = found_binary.unwrap();
    match &binary.kind {
        NodeKind::Binary { left, .. } => match &left.kind {
            NodeKind::Variable { sigil, name } => {
                assert_eq!(sigil, "@", "Left child should be @ variable");
                assert_eq!(name, "hash", "Variable name should be 'hash'");
            }
            _ => panic!("Left child should be Variable, got: {:?}", left.kind),
        },
        _ => panic!("Should be Binary node, got: {:?}", binary.kind),
    }
}

// =============================================================================
// Integration Test 3: Multi-Component Flows - Chained Operations
// Test complex expressions involving hash slices combined with other operators
// =============================================================================

/// Test: Hash slice chained with method call
/// Flow: parse_postfix_chain handles %hash{key} then ->method()
///
/// When you have `%hash{key}->method()`, the parser should:
/// 1. Parse %hash{key} as a binary {} operation
/// 2. Then parse ->method() as a method call on the result
#[test]
fn integration_hash_slice_chained_method() {
    let source = r#"%hash{key}->method()"#;
    let ast = parse(source);

    assert_clean_parse(source);

    // Should have method call
    let sexp = ast.to_sexp();
    assert!(sexp.contains("method_call"), "Should have method_call node, got: {}", sexp);
}

/// Test: Arrow deref then hash slice
/// Flow: $ref->{key} still works via Arrow arm
///
/// This should NOT be affected by our change because it goes through
/// the Arrow arm which is checked before our new LeftBrace arm.
#[test]
fn integration_arrow_deref_then_hash_slice() {
    let source = r#"$ref->{key}"#;
    let ast = parse(source);

    assert_clean_parse(source);

    // Should parse correctly - arrow hash deref uses arrow_hash_deref node
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("arrow_hash_deref"),
        "Arrow hash deref should produce arrow_hash_deref node, got: {}",
        sexp
    );
}

/// Test: Array index then hash slice (multi-dimensional access)
/// Flow: @array[$i]{key} parses @array[$i] first, then {key} on result
///
/// This tests that our LeftBrace arm works correctly on expressions
/// that are already postfix chains.
#[test]
fn integration_array_index_then_hash_slice() {
    let source = r#"@array[$i]{key1, key2}"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should have both array subscript and hash slice
    assert!(
        sexp.contains("binary_[]") && sexp.contains("binary_{}"),
        "Should have both array subscript and hash slice, got: {}",
        sexp
    );
}

/// Test: Hash slice then array index (hash of arrays pattern)
/// Flow: %hash{key}[0, 2] parses %hash{key} first, then [0, 2] on result
#[test]
fn integration_hash_slice_then_array_index() {
    let source = r#"%hash{key}[0, 2]"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should have both hash slice and array subscript
    assert!(
        sexp.contains("binary_{}") && sexp.contains("binary_[]"),
        "Should have both hash slice and array subscript, got: {}",
        sexp
    );
}

/// Test: Nested arrow dereference with hash slice
/// Flow: $a->[0]->{key} parses $a then ->[0] then ->{key}
#[test]
fn integration_nested_arrow_hash_slice() {
    let source = r#"$a->[0]->{key}"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should have arrow_array_deref and arrow_hash_deref
    assert!(
        sexp.contains("arrow_array_deref") && sexp.contains("arrow_hash_deref"),
        "Should have array deref and hash deref, got: {}",
        sexp
    );
}

// =============================================================================
// Integration Test 4: Error Propagation
// Test that errors in hash slice parsing are correctly reported
// =============================================================================

/// Test: Unclosed brace in hash slice - error propagation
/// When hash slice has unclosed brace, error should be reported
#[test]
fn integration_unclosed_brace_error() {
    // This should produce an error (unclosed brace)
    let source = r#"%hash{key"#;
    let ast = parse(source);

    // The AST should have an error node (parser recovers)
    let has_error = find_first_error(&ast);
    assert!(has_error.is_some(), "Unclosed brace should produce error in AST");
}

/// Test: Invalid hash slice key expression - error propagation
/// When hash slice has invalid key, error should be propagated
#[test]
fn integration_invalid_key_expression() {
    // This should produce an error (invalid expression)
    let source = r#"%hash{++}"#;
    let ast = parse(source);

    // Parser should recover and produce an error node
    let has_error = find_first_error(&ast);
    assert!(has_error.is_some(), "Invalid key expression should produce error in AST");
}

// =============================================================================
// Integration Test 5: Full Statement Parsing
// Test complete Perl statements containing hash slices
// =============================================================================

/// Test: Assignment statement with hash slice
/// Full statement: `my @vals = %hash{key1, key2};`
#[test]
fn integration_assignment_with_hash_slice() {
    let source = r#"my @vals = %hash{key1, key2};"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should have variable declaration and hash slice
    assert!(sexp.contains("my_decl"), "Should have my_decl node, got: {}", sexp);
    assert!(sexp.contains("binary_{}"), "Should have binary_{{}} node, got: {}", sexp);
}

/// Test: Subroutine call with hash slice as argument
/// Full statement: `some_func(%hash{key1, key2})`
#[test]
fn integration_sub_call_with_hash_slice() {
    let source = r#"some_func(%hash{key1, key2})"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should have function call with hash slice as argument
    assert!(sexp.contains("call"), "Should have call node, got: {}", sexp);
    assert!(sexp.contains("binary_{}"), "Should have binary_{{}} node, got: {}", sexp);
}

/// Test: Hash slice in conditional
/// Full statement: `if (%hash{@keys}) { ... }`
#[test]
fn integration_hash_slice_in_conditional() {
    let source = r#"if (%hash{@keys}) { 1 }"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should have if block with hash slice condition
    assert!(sexp.contains("if"), "Should have if node, got: {}", sexp);
    assert!(sexp.contains("binary_{}"), "Should have binary_{{}} node, got: {}", sexp);
}

/// Test: Hash slice in list assignment (tuple destructuring)
/// Full statement: `my ($a, $b) = %hash{key1, key2};`
#[test]
fn integration_hash_slice_in_list_assignment() {
    let source = r#"my ($a, $b) = %hash{key1, key2};"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("list_assignment") || sexp.contains("my_decl"),
        "Should have list assignment, got: {}",
        sexp
    );
    assert!(sexp.contains("binary_{}"), "Should have binary_{{}} node, got: {}", sexp);
}

// =============================================================================
// Integration Test 6: Real-World Corpus Patterns
// Test patterns from actual CPAN files that were previously failing
// =============================================================================

/// Test: Pattern from App::Cpan - hash slice with map/split/values
/// This pattern was causing unexpected_comma_expr errors before the fix
#[test]
fn integration_cpan_pattern_map_split_values() {
    let source = r#"@ops_seen{ map split(/ /), values %ops } = ();"#;
    let ast = parse(source);

    // This was the primary failing pattern
    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    // Should contain the hash slice (binary_{}) and the map/split/values calls
    // The sexp shows: (call map ...) and (call values ...) and (regex ...)
    assert!(
        sexp.contains("binary_{}") && sexp.contains("call map") && sexp.contains("call values"),
        "Should contain hash slice and map/values calls: {}",
        sexp
    );
}

/// Test: Hash slice with values builtin
/// Pattern: `keys %hash{ values %other }`
#[test]
fn integration_cpan_pattern_values_builtin() {
    let source = r#"my @keys = keys %hash{ values %other };"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("keys") && sexp.contains("values"),
        "Should contain keys and values: {}",
        sexp
    );
}

/// Test: Hash slice in sort
/// Pattern: `sort %hash{@keys}`
#[test]
fn integration_cpan_pattern_in_sort() {
    let source = r#"my @sorted = sort %hash{@keys};"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("sort") && sexp.contains("binary_{}"),
        "Should contain sort and hash slice: {}",
        sexp
    );
}

/// Test: Hash slice in map
/// Pattern: `map { ... } %hash{@keys}`
#[test]
fn integration_cpan_pattern_in_map() {
    let source = r#"my @mapped = map { $_ x 2 } %hash{@keys};"#;
    let ast = parse(source);

    assert_clean_parse(source);

    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("map") && sexp.contains("binary_{}"),
        "Should contain map and hash slice: {}",
        sexp
    );
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Find a binary node with the specified operator in the AST
fn find_binary_node_with_op(node: &Node, op: &str) -> Option<Node> {
    match &node.kind {
        NodeKind::Binary { op: node_op, .. } if node_op == op => Some(node.clone()),
        _ => {
            for child in node.children() {
                if let Some(found) = find_binary_node_with_op(child, op) {
                    return Some(found);
                }
            }
            None
        }
    }
}

/// Walk the AST recursively and return the kind_name of the first error or
/// missing node found, or `None` if the tree is clean.
fn find_first_error(node: &Node) -> Option<&'static str> {
    match &node.kind {
        NodeKind::Error { .. }
        | NodeKind::MissingExpression
        | NodeKind::MissingStatement
        | NodeKind::MissingIdentifier
        | NodeKind::MissingBlock => return Some(node.kind.kind_name()),
        _ => {}
    }
    for child in node.children() {
        if let Some(name) = find_first_error(child) {
            return Some(name);
        }
    }
    None
}
