mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_delete_last_expr_in_block() {
    assert_clean_parse(r"sub close { delete $_[0]{cb} }");
}

#[test]
fn test_exists_last_expr_in_block() {
    assert_clean_parse(r"sub has_foo { exists $_[0]{foo} }");
}

#[test]
fn test_delete_in_foreach_block() {
    assert_clean_parse(r"foreach $sym (@names) { delete $imports{$sym} }");
}

#[test]
fn test_exists_in_grep_block() {
    assert_clean_parse(r"grep { exists $h{$_} } @list");
}

#[test]
fn test_say_stderr_in_block() {
    assert_clean_parse(r"sub debug { say STDERR 'debug' }");
}

#[test]
fn test_print_stderr_in_block() {
    assert_clean_parse(r"sub warn_user { print STDERR 'warning' }");
}
