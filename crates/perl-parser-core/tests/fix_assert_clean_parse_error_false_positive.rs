//! Regression test: assert_clean_parse must not false-positive on valid Perl
//! that contains the literal string "ERROR" in an identifier.
//!
//! Issue #2553: `use constant ERROR => 2;` was flagged as a parse error because
//! assert_clean_parse string-matched "(ERROR " in the S-expression output.

mod cpan_test_helpers;
use cpan_test_helpers::*;

/// `use constant ERROR => 2;` is valid Perl -- the word ERROR is a constant name,
/// not an AST error node. The old string-matching approach would see "(ERROR "
/// in the sexp and incorrectly fail this assertion.
#[test]
fn test_constant_named_error_is_clean() {
    let source = "use constant ERROR => 2;";
    assert_clean_parse(source);
}

/// Also verify a constant named with an ERROR-prefixed name doesn't trip it.
#[test]
fn test_constant_error_prefix_name_is_clean() {
    let source = "use constant ERROR_CODE => 42;";
    assert_clean_parse(source);
}

/// Verify that assert_has_error still correctly identifies real parse errors.
#[test]
fn test_assert_has_error_still_catches_real_errors() {
    // Phase 2: `my $x = ;` now recovers with a MissingExpression node (not an Error node
    // with "expected expression" text). Use the sexp name "missing_expression" as the needle.
    let source = "my $x = ;";
    assert_has_error(source, "missing_expression");
}
