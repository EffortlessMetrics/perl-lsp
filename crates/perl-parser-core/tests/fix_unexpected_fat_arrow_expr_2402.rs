mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for issue #2402: unexpected_fat_arrow_expr — hash in list context
// Root cause: `=>` used with indirect-call builtins (print/say/printf) where
// an uppercase bareword filehandle is followed by `=>` instead of a space.
// Also covers `=>` as comma in various statement-level list contexts.

// `print FILEHANDLE =>` — fat arrow after uppercase bareword filehandle
#[test]
fn test_print_stderr_fat_arrow() {
    let source = r#"print STDERR => "error message\n";"#;
    assert_clean_parse(source);
}

// `say STDOUT =>` variant
#[test]
fn test_say_stdout_fat_arrow() {
    let source = r#"say STDOUT => "hello\n";"#;
    assert_clean_parse(source);
}

// `printf STDERR =>` variant
#[test]
fn test_printf_stderr_fat_arrow() {
    let source = r#"printf STDERR => "value: %d\n", $val;"#;
    assert_clean_parse(source);
}

// Multiple fat arrow pairs after indirect filehandle
#[test]
fn test_print_fh_fat_arrow_multiple() {
    let source = r#"print LOG => "key: ", $key, " val: ", $val;"#;
    assert_clean_parse(source);
}

// print with variable filehandle and fat arrow (non-uppercase)
#[test]
fn test_print_var_fh_fat_arrow() {
    let source = r#"print $fh => "message\n";"#;
    assert_clean_parse(source);
}
