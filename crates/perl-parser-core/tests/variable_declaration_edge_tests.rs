//! Variable Declaration Edge Cases
//!
//! Covers edge cases identified as a test gap (38 errors):
//! - undef placeholder in list destructuring
//! - attributes on variables and subs
//! - package-qualified `our` declarations
//! - `local` with lists and typeglobs
//! - `state` with non-scalar types
//! - complex mixed-type initializers

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---------------------------------------------------------------------------
// Undef placeholder in list destructuring
// ---------------------------------------------------------------------------

#[test]
fn test_undef_placeholder_in_my_list() {
    assert_clean_parse("my ($a, undef, $c) = @_;");
}

// ---------------------------------------------------------------------------
// Attributes on variables and subs
// ---------------------------------------------------------------------------

#[test]
fn test_my_scalar_with_lvalue_attribute() {
    assert_clean_parse("my $x :lvalue;");
}

#[test]
fn test_sub_with_multiple_attributes() {
    assert_clean_parse("sub foo :lvalue :method { }");
}

// ---------------------------------------------------------------------------
// Package-qualified `our` declarations
// ---------------------------------------------------------------------------

#[test]
fn test_our_package_qualified_variable() {
    assert_clean_parse("our $Foo::Bar::baz;");
}

// ---------------------------------------------------------------------------
// Local with list and typeglobs
// ---------------------------------------------------------------------------

#[test]
fn test_local_list_assignment() {
    assert_clean_parse("local ($a, $b) = @_;");
}

#[test]
fn test_local_typeglob() {
    assert_clean_parse("local *FH;");
}

// ---------------------------------------------------------------------------
// State variables with non-scalar types
// ---------------------------------------------------------------------------

#[test]
fn test_state_scalar_with_initializer() {
    assert_clean_parse("state $count = 0;");
}

#[test]
fn test_state_array() {
    assert_clean_parse("state @cache;");
}

// ---------------------------------------------------------------------------
// Complex initializers with mixed types
// ---------------------------------------------------------------------------

#[test]
fn test_my_list_with_rest_array() {
    assert_clean_parse("my ($x, @rest) = (1, 2, 3, 4);");
}

#[test]
fn test_my_hash_fat_comma_initializer() {
    assert_clean_parse("my %opts = (verbose => 1, debug => 0);");
}
