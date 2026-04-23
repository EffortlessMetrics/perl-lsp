//! Ternary operator expression tests
//!
//! Covers basic, nested, chained, and contextual uses of the Perl
//! ternary (`?:`) operator.

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_ternary_basic_assignment() {
    assert_clean_parse("my $x = $a ? 1 : 0;");
}

#[test]
fn test_ternary_nested_in_true_branch() {
    assert_clean_parse("my $x = $a ? $b ? 1 : 2 : 3;");
}

#[test]
fn test_ternary_in_function_args() {
    assert_clean_parse("foo($x ? 1 : 2);");
}

#[test]
fn test_ternary_with_method_call_condition() {
    assert_clean_parse("my $r = $obj->method ? 'yes' : 'no';");
}

#[test]
fn test_ternary_multiline() {
    assert_clean_parse("my $x = $cond\n  ? 'true'\n  : 'false';");
}

#[test]
fn test_ternary_after_regex_match() {
    assert_clean_parse("my $r = $x =~ /pattern/\n  ? 'match'\n  : 'no match';");
}

#[test]
fn test_ternary_in_list_context() {
    assert_clean_parse("my @x = ($a ? $b : $c, $d);");
}

#[test]
fn test_ternary_in_hash_value() {
    assert_clean_parse("my %h = (key => $x ? 1 : 0);");
}

#[test]
fn test_ternary_chained() {
    assert_clean_parse("my $x = $a ? 1 : $b ? 2 : $c ? 3 : 4;");
}
