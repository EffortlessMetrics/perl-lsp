//! Delimiter owner-bucket regression tests.
//!
//! These tests keep the known delimiter-heavy corpus patterns grouped by
//! nearest syntactic owner so only true malformed input remains in the
//! unclosed delimiter buckets.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// function call args
#[test]
fn bucket_call_args_multiline_chained_arg_is_clean() {
    assert_clean_parse(
        r#"my $v = func(
    $obj->build()->finish(),
    $x,
);"#,
    );
}

// declaration lists
#[test]
fn bucket_declaration_list_in_parens_is_clean() {
    assert_clean_parse(r#"my ($x, $y) = (my $a, my $b);"#);
}

// hash literal vs block
#[test]
fn bucket_hash_subscript_keyword_key_is_clean() {
    assert_clean_parse(r#"delete _getstash($target)->{new};"#);
}

// postfix deref chain
#[test]
fn bucket_postfix_deref_chain_with_slice_is_clean() {
    assert_clean_parse(r#"my @v = $obj->factory->items->[$start..$end];"#);
}

// signature/prototype
#[test]
fn bucket_signature_like_parens_in_decl_is_clean() {
    assert_clean_parse(r#"sub f ($x, $y) { return $x + $y; }"#);
}

// quote-like expression
#[test]
fn bucket_quote_like_balanced_is_clean() {
    assert_clean_parse(r#"my $s = qq{hello $name};"#);
}

// heredoc boundary
#[test]
fn bucket_heredoc_argument_boundary_is_clean() {
    assert_clean_parse("foo(<<END, $x);\nline\nEND\n");
}

// malformed-input recovery-only guards
#[test]
fn malformed_missing_quote_like_closer_still_reports_error() {
    assert_has_error(r#"my $s = qq{hello;"#, "unclosed");
}

#[test]
fn malformed_missing_call_paren_still_reports_error() {
    assert_has_error(r#"my $x = func($a, $b;"#, "insertedcloser");
}
