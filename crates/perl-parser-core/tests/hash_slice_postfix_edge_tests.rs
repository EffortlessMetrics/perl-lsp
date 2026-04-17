//! Hash slice postfix edge case tests (work-e5278c16)
//!
//! Edge cases beyond the core acceptance criteria tests.
//! These tests verify boundary conditions and unusual patterns.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Edge Case: Single-Element Hash Slice ===
/// Single bareword key
#[test]
fn test_single_bareword_key_hash_slice() {
    let source = r#"my @vals = %hash{key};"#;
    assert_clean_parse(source);
}

/// Single single-quoted string key
#[test]
fn test_single_quoted_string_key_hash_slice() {
    let source = r#"my @vals = %hash{'key'};"#;
    assert_clean_parse(source);
}

/// Single double-quoted string key (interpolated)
#[test]
fn test_single_double_quoted_key_hash_slice() {
    let source = r#"my $val = $hash{"key"};"#;
    // This is scalar access of hash element, not a slice
    // But it should still parse correctly
    assert_clean_parse(source);
}

// === Edge Case: Multiple Keys ===

/// Multiple bareword keys (comma-separated)
#[test]
fn test_multiple_bareword_keys_hash_slice() {
    let source = r#"my @vals = %hash{key1, key2, key3};"#;
    assert_clean_parse(source);
}

/// Mixed bareword and variable keys
#[test]
fn test_mixed_bareword_variable_keys_hash_slice() {
    let source = r#"my @vals = %hash{key1, $key2, key3};"#;
    assert_clean_parse(source);
}

/// Keys with trailing comma
#[test]
fn test_trailing_comma_hash_slice() {
    let source = r#"my @vals = %hash{key1, key2,};"#;
    assert_clean_parse(source);
}

// === Edge Case: Qualified Package Variables ===

/// Hash slice on qualified variable (package::name)
#[test]
fn test_qualified_package_hash_slice() {
    let source = r#"my @vals = %Pkg::Hash{key1, key2};"#;
    assert_clean_parse(source);
}

/// Hash slice on scalar-ref variable (%$href)
#[test]
fn test_scalar_ref_hash_slice() {
    let source = r#"my @vals = %$href{key1, key2};"#;
    assert_clean_parse(source);
}

/// Array slice alias on scalar-ref variable (@$href)
#[test]
fn test_scalar_ref_array_slice_alias() {
    let source = r#"my @vals = @$href{key1, key2};"#;
    assert_clean_parse(source);
}

// === Edge Case: Hash Slice in Various Contexts ===

/// Hash slice in conditional
#[test]
fn test_hash_slice_in_conditional() {
    let source = r#"if (%hash{@keys}) { print "found" }"#;
    assert_clean_parse(source);
}

/// Hash slice as sort argument
#[test]
fn test_hash_slice_in_sort() {
    let source = r#"my @sorted = sort %hash{@keys};"#;
    assert_clean_parse(source);
}

/// Hash slice as map argument
#[test]
fn test_hash_slice_in_map() {
    let source = r#"my @mapped = map { $_ x 2 } %hash{@keys};"#;
    assert_clean_parse(source);
}

/// Hash slice in list assignment
#[test]
fn test_hash_slice_in_list_assignment() {
    let source = r#"my ($a, $b) = %hash{key1, key2};"#;
    assert_clean_parse(source);
}

// === Edge Case: Complex Keys ===

/// Hash slice with double-quoted string key containing interpolation
#[test]
fn test_hash_slice_with_double_quoted_key() {
    let source = r#"my $val = $hash{"$key"};"#;
    // Note: this is scalar access $hash{"$key"}, not a slice
    assert_clean_parse(source);
}

/// Hash slice with Here-doc key (not valid, just to verify error handling)
/// This is NOT valid Perl - here-docs can't be hash keys
#[test]
fn test_hash_slice_with_here_document_not_valid() {
    // This would be a syntax error, not a parsing error per se
    // We don't test invalid Perl syntax here
}

// === Edge Case: Nested/Chained Operations ===

/// Hash slice followed by arrow method call
#[test]
fn test_hash_slice_then_method_call() {
    let source = r#"my $val = %hash{key}->method();"#;
    assert_clean_parse(source);
}

/// Hash slice in expression then method
#[test]
fn test_hash_slice_chained_method() {
    let source = r#"my $val = $obj->get_hash()->{key};"#;
    // This chains: $obj->get_hash() returns hash ref, then ->{key} dereferences
    assert_clean_parse(source);
}

/// Assignment to hash slice with complex LHS
#[test]
fn test_assignment_to_hash_slice_simple() {
    let source = r#"%hash{key1, key2} = (1, 2);"#;
    assert_clean_parse(source);
}

// === Edge Case: Negative/Boundary Values ===

/// Hash slice with negative number key
#[test]
fn test_hash_slice_negative_key() {
    let source = r#"my $val = $hash{-1};"#;
    // Negative number as hash key
    assert_clean_parse(source);
}

/// Hash slice with large number key
#[test]
fn test_hash_slice_large_number_key() {
    let source = r#"my $val = $hash{999999999};"#;
    assert_clean_parse(source);
}

// === Edge Case: Special Characters in Keys ===

/// Hash slice with special characters in bareword key
#[test]
fn test_hash_slice_special_char_bareword() {
    let source = r#"my $val = $hash{_private_key};"#;
    // Bareword starting with underscore
    assert_clean_parse(source);
}

/// Hash slice with colons in key
#[test]
fn test_hash_slice_colon_key() {
    let source = r#"my $val = $hash{'key::with::colons'};"#;
    // Colons in quoted string key
    assert_clean_parse(source);
}

// === Edge Case: Perl Specific Idioms ===

/// Hash slice using exists function
#[test]
fn test_hash_slice_with_exists() {
    let source = r#"if (exists %hash{key}) { }"#;
    assert_clean_parse(source);
}

/// Hash slice with delete function
#[test]
fn test_hash_slice_with_delete() {
    let source = r#"delete %hash{key};"#;
    assert_clean_parse(source);
}

/// Hash slice with defined function
#[test]
fn test_hash_slice_with_defined() {
    let source = r#"if (defined %hash{key}) { }"#;
    assert_clean_parse(source);
}

// === Regression: Ensure Hash Literals Still Work ===

/// Hash literal (not a slice) should still work
#[test]
fn test_hash_literal_not_slice() {
    let source = r#"my %h = (a => 1, b => 2);"#;
    assert_clean_parse(source);
}

/// Block with statement (not a hash literal)
#[test]
fn test_block_not_hash_literal() {
    let source = r#"my $ref = { my $x = 1; $x };"#;
    assert_clean_parse(source);
}

// === Regression: Ensure Arrow Deref Still Works ===

/// Arrow array deref followed by hash slice
#[test]
fn test_arrow_array_deref_then_hash_slice() {
    let source = r#"my $val = $array_ref->[0]->{key};"#;
    assert_clean_parse(source);
}

/// Arrow hash deref followed by array index
#[test]
fn test_arrow_hash_deref_then_array_index() {
    let source = r#"my $val = $hash_ref->{key}[0];"#;
    assert_clean_parse(source);
}

// === Edge Case: Multi-dimensional-like Access ===

/// Array of hashes - slice access
#[test]
fn test_array_of_hashes_slice() {
    let source = r#"my @vals = @array[$i]{key1, key2};"#;
    // @array[$i] is array index, then {key1, key2} is hash slice on result
    assert_clean_parse(source);
}

/// Hash of arrays - slice access
#[test]
fn test_hash_of_arrays_slice() {
    let source = r#"my @vals = %hash{key}[0, 2];"#;
    // %hash{key} is hash slice, then [0, 2] is array index on result
    assert_clean_parse(source);
}
