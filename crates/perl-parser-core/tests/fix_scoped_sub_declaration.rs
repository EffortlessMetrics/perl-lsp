//! Regression coverage for scoped subroutine declarations.
//! Perl allows lexical and persistent scoped sub declarations such as
//! `my sub` and `state sub`.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;

#[test]
fn parses_state_sub_declaration() {
    assert_clean_parse("state sub helper { 1 }");
}

#[test]
fn parses_my_sub_declaration() {
    assert_clean_parse("my sub helper { 1 }");
}
