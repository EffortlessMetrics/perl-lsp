mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn split_bare_regex_separator_parses_cleanly() {
    assert_clean_parse("my @parts = split /,/, $string;");
}

#[test]
fn join_bare_regex_separator_parses_cleanly() {
    assert_clean_parse("my $joined = join /,/, @parts;");
}

#[test]
fn grep_bare_regex_expression_parses_cleanly() {
    assert_clean_parse("my @matches = grep /pattern/, @list;");
}

#[test]
fn map_bare_regex_expression_parses_cleanly() {
    assert_clean_parse("my @flags = map /pattern/, @list;");
}

#[test]
fn print_bare_regex_argument_parses_cleanly() {
    assert_clean_parse("print /pattern/;");
}
