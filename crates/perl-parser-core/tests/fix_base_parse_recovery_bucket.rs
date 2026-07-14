//! Regression tests for receipt-backed Perl core `base` parse-recovery gaps.
//!
//! These patterns come from the real upstream `base` smoke receipt:
//! - `base/lex.t`: spaced braced scalar deref of a `^` special variable
//! - `base/term.t`: old-style bareword filehandle named `try`

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn base_lex_spaced_braced_special_scalar_parses_cleanly() {
    assert_clean_parse(r#"if ($ {^XY} != 23) { print "not " }"#);
}

#[test]
fn base_lex_spaced_symbolic_scalar_deref_stays_clean() {
    assert_clean_parse(r#"$ {$CX} = 17; $ {$CXY} = 23;"#);
}

#[test]
fn base_lex_subroutine_with_empty_package_segments_parses_cleanly() {
    assert_clean_parse(r#"sub foo::::::bar { print "ok" } foo::::::bar;"#);
}

#[test]
fn base_term_open_try_bareword_filehandle_parses_cleanly() {
    assert_clean_parse(
        r#"open(try, "/dev/null") || open(try,"nla0:") || (die "Can't open /dev/null.");"#,
    );
}
