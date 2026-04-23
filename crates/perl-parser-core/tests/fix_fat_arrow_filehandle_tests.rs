mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #2392: print STDERR => "msg" was misclassified as indirect-call syntax
// because the is_indirect_call_pattern() guard checked for Comma and Arrow but
// not FatArrow. Adding FatArrow to the short-circuit guard fixes this.

#[test]
fn test_print_stderr_fat_arrow() {
    let source = r#"print STDERR => "error message\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_say_stdout_fat_arrow() {
    let source = r#"say STDOUT => "hello\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_printf_stderr_fat_arrow() {
    let source = r#"printf STDERR => "%s\n", $msg;"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_filehandle_fat_arrow_in_sub() {
    let source = r#"
sub log_error {
    my ($msg) = @_;
    print STDERR => $msg;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_stderr_fat_arrow_with_concat() {
    let source = r#"print STDERR => "Error: " . $err . "\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_stdout_fat_arrow() {
    let source = r#"print STDOUT => "message\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_comma_still_works() {
    // Ensure comma separator still treated as regular call (not indirect)
    let source = r#"print STDERR, "message\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_indirect_object_still_works() {
    // print STDERR "msg" (no fat arrow, no comma) should still work as indirect object
    let source = r#"print STDERR "direct message\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_fat_arrow_multiple_args() {
    let source = r#"print STDERR => "val=%s\n", $val;"#;
    assert_clean_parse(source);
}
