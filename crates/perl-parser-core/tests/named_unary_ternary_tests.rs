mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_defined_ternary_basic() {
    assert_clean_parse("defined $tzname ? $tzname : 'fallback';");
}

#[test]
fn test_ref_ternary_basic() {
    assert_clean_parse("ref $_[0] ? 1 : 0;");
}

#[test]
fn test_defined_ternary_function_calls() {
    assert_clean_parse("defined $base_path ? do_a() : do_b();");
}

#[test]
fn test_defined_ternary_assignment() {
    assert_clean_parse("my $val = defined $x ? $x : $default;");
}

#[test]
fn test_ref_ternary_with_comparison() {
    assert_clean_parse("ref $obj eq 'HASH' ? $obj->{key} : undef;");
}

#[test]
fn test_defined_ternary_nested() {
    assert_clean_parse("defined $a ? defined $b ? $b : $c : $d;");
}

#[test]
fn test_exists_ternary() {
    assert_clean_parse("exists $hash{$key} ? $hash{$key} : 'missing';");
}

#[test]
fn test_defined_ternary_in_list_context() {
    assert_clean_parse("push @result, defined $v ? $v : 0;");
}
