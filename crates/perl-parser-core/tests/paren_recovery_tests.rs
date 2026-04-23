//! Tests for parenthesis recovery — the #1 test gap (288 error nodes).
//!
//! Validates that the parser produces error nodes for unclosed/malformed
//! parenthesized expressions and parses well-formed ones cleanly.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── Error cases: malformed parenthesized expressions ───────────────────

#[test]
fn test_unclosed_paren_with_identifier_tail_produces_error() {
    // Perl allows filehandle-style print forms: print($fh EXPR).
    assert_clean_parse("print($foo bar)");
}

#[test]
fn test_unclosed_paren_at_eof_produces_error() {
    // Missing closing paren at end of input.
    assert_has_error("my @x = (1, 2, 3", "insertedcloser");
}

#[test]
fn test_mixed_sigils_in_unclosed_paren_produces_error() {
    // Missing closing paren with mixed variable sigils.
    assert_has_error("foo($x, @y, %z", "insertedcloser");
}

// ── Clean cases: well-formed parenthesized expressions ─────────────────

#[test]
fn test_function_call_with_scalar_args_parses_clean() {
    assert_clean_parse("print($foo, $bar);");
}

#[test]
fn test_array_assignment_with_paren_list_parses_clean() {
    assert_clean_parse("my @x = (1, 2, 3);");
}

#[test]
fn test_function_call_with_mixed_sigils_parses_clean() {
    assert_clean_parse("foo($x, @y, %z);");
}

#[test]
fn test_map_block_with_paren_list_parses_clean() {
    assert_clean_parse("map { $_ + 1 } (1, 2, 3);");
}

#[test]
fn test_list_declaration_with_assignment_parses_clean() {
    assert_clean_parse("my ($a, $b) = @_;");
}

#[test]
fn test_list_assignment_parses_clean() {
    assert_clean_parse("($a, $b, $c) = (1, 2, 3);");
}
