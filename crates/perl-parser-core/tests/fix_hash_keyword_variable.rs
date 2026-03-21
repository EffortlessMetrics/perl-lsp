mod cpan_test_helpers;
use cpan_test_helpers::*;

// Pattern B from issue #2149: Hash variable with keyword name after block close
//
// After `}`, the lexer is in ExpectOperator mode. `%` becomes a Percent (modulo)
// token, and `try` becomes TokenKind::Try. The parser's `parse_variable_from_sigil()`
// only accepted keyword tokens as variable names after `&` sigil, not `%` or others.
// The fix removes the `sigil == "&"` guard so ALL sigils accept keyword-named variables.

#[test]
fn hash_keyword_name_try_after_block() {
    // %try after block close was parsed as modulo + try keyword
    assert_clean_parse(r#"if (1) { 1; } %try = ();"#);
}

#[test]
fn hash_keyword_name_default_after_block() {
    assert_clean_parse(r#"if (1) { 1; } %default = ();"#);
}

#[test]
fn my_hash_keyword_name_for() {
    // my %for should work as a hash variable declaration
    assert_clean_parse(r#"my %for = (a => 1);"#);
}

#[test]
fn my_hash_keyword_name_if_with_access() {
    // my %if; then access with $if{key}
    assert_clean_parse(r#"my %if; $if{key} = 1;"#);
}

#[test]
fn hash_keyword_name_given_after_unless() {
    assert_clean_parse(r#"unless ($x) { 1; } %given = ();"#);
}

#[test]
fn hash_keyword_name_try_in_block() {
    // %try after bare block
    assert_clean_parse(r#"{ 1 } %try = ();"#);
}

#[test]
fn my_hash_keyword_name_try() {
    assert_clean_parse(r#"if (1) { 1; } my %try = ();"#);
}

#[test]
fn my_hash_keyword_name_default() {
    assert_clean_parse(r#"if (1) { 1; } my %default = ();"#);
}

// Regression: regular hash variables must still work
#[test]
fn regular_hash_still_works() {
    assert_clean_parse(r#"%regular_hash = (a => 1, b => 2);"#);
}

#[test]
fn regular_hash_after_block() {
    assert_clean_parse(r#"if (1) { 1; } %regular_hash = ();"#);
}

// Regression: scalar/array keyword names should also work with the fix
#[test]
fn scalar_keyword_name_try() {
    assert_clean_parse(r#"my $try = 1;"#);
}

#[test]
fn array_keyword_name_try() {
    assert_clean_parse(r#"my @try = (1, 2);"#);
}

// Regression: & sigil with keyword name still works
#[test]
fn ampersand_keyword_name_still_works() {
    assert_clean_parse(r#"&try();"#);
}
