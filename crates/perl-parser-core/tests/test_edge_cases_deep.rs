mod cpan_test_helpers;
use cpan_test_helpers::*;

// Edge case: mixed slice with regex-op names and variable
#[test]
fn test_hash_slice_mixed_regex_op_and_var() {
    assert_clean_parse(r#"my @v = @h{m, $var, s};"#);
}

// Edge case: slice with regex-op name and regular string key
#[test]
fn test_hash_slice_regex_op_and_string_key() {
    assert_clean_parse(r#"my @v = @h{m, 'regular', s};"#);
}

// Edge case: trailing comma in slice with regex-op key
#[test]
fn test_hash_slice_trailing_comma() {
    // Trailing comma before `}` is valid Perl
    assert_clean_parse(r#"my @v = @h{m, s,};"#);
}

// Edge case: empty hash subscript (should not be affected)
#[test]
fn test_empty_hash_subscript() {
    // {} in hash context should parse as empty hash ref or block
    // Not a subscript, but verify depth is not corrupted
    assert_clean_parse(r#"my %h; $h{key} = 1;"#);
}

// Edge case: chained subscripts with regex-op key
#[test]
fn test_chained_subscripts_with_regex_key() {
    assert_clean_parse(r#"my $x = $h{m}{nested_key};"#);
}

// Edge case: deeply nested subscripts
#[test]
fn test_deeply_nested_subscripts_with_regex_key() {
    assert_clean_parse(r#"my $x = $h{a}{b}{m};"#);
}

// Edge case: hash ref dereference then subscript with regex key
#[test]
fn test_deref_hash_subscript_regex_key() {
    assert_clean_parse(r#"my $x = ${$ref}{m};"#);
}

// Edge case: regex-op key in ternary context
#[test]
fn test_regex_key_in_ternary() {
    assert_clean_parse(r#"my $x = $cond ? $h{m} : $h{s};"#);
}

// Edge case: regex-op key as function argument
#[test]
fn test_regex_key_as_argument() {
    assert_clean_parse(r#"foo($h{m}, $h{s});"#);
}

// Edge case: regex-op name as sole key in arrow subscript with peek_second returning EOF
#[test]
fn test_arrow_subscript_regex_key_at_end() {
    assert_clean_parse(r#"$ref->{m}"#); // No trailing semicolon
}
