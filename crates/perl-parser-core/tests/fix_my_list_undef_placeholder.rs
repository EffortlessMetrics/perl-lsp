// Regression tests for `undef` as a placeholder in `my (...)` list declarations
// inside parenthesised expression context.
//
// Root cause: `parse_declaration_arg` (used when `my (...)` appears inside a
// parenthesised expression like `(my ($a, $b, undef), $c) = func()`) did not
// allow `undef` in the variable list, unlike `parse_variable_declaration`
// which already handled it correctly.
//
// Pattern from real CPAN code:
// - Unicode/UCD.pm:2020
//   `(my ($simple_invlist_ref, $simple_invmap_ref, undef), $default)
//        = prop_invmap('Simple_Case_Folding');`

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_my_list_undef_in_nested_parens() {
    // Unicode/UCD.pm:2020 - undef placeholder inside my() inside outer parens
    let source = r#"(my ($simple_invlist_ref, $simple_invmap_ref, undef), $default) = prop_invmap('Simple_Case_Folding');"#;
    assert_clean_parse(source);
}

#[test]
fn test_my_list_undef_leading() {
    assert_clean_parse("(my (undef, $b, $c), $d) = func();");
}

#[test]
fn test_my_list_undef_only() {
    assert_clean_parse("(my (undef), $rest) = func();");
}

#[test]
fn test_my_list_undef_multiple() {
    assert_clean_parse("(my ($a, undef, $c, undef), $last) = func();");
}

#[test]
fn test_my_list_undef_standalone_still_works() {
    // The standalone `my (..., undef)` case was already working; regression guard
    assert_clean_parse("my ($a, $b, undef) = func();");
}

#[test]
fn test_outer_list_undef_item_still_works() {
    // undef in outer list (not inside my) was already working
    assert_clean_parse("($a, undef, $c) = func();");
}
