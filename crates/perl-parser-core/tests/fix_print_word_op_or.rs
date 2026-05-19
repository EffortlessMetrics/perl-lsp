mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn print_or_die_in_while_is_clean() {
    // Pod::Perldoc.pm pattern: zero-arg print, then low-precedence word-or.
    assert_clean_parse(
        r#"
while (<$fh>) {
    print or die "Can't print: $!";
}
"#,
    );
}

#[test]
fn close_fh_or_die_is_clean() {
    assert_clean_parse(r#"close $fh or die "Can't close: $!";"#);
}

#[test]
fn bare_zero_arg_builtins_before_word_operators_are_clean() {
    assert_clean_parse(r#"print or die "error";"#);
    assert_clean_parse(r#"say or die "error";"#);
    assert_clean_parse(r#"write or die "Can't write";"#);
    assert_clean_parse(r#"print and next;"#);
}

#[test]
fn print_with_actual_args_still_parses() {
    assert_clean_parse(r#"print "hello\n";"#);
    assert_clean_parse(r#"print $fh "hello\n";"#);
    assert_clean_parse(r#"print $_ or die "error";"#);
    assert_clean_parse(r#"print STDOUT "hello\n";"#);
}
