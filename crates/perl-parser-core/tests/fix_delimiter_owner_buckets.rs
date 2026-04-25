//! Delimiter owner-bucket regression tests.
//!
//! These tests keep the known delimiter-heavy corpus patterns grouped by
//! nearest syntactic owner so only true malformed input remains in the
//! unclosed delimiter buckets.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::error::{ParseError, RecoveryKind};
use perl_parser_core::Parser;
use perl_tdd_support::must;

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

// nested hash subscript — inner } consumed by inner expect_closing_delimiter,
// outer } consumed by outer expect_closing_delimiter; no InsertedCloser emitted
#[test]
fn bucket_nested_hash_subscript_is_clean() {
    assert_clean_parse(r#"my $v = $outer{$inner{key}};"#);
}

// hash slice — @hash{@keys} uses the @-sigil path (postfix.rs line ~64).
// Must parse cleanly with no InsertedCloser.
#[test]
fn bucket_hash_slice_at_sigil_is_clean() {
    assert_clean_parse(r#"my @vals = @hash{@keys};"#);
}

// tie with parens — clean form exercises the has_parens path in statements.rs
#[test]
fn bucket_tie_with_parens_is_clean() {
    assert_clean_parse(r#"tie(%hash, 'Tie::Hash', $arg);"#);
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

// missing inner } in nested hash — InsertedCloser must fire for inner site
#[test]
fn malformed_missing_inner_hash_brace_in_nested_subscript_reports_error() {
    assert_has_error(r#"my $v = $outer{$inner{key;"#, "insertedcloser");
}

// tie with parens — missing ) before ; must emit InsertedCloser via
// expect_closing_delimiter (statements.rs change).
#[test]
fn malformed_tie_missing_paren_emits_inserted_closer() {
    let src = r#"tie(%hash, 'MyPkg', $arg;"#;
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let errors = parser.errors().to_vec();

    let has_inserted_closer = errors.iter().any(|e| {
        matches!(e, ParseError::Recovered { kind: RecoveryKind::InsertedCloser, .. })
    });
    assert!(
        has_inserted_closer,
        "Expected InsertedCloser for missing ')' in tie() for '{}', got errors: {:?}",
        src,
        errors
    );
    assert!(
        matches!(ast.kind, perl_parser_core::NodeKind::Program { .. }),
        "Parser must return a Program node for tie with missing ')'"
    );
}
