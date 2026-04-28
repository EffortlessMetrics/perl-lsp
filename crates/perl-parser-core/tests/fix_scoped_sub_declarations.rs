//! Regression tests for scoped subroutine declarations.
//!
//! Perl allows `my sub`, `our sub`, and `state sub` declarations.
//! These should parse as subroutine statements rather than variable declarations.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;

#[test]
fn parses_my_sub_declaration() {
    assert_clean_parse("my sub helper ($x) { $x }");
}

#[test]
fn parses_our_sub_declaration() {
    assert_clean_parse("our sub helper ($x) { $x }");
}

#[test]
fn parses_state_sub_declaration() {
    assert_clean_parse("state sub memo { state $x = 1; $x }");
}
