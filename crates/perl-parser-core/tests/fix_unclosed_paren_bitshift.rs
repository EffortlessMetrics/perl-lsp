//! Tests for issue #2750 Pattern A: `<<` bitshift vs heredoc inside parenthesized expressions.
//!
//! Root cause: `try_heredoc()` in `crates/perl-lexer/src/lib.rs` does not check
//! `self.mode`. After `(` the lexer is in `ExpectTerm` mode. When the expression is
//! `(1<<index(...))`, the `1` sets `ExpectOperator` mode, but the subsequent `<<` is
//! seen and `index` starts with an identifier char, so the lexer classifies it as a
//! heredoc `<<index` instead of the bitshift operator `<<`.
//!
//! Fix: Add a mode guard at the top of `try_heredoc()` — return `None` immediately
//! when `self.mode == LexerMode::ExpectOperator`.
//!
//! Affected corpus files: `Devel/Peek.pm`, `ExtUtils/MM_Any.pm`

mod cpan_test_helpers;
use cpan_test_helpers::*;

// -- Failing cases: `<<` inside parens after a term --

#[test]
fn test_bitshift_in_paren_with_index_call() {
    // The primary reproducer from Devel/Peek.pm
    assert_clean_parse(r#"$x |= (1<<index($D_flags, $_));"#);
}

#[test]
fn test_bitshift_in_paren_list() {
    // Multiple bitshift results in a list context
    assert_clean_parse(r#"my @a = (1<<foo(), 1<<bar());"#);
}

#[test]
fn test_bitshift_in_if_condition() {
    // Bitshift inside an if-condition paren
    assert_clean_parse(r#"if (1<<length($s)) { }"#);
}

#[test]
fn test_bitshift_in_return() {
    // Bitshift as return expression
    assert_clean_parse(r#"return (1<<foo());"#);
}

// -- Additional edge cases --

#[test]
fn test_left_shift_assign_in_paren() {
    // <<= (left-shift-assign) after a term must not be treated as heredoc start
    assert_clean_parse(r#"$x <<= 3;"#);
}

#[test]
fn test_left_shift_assign_in_paren_expr() {
    // <<= inside paren expression
    assert_clean_parse(r#"($flags <<= $n);"#);
}

#[test]
fn test_bitshift_after_array_subscript() {
    // ] also sets ExpectOperator — bitshift after array element
    assert_clean_parse(r#"my $r = $arr[0] << func();"#);
}

#[test]
fn test_print_variable_fh_heredoc_regression() {
    // print $fh <<END — $fh sets ExpectOperator, but the parser must handle
    // the filehandle-then-heredoc pattern correctly.
    // Note: the parser sees $fh as the indirect object and <<END starts
    // a heredoc. Verify clean parse.
    assert_clean_parse("print $fh <<END;\nhello\nEND\n");
}

#[test]
fn test_heredoc_after_ternary_regression() {
    // <<END after ? — ? sets ExpectTerm, so heredoc must be recognized
    assert_clean_parse("my $x = $cond ? <<END : \"default\";\nhello\nEND\n");
}

// -- Regression: actual heredocs must still work --

#[test]
fn test_heredoc_as_first_arg_regression() {
    // Actual heredoc in paren context must not regress
    assert_clean_parse("foo(<<END, $x);\nhello\nEND\n");
}

#[test]
fn test_heredoc_in_list_regression() {
    // Actual heredoc in list context must not regress
    assert_clean_parse("my @a = (<<END, $x);\nhello\nEND\n");
}

#[test]
fn test_bitshift_at_statement_level_regression() {
    // Statement-level bitshift (already worked; must still work)
    assert_clean_parse(r#"$x = 1<<2;"#);
}

#[test]
fn test_bitshift_with_variable_rhs_regression() {
    // Variable RHS: already worked; must still work
    assert_clean_parse(r#"$x = 1 << $y;"#);
}

#[test]
fn test_bitshift_chained_regression() {
    // ($a << $b << $c) — both << are operators; must not treat second as heredoc
    assert_clean_parse(r#"my $x = ($a << $b << $c);"#);
}

#[test]
fn test_indented_heredoc_in_paren_regression() {
    // (<<~END) — indented heredoc in paren context must still be recognized
    assert_clean_parse("my $x = (<<~END);\n  hello\n  END\n");
}

#[test]
fn test_bitshift_after_array_subscript_in_paren() {
    // paren_depth > 0 AND ] sets ExpectOperator — both conditions must fire the guard
    // `($arr[0] << func())` — `[` doesn't increment paren_depth, but outer `(` does.
    // After `]`, mode=ExpectOperator, paren_depth=1, so `<<` must be treated as bitshift.
    assert_clean_parse(r#"my $x = ($arr[0] << func());"#);
}

#[test]
fn test_bitshift_after_hash_subscript_in_paren() {
    // paren_depth > 0 AND `}` sets ExpectOperator — guard must fire
    // `($hash{key} << $n)` — `{`/`}` don't touch paren_depth; outer `(` means depth=1.
    assert_clean_parse(r#"my $x = ($hash{key} << $n);"#);
}
