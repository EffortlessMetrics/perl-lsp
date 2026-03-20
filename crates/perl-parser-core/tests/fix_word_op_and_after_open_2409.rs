//! Regression tests for issue #2409: `and` word operator after open(...) or
//! similar 3-argument list-operator calls with explicit parentheses.
//!
//! Pattern: `open(my $fh, '<', $file) and do { ... }`
//! Fixed as part of #2396 (word-op dispatch in parse_expression_statement).

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === open() + and ===

#[test]
fn test_open_3arg_and_binmode() {
    // Exact pattern from the issue description
    assert_clean_parse("open(my $fh, '<', $f) and binmode($fh);");
}

#[test]
fn test_open_3arg_and_do_block() {
    // Pattern from Catalyst.pm: open(...) and do { ... }
    assert_clean_parse(r#"open(my $fh, '<', $file) and do { print "ok\n" };"#);
}

#[test]
fn test_open_2arg_and_die() {
    // 2-argument open followed by and die
    assert_clean_parse(r#"open(my $fh, $file) and die "unexpected";"#);
}

#[test]
fn test_open_bareword_and_die() {
    // open with bareword filehandle
    assert_clean_parse(r#"open(FH, '<', $file) and print "opened";"#);
}

// === other list-ops + and ===

#[test]
fn test_close_fh_and_next() {
    assert_clean_parse("close($fh) and next;");
}

#[test]
fn test_function_call_and_expr() {
    // Generic 3-arg function call followed by `and`
    assert_clean_parse("foo($a, $b, $c) and bar();");
}

#[test]
fn test_function_call_and_die() {
    assert_clean_parse(r#"do_thing($x, $y) and die "unexpected success";"#);
}

// === and with complex RHS ===

#[test]
fn test_open_and_do_complex() {
    assert_clean_parse(r#"open(my $fh, '<', $file) and do { local $/; <$fh> };"#);
}
