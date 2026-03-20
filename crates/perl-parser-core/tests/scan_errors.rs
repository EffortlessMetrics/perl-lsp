mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_our_or_assign() {
    assert_clean_parse(r#"our $Verbose ||= 0;"#);
}

#[test]
fn test_our_defined_or_assign() {
    assert_clean_parse(r#"our $DYNAMIC_FILE_UPLOAD ||= 0;"#);
}

#[test]
fn test_my_dotassign() {
    assert_clean_parse(r#"my $result .= "hello";"#);
}

#[test]
fn test_eval_block_and_operator() {
    assert_clean_parse(r#"eval { 1 } && print("ok");"#);
}

#[test]
fn test_do_block_or_die() {
    assert_clean_parse(r#"do { require "config.pl" } || die "failed";"#);
}
