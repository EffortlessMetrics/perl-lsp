//! Hash slice postfix tests (work-e5278c16)
//!
//! Tests that hash slices and array slices (`@hash{...}`, `%hash{...}`)
//! are parsed correctly as postfix subscript operations without requiring
//! an intervening arrow (`->`).
//!
//! These tests should FAIL before the fix is implemented (red state).
//! After the fix in postfix.rs, they should PASS (green state).

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === AC1: Hash Slice Without Arrow ===

/// Hash slice using % sigil should parse as a single postfix operation
/// Currently fails: @hash{...} is treated as two separate expressions
#[test]
fn test_percent_hash_slice_without_arrow_simple() {
    let source = r#"my @vals = %hash{$key1, $key2};"#;
    // This should parse cleanly - currently it produces unexpected_comma_expr errors
    assert_clean_parse(source);
}

/// Hash slice using @ sigil (Perl alias for hash slice) should parse correctly
/// Currently fails: @hash{...} is treated as two separate expressions
#[test]
fn test_at_hash_slice_without_arrow_simple() {
    let source = r#"my $val = @hash{$key};"#;
    // This should parse cleanly
    assert_clean_parse(source);
}

/// Array slice using @ sigil with bracket notation should also work
#[test]
fn test_array_slice_with_brackets() {
    let source = r#"my @selected = @array[0, 2, 4];"#;
    // This already works via LeftBracket arm
    assert_clean_parse(source);
}

// === AC2: Complex Hash Slice Expressions ===

/// Complex hash slice from actual CPAN code (overload.pm:27)
/// This is the primary failing pattern causing unexpected_comma_expr errors
#[test]
fn test_hash_slice_with_map_split_values() {
    let source = r#"@ops_seen{ map split(/ /), values %ops } = ();"#;
    // Should parse as: hash slice with complex key expression
    // Currently fails because the comma inside "map split(/ /), values %ops"
    // is flagged as unexpected_comma_expr
    assert_clean_parse(source);
}

/// Hash slice with map expression as key
#[test]
fn test_hash_slice_with_map_expr() {
    let source = r#"%seen{ map { $_ => 1 } keys %other };"#;
    assert_clean_parse(source);
}

/// Hash slice with values %hash as key
#[test]
fn test_hash_slice_with_values() {
    let source = r#"my @keys = keys %hash{ values %other };"#;
    assert_clean_parse(source);
}

/// Assignment to hash slice with complex expression
#[test]
fn test_assignment_to_hash_slice_complex() {
    let source = r#"@cache{ map $_->name, @objects } = ();"#;
    assert_clean_parse(source);
}

// === AC3: Arrow-Based Hash Dereference Unchanged ===

/// Arrow hash dereference should still work via Arrow arm
#[test]
fn test_arrow_hash_deref_still_works() {
    let source = r#"my $val = $ref->{key};"#;
    assert_clean_parse(source);
}

/// Arrow hash dereference with expression key
#[test]
fn test_arrow_hash_deref_with_expr() {
    let source = r#"my $val = $ref->{ $expr };"#;
    assert_clean_parse(source);
}

/// Arrow hash dereference with complex expression
#[test]
fn test_arrow_hash_deref_complex() {
    let source = r#"my $val = $ref->{ $h->{nested} };"#;
    assert_clean_parse(source);
}

// === AC4: Hash Literal vs Block Unchanged ===

/// Hash literal should still parse correctly
#[test]
fn test_hash_literal_still_works() {
    let source = r#"my $href = { $a => $b };"#;
    assert_clean_parse(source);
}

/// Block with list (comma-separated) should still parse correctly
#[test]
fn test_block_with_list_still_works() {
    let source = r#"my $ref = { $a, $b };"#;
    assert_clean_parse(source);
}

/// Empty hash literal
#[test]
fn test_empty_hash_literal() {
    let source = r#"my $href = {};"#;
    assert_clean_parse(source);
}

/// Hash literal with multiple pairs
#[test]
fn test_hash_literal_multiple_pairs() {
    let source = r#"my %h = (a => 1, b => 2, c => 3);"#;
    assert_clean_parse(source);
}

// === Additional Edge Cases ===

/// Hash slice on hash declaration
#[test]
fn test_hash_slice_on_declared_hash() {
    let source = r#"my %h; $h{key1, key2} = 1;"#;
    assert_clean_parse(source);
}

/// Nested hash slice
#[test]
fn test_nested_hash_deref_hash_slice() {
    let source = r#"$outer->{inner}{key} = 1;"#;
    // This chains: $outer->{inner} then {key} on result
    assert_clean_parse(source);
}

/// Hash slice after method call
#[test]
fn test_hash_slice_after_method_call() {
    let source = r#"$obj->get_hash()->{key};"#;
    assert_clean_parse(source);
}

/// Multiple hash slices in expression
#[test]
fn test_multiple_hash_slices() {
    let source = r#"@a{@x} = @b{@y};"#;
    assert_clean_parse(source);
}
