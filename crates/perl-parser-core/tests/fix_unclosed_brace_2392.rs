//! Tests for issue #2392 — print/say/printf with uppercase bareword filehandle
//! followed by fat-arrow (=>) were incorrectly treated as indirect call syntax.
//!
//! Root cause: `is_indirect_call_pattern()` in calls.rs checked for Comma and Arrow
//! but not FatArrow when deciding whether `print STDERR => ...` is a list call.

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn print_stderr_fat_arrow() {
    assert_clean_parse(r#"print STDERR => "error message\n";"#);
}

#[test]
fn warn_stderr_fat_arrow() {
    assert_clean_parse(r#"warn STDERR => "something wrong\n";"#);
}

#[test]
fn say_stdout_fat_arrow() {
    assert_clean_parse(r#"say STDOUT => "line\n";"#);
}

#[test]
fn printf_stderr_fat_arrow() {
    assert_clean_parse(r#"printf STDERR => "%s\n", $msg;"#);
}

#[test]
fn print_stdout_fat_arrow_multi_args() {
    assert_clean_parse(r#"print STDOUT => $scalar, "\n";"#);
}

#[test]
fn print_custom_fh_fat_arrow() {
    assert_clean_parse(r#"print FILE => "data";"#);
}
