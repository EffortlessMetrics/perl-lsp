mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_print_or_die_in_while() {
    // Pod::Perldoc.pm pattern: print with no args (prints $_) followed by word-or
    assert_clean_parse(
        r#"
while (<$fh>) {
    print or die "Can't print: $!";
}
"#,
    );
}

#[test]
fn test_close_fh_or_die() {
    // Pod::Perldoc.pm pattern: close with filehandle arg followed by word-or
    assert_clean_parse(r#"close $fh or die "Can't close: $!";"#);
}

#[test]
fn test_print_or_die_bare() {
    // Minimal regression for word-or after zero-arg builtin
    assert_clean_parse(r#"print or die "error";"#);
}

#[test]
fn test_say_or_die() {
    assert_clean_parse(r#"say or die "error";"#);
}

#[test]
fn test_print_and_next() {
    // word-and variant
    assert_clean_parse(r#"print and next;"#);
}

#[test]
fn test_write_or_die() {
    assert_clean_parse(r#"write or die "Can't write";"#);
}

#[test]
fn test_print_with_args_still_works() {
    // Regression: print with actual args should not be broken
    assert_clean_parse(r#"print "hello\n";"#);
    assert_clean_parse(r#"print $fh "hello\n";"#);
    assert_clean_parse(r#"print $_ or die "error";"#);
    assert_clean_parse(r#"print STDOUT "hello\n";"#);
}
