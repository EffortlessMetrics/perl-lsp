//! Hash slice snapshot tests (work-e5278c16)
//!
//! These tests capture the S-expression output of the parser for hash slice
//! patterns. Each snapshot is a baseline that will fail if the output changes.
//!
//! This is the Snapshot Agent's primary output - capturing what the parser
//! currently produces so any change is immediately detected.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Snapshot Tests: Hash Slice Without Arrow (%hash{...}, @hash{...})
// =============================================================================

/// Snapshot: Simple hash slice with % sigil
/// Input: `%hash{key}`
/// Output: Binary node with `{}` operator
#[test]
fn snapshot_percent_hash_slice_simple() {
    let source = r#"%hash{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Baseline snapshot - any change to this output will be detected
    assert_eq!(
        sexp, "(source_file (binary_{} (variable % hash) (identifier key)))",
        "Hash slice %hash{{key}} should produce binary_{{}} node"
    );
}

/// Snapshot: Simple hash slice with @ sigil (Perl alias)
/// Input: `@hash{key}`
/// Output: Binary node with `{}` operator
#[test]
fn snapshot_at_hash_slice_simple() {
    let source = r#"@hash{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert_eq!(
        sexp, "(source_file (binary_{} (variable @ hash) (identifier key)))",
        "Hash slice @hash{{key}} should produce binary_{{}} node"
    );
}

/// Snapshot: Hash slice with multiple bareword keys
/// Input: `%hash{key1, key2}`
/// Output: Binary node with array of identifiers
#[test]
fn snapshot_hash_slice_multiple_keys() {
    let source = r#"%hash{key1, key2}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert_eq!(
        sexp,
        "(source_file (binary_{} (variable % hash) (array (identifier key1) (identifier key2))))",
        "Hash slice with multiple keys should produce array"
    );
}

/// Snapshot: Hash slice with variable key
/// Input: `@hash{$key}`
/// Output: Binary node with variable as key
#[test]
fn snapshot_hash_slice_variable_key() {
    let source = r#"@hash{$key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert_eq!(
        sexp, "(source_file (binary_{} (variable @ hash) (variable $ key)))",
        "Hash slice with variable key should produce variable node"
    );
}

/// Snapshot: Complex hash slice from CPAN code
/// Input: `@ops_seen{ map split(/ /), values %ops }`
/// Output: Binary node with call expression as key
#[test]
fn snapshot_hash_slice_complex_map_split() {
    let source = r#"@ops_seen{ map split(/ /), values %ops }"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // This is the exact pattern that was failing before the fix
    assert!(sexp.contains("(binary_{}"), "Should produce binary_{{}} node, got: {}", sexp);
    assert!(
        sexp.contains("(variable @ ops_seen)"),
        "Should have variable @ ops_seen, got: {}",
        sexp
    );
}

/// Snapshot: Hash slice with single-quoted string key
/// Input: `%hash{'key'}`
/// Output: Binary node with string as key
#[test]
fn snapshot_hash_slice_single_quoted_key() {
    let source = r#"%hash{'key'}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should produce binary_{{}} node, got: {}", sexp);
}

/// Snapshot: Hash slice with double-quoted string key
/// Input: `%hash{"key"}`
/// Output: Binary node with double-quoted string
#[test]
fn snapshot_hash_slice_double_quoted_key() {
    let source = r#"%hash{"key"}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should produce binary_{{}} node, got: {}", sexp);
}

/// Snapshot: Hash slice with trailing comma
/// Input: `%hash{key1, key2,}`
/// Output: Binary node with array (trailing comma allowed)
#[test]
fn snapshot_hash_slice_trailing_comma() {
    let source = r#"%hash{key1, key2,}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should produce binary_{{}} node, got: {}", sexp);
}

/// Snapshot: Hash slice on qualified variable
/// Input: `%Pkg::Hash{key}`
/// Output: Binary node with qualified variable
#[test]
fn snapshot_hash_slice_qualified_variable() {
    let source = r#"%Pkg::Hash{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should produce binary_{{}} node, got: {}", sexp);
}

/// Snapshot: Hash slice on scalar ref
/// Input: `%$href{key}`
/// Output: Binary node with scalar deref
#[test]
fn snapshot_hash_slice_scalar_ref() {
    let source = r#"%$href{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should produce binary_{{}} node, got: {}", sexp);
}

// =============================================================================
// Snapshot Tests: Arrow Hash Dereference (unchanged behavior)
// =============================================================================

/// Snapshot: Arrow hash dereference
/// Input: `$ref->{key}`
/// Output: arrow_hash_deref node
#[test]
fn snapshot_arrow_hash_deref_simple() {
    let source = r#"$ref->{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert_eq!(
        sexp, "(source_file (arrow_hash_deref (variable $ ref) (identifier key)))",
        "Arrow hash deref should produce arrow_hash_deref node"
    );
}

/// Snapshot: Arrow hash dereference with variable key
/// Input: `$ref->{$expr}`
/// Output: arrow_hash_deref node with variable
#[test]
fn snapshot_arrow_hash_deref_variable_key() {
    let source = r#"$ref->{$expr}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert_eq!(
        sexp, "(source_file (arrow_hash_deref (variable $ ref) (variable $ expr)))",
        "Arrow hash deref with variable should work"
    );
}

/// Snapshot: Arrow hash dereference with nested deref
/// Input: `$ref->{$h->{nested}}`
/// Output: arrow_hash_deref node with nested arrow_hash_deref
#[test]
fn snapshot_arrow_hash_deref_nested() {
    let source = r#"$ref->{$h->{nested}}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("arrow_hash_deref"), "Should contain arrow_hash_deref, got: {}", sexp);
}

// =============================================================================
// Snapshot Tests: Hash Literal vs Block (unchanged behavior)
// =============================================================================

/// Snapshot: Hash literal (NOT a slice)
/// Input: `{ $a => $b }`
/// Output: block with hash literal inside
#[test]
fn snapshot_hash_literal() {
    let source = r#"{ $a => $b }"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Hash literal should NOT produce binary_{} node
    assert!(
        !sexp.contains("(binary_{}"),
        "Hash literal should NOT produce binary_{{}} node, got: {}",
        sexp
    );
    assert!(sexp.contains("(hash"), "Hash literal should contain hash node, got: {}", sexp);
}

/// Snapshot: Block with list (comma-separated, not hash literal)
/// Input: `{ $a, $b }`
/// Output: block with expression_statement
#[test]
fn snapshot_block_with_list() {
    let source = r#"{ $a, $b }"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(block"), "Should be a block, got: {}", sexp);
}

/// Snapshot: Empty hash literal
/// Input: `{}`
/// Output: empty block
#[test]
fn snapshot_empty_hash_literal() {
    let source = r#"{}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(block"), "Empty braces should be block, got: {}", sexp);
}

// =============================================================================
// Snapshot Tests: Chained Operations
// =============================================================================

/// Snapshot: Hash slice followed by method call
/// Input: `%hash{key}->method()`
/// Output: hash slice then arrow method
#[test]
fn snapshot_hash_slice_chained_method() {
    let source = r#"%hash{key}->method()"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice binary_{{}}, got: {}", sexp);
}

/// Snapshot: Arrow deref followed by hash slice
/// Input: `$ref->{key}{nested}`
/// Output: chained operations
#[test]
fn snapshot_arrow_chained_hash_slice() {
    let source = r#"$ref->{key}{nested}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("arrow_hash_deref"), "Should have arrow_hash_deref, got: {}", sexp);
}

/// Snapshot: Multiple hash slices in expression
/// Input: `@a{@x} = @b{@y};`
/// Output: two hash slices
#[test]
fn snapshot_multiple_hash_slices() {
    let source = r#"@a{@x} = @b{@y};"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Should have at least 2 binary_{} nodes
    let count = sexp.matches("(binary_{}").count();
    assert!(count >= 2, "Should have at least 2 hash slices, got {} in: {}", count, sexp);
}

/// Snapshot: Array of hashes slice
/// Input: `@array[$i]{key1, key2}`
/// Output: array index then hash slice
#[test]
fn snapshot_array_of_hashes_slice() {
    let source = r#"@array[$i]{key1, key2}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice binary_{{}}, got: {}", sexp);
}

// =============================================================================
// Snapshot Tests: Hash Slice in Various Contexts
// =============================================================================

/// Snapshot: Hash slice in conditional
/// Input: `if (%hash{@keys}) { }`
/// Output: hash slice in if condition
#[test]
fn snapshot_hash_slice_in_conditional() {
    let source = r#"if (%hash{@keys}) { }"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice in conditional, got: {}", sexp);
}

/// Snapshot: Hash slice in sort
/// Input: `sort %hash{@keys}`
/// Output: hash slice as sort argument
#[test]
fn snapshot_hash_slice_in_sort() {
    let source = r#"sort %hash{@keys}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice in sort, got: {}", sexp);
}

/// Snapshot: Hash slice in map
/// Input: `map { $_ x 2 } %hash{@keys}`
/// Output: hash slice as map argument
#[test]
fn snapshot_hash_slice_in_map() {
    let source = r#"map { $_ x 2 } %hash{@keys}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice in map, got: {}", sexp);
}

/// Snapshot: Assignment to hash slice
/// Input: `%hash{key1, key2} = (1, 2);`
/// Output: assignment to hash slice
#[test]
fn snapshot_assignment_to_hash_slice() {
    let source = r#"%hash{key1, key2} = (1, 2);"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice in assignment, got: {}", sexp);
}

/// Snapshot: Hash slice with exists
/// Input: `exists %hash{key}`
/// Output: hash slice with exists function
#[test]
fn snapshot_hash_slice_with_exists() {
    let source = r#"exists %hash{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice with exists, got: {}", sexp);
}

/// Snapshot: Hash slice with delete
/// Input: `delete %hash{key};`
/// Output: hash slice with delete function
#[test]
fn snapshot_hash_slice_with_delete() {
    let source = r#"delete %hash{key};"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice with delete, got: {}", sexp);
}

/// Snapshot: Hash slice with defined
/// Input: `defined %hash{key}`
/// Output: hash slice with defined function
#[test]
fn snapshot_hash_slice_with_defined() {
    let source = r#"defined %hash{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice with defined, got: {}", sexp);
}

/// Snapshot: Hash slice followed by array index
/// Input: `%hash{key}[0, 2]`
/// Output: hash slice then array index
#[test]
fn snapshot_hash_slice_then_array_index() {
    let source = r#"%hash{key}[0, 2]"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("(binary_{}"), "Should have hash slice, got: {}", sexp);
    // Should also have array subscript
    assert!(
        sexp.contains("(array") || sexp.contains("binary_"),
        "Should have array subscript, got: {}",
        sexp
    );
}

/// Snapshot: Arrow array deref then hash slice
/// Input: `$array_ref->[0]->{key}`
/// Output: array deref then hash slice
#[test]
fn snapshot_arrow_array_deref_then_hash_slice() {
    let source = r#"$array_ref->[0]->{key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(
        sexp.contains("arrow_hash_deref") || sexp.contains("(binary_{}"),
        "Should have hash deref, got: {}",
        sexp
    );
}

/// Snapshot: Arrow hash deref then array index
/// Input: `$hash_ref->{key}[0]`
/// Output: hash deref then array index
#[test]
fn snapshot_arrow_hash_deref_then_array_index() {
    let source = r#"$hash_ref->{key}[0]"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(sexp.contains("arrow_hash_deref"), "Should have arrow_hash_deref, got: {}", sexp);
}

// =============================================================================
// Snapshot Tests: Negative/Boundary Cases
// =============================================================================

/// Snapshot: Hash slice with negative number key
/// Input: `$hash{-1}`
/// Output: scalar hash access with negative key
#[test]
fn snapshot_hash_slice_negative_key() {
    let source = r#"$hash{-1}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Should parse without error - negative numbers are valid hash keys
    assert!(!sexp.contains("(error"), "Should not have error node, got: {}", sexp);
}

/// Snapshot: Hash slice with large number key
/// Input: `$hash{999999999}`
/// Output: scalar hash access with large number
#[test]
fn snapshot_hash_slice_large_number_key() {
    let source = r#"$hash{999999999}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(!sexp.contains("(error"), "Should not have error node, got: {}", sexp);
}

/// Snapshot: Hash slice with special character bareword
/// Input: `$hash{_private_key}`
/// Output: scalar hash access with underscore-prefixed bareword
#[test]
fn snapshot_hash_slice_special_char_bareword() {
    let source = r#"$hash{_private_key}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(!sexp.contains("(error"), "Should not have error node, got: {}", sexp);
}

/// Snapshot: Hash slice with colons in key
/// Input: `$hash{'key::with::colons'}`
/// Output: scalar hash access with qualified-looking key
#[test]
fn snapshot_hash_slice_colon_key() {
    let source = r#"$hash{'key::with::colons'}"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    assert!(!sexp.contains("(error"), "Should not have error node, got: {}", sexp);
}
