//! Regression tests: `<$fh>` must be classified as `Readline`, not `Glob`.
//!
//! Per perlop: a simple scalar variable inside `<...>` is an indirect filehandle
//! read — the scalar holds the filehandle reference.  Only patterns with glob
//! metacharacters (`* ? [ . / { }`) or multiple tokens remain `Glob`.
//!
//! Issue: the angle-bracket classifier in `primary.rs` fell through to the
//! default Glob branch for any pattern starting with `$`, because the
//! uppercase-only check (`c.is_uppercase() || c == '_'`) correctly matched
//! `STDIN`/`FH` but silently excluded lowercase-named scalars like `$fh`.

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::Parser;
use perl_tdd_support::must;

// -----------------------------------------------------------------------
// Helper: parse `<EXPR>` as a standalone expression statement and return the
// sexp of the expression so we can assert on node kind without pattern-matching
// the full declaration wrapper.
// -----------------------------------------------------------------------

fn parse_expr_sexp(source: &str) -> String {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    // The test snippets are full statements (`my $x = <$fh>;` or `<$fh>;`),
    // so we use to_sexp() on the root and verify the substring we care about.
    ast.to_sexp()
}

// -----------------------------------------------------------------------
// Core cases: simple scalars MUST become Readline after the fix
// -----------------------------------------------------------------------

/// `<$fh>` — simple lowercase scalar is an indirect filehandle read.
#[test]
fn test_angle_bracket_simple_scalar_lowercase_is_readline() {
    let sexp = parse_expr_sexp("my $line = <$fh>;");
    assert!(sexp.contains("(readline $fh)"), "<$fh> should produce (readline $fh) but got: {sexp}");
    assert!(!sexp.contains("(glob $fh)"), "<$fh> must NOT produce (glob $fh) but got: {sexp}");
}

/// `<$FH>` — uppercase scalar is still a simple scalar, not a bareword filehandle.
#[test]
fn test_angle_bracket_simple_scalar_uppercase_is_readline() {
    let sexp = parse_expr_sexp("my $line = <$FH>;");
    assert!(sexp.contains("(readline $FH)"), "<$FH> should produce (readline $FH) but got: {sexp}");
    assert!(!sexp.contains("(glob $FH)"), "<$FH> must NOT produce (glob $FH) but got: {sexp}");
}

/// `<$pattern>` — the variable name `pattern` looks like a glob pattern word,
/// but the `$` sigil makes it a simple scalar → Readline.
#[test]
fn test_angle_bracket_simple_scalar_pattern_name_is_readline() {
    let sexp = parse_expr_sexp("my $line = <$pattern>;");
    assert!(
        sexp.contains("(readline $pattern)"),
        "<$pattern> should produce (readline $pattern) but got: {sexp}"
    );
    assert!(
        !sexp.contains("(glob $pattern)"),
        "<$pattern> must NOT produce (glob $pattern) but got: {sexp}"
    );
}

// -----------------------------------------------------------------------
// Regression guards: these must continue to work correctly after the fix
// -----------------------------------------------------------------------

/// `<STDIN>` — bareword uppercase filehandle → Readline.  Must not regress.
#[test]
fn test_angle_bracket_bareword_filehandle_stays_readline() {
    let sexp = parse_expr_sexp("my $x = <STDIN>;");
    assert!(
        sexp.contains("(readline STDIN)"),
        "<STDIN> should remain (readline STDIN) but got: {sexp}"
    );
}

/// `<*.pm>` — glob metacharacter → Glob.  Must not regress.
#[test]
fn test_angle_bracket_glob_pattern_stays_glob() {
    let sexp = parse_expr_sexp(r#"my @f = <*.pm>;"#);
    assert!(sexp.contains("(glob *.pm)"), "<*.pm> should remain (glob *.pm) but got: {sexp}");
}

/// `<$dir/*>` — scalar + path separator + glob metachar → Glob.
/// The `/` and `*` disqualify it from being a simple scalar.
#[test]
fn test_angle_bracket_scalar_with_glob_chars_stays_glob() {
    let sexp = parse_expr_sexp(r#"my @f = <$dir/*>;"#);
    assert!(sexp.contains("(glob"), "<$dir/*> should remain a glob but got: {sexp}");
    assert!(!sexp.contains("(readline"), "<$dir/*> must NOT become readline but got: {sexp}");
}

/// `<$h{key}>` — hash subscript access, not a simple scalar → stays Glob.
/// The `{` disqualifies it from the simple-scalar fast path.
#[test]
fn test_angle_bracket_hash_subscript_stays_glob() {
    let sexp = parse_expr_sexp(r#"my @v = <$h{key}>;"#);
    assert!(sexp.contains("(glob"), "<$h{{key}}> should remain a glob but got: {sexp}");
    assert!(!sexp.contains("(readline"), "<$h{{key}}> must NOT become readline but got: {sexp}");
}

/// `<>` — diamond operator.  Must not regress.
#[test]
fn test_angle_bracket_empty_stays_diamond() {
    let sexp = parse_expr_sexp("my $x = <>;");
    assert!(sexp.contains("(diamond)"), "<> should remain (diamond) but got: {sexp}");
}

// -----------------------------------------------------------------------
// Additional coverage: package-qualified scalar still classified as simple scalar
// -----------------------------------------------------------------------

/// `<$Foo::bar>` — package-qualified scalar.  perlop treats it the same as a
/// simple scalar for readline purposes (the qualifier is part of the identifier).
#[test]
fn test_angle_bracket_qualified_scalar_is_readline() {
    let sexp = parse_expr_sexp("my $line = <$Foo::bar>;");
    assert!(
        sexp.contains("(readline $Foo::bar)"),
        "<$Foo::bar> should produce (readline $Foo::bar) but got: {sexp}"
    );
}

// -----------------------------------------------------------------------
// Clean-parse guards: no Error nodes in any of these forms
// -----------------------------------------------------------------------

#[test]
fn test_readline_forms_parse_cleanly() {
    assert_clean_parse("my $line = <$fh>;");
    assert_clean_parse("my $line = <$FH>;");
    assert_clean_parse("my $line = <$pattern>;");
    assert_clean_parse("my $x = <STDIN>;");
    assert_clean_parse("my @f = <*.pm>;");
    assert_clean_parse("my @f = <$dir/*>;");
    assert_clean_parse("my $x = <>;");
}
