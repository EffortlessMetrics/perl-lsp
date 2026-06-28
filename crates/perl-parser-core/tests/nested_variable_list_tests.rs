//! Tests for nested variable lists in lexical declarations.
//!
//! Covers parsing of constructs like: my ($a, ($b, $c)) = ...
//! and my ($x, ($y, ($z, $w))) = ...

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn nested_variable_list_simple_pair() {
    let code = "my ($a, ($b, $c)) = (1, (2, 3));";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_with_three_items() {
    let code = "my ($x, ($y, $z, $w)) = (1, (2, 3, 4));";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_deeply_nested() {
    let code = "my ($a, ($b, ($c, $d))) = (1, (2, (3, 4)));";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_with_undef() {
    let code = "my ($x, (undef, $y)) = (1, (2, 3));";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_multiple_nesting_branches() {
    let code = "my ($a, ($b, $c), ($d, $e)) = (1, (2, 3), (4, 5));";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_array_and_nested() {
    let code = "my (@arr, ($x, $y)) = @_;";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_hash_and_nested() {
    let code = "my (%hash, ($key, $value)) = @_;";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_in_for_loop() {
    let code = "for my ($a, ($b, $c)) (@data) { }";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_our_declarator() {
    let code = "our ($x, ($y, $z));";
    assert_clean_parse(code);
}

#[test]
fn nested_variable_list_state_declarator() {
    let code = "state ($counter, ($backup, $temp)) = (0, (0, 0));";
    assert_clean_parse(code);
}
