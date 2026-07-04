//! Regression tests for issue #1911 — incomplete hashref/brace literal must
//! yield a recovery marker, not a silently-empty block.
//!
//! Before the fix, an incomplete brace literal that the user is mid-typing
//! (e.g. `my $cfg = { host => "localhost", port =>`) parsed to an empty
//! `(block )` node: the partial content AND the incompleteness were both lost
//! from the AST, so consumers had no recovery marker to act on. The fix emits a
//! `MissingExpression` marker in the returned block while preserving the #1352
//! anti-swallow behavior (trailing declarations stay separate top-level nodes).

use perl_parser_core::Parser;
use perl_tdd_support::must;

fn sexp(src: &str) -> String {
    let mut parser = Parser::new(src);
    must(parser.parse()).to_sexp()
}

fn error_count(src: &str) -> usize {
    let mut parser = Parser::new(src);
    let _ = must(parser.parse());
    parser.get_errors().len()
}

/// Core case: an incomplete hashref literal at EOF must leave a recovery marker
/// in the AST rather than a silently-empty block.
#[test]
fn incomplete_hashref_literal_emits_recovery_marker() {
    let src = "my $cfg = {\n    host => \"localhost\",\n    port =>\n";
    let s = sexp(src);
    assert!(
        s.contains("(missing_expression)")
            || s.contains("ERROR")
            || s.contains("(missing_statement)")
            || s.contains("(UNKNOWN_REST)"),
        "incomplete hashref must carry a recovery marker, got: {s}"
    );
    assert!(error_count(src) >= 1, "incomplete hashref must record at least one diagnostic");
}

// Note: the #1352 anti-swallow contract (trailing `sub`/`my` after an unclosed
// brace stay as separate top-level statements) is comprehensively covered by
// `tests/test_1352_recovery_premature_bail.rs` and is unchanged by this fix.

/// A complete, valid hashref must remain clean — no recovery marker, no errors.
#[test]
fn complete_hashref_literal_stays_clean() {
    let src = "my $cfg = { host => \"localhost\", port => 8080 };";
    let s = sexp(src);
    assert!(
        !s.contains("(missing_expression)") && !s.contains("ERROR"),
        "valid hashref must not produce recovery markers, got: {s}"
    );
    assert_eq!(error_count(src), 0, "valid hashref must have 0 errors");
}
