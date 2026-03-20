mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_else_as_hash_key() {
    assert_clean_parse(r#"my %h = (else => 1);"#);
}

#[test]
fn test_elsif_as_hash_key() {
    assert_clean_parse(r#"my %h = (elsif => 1);"#);
}

#[test]
fn test_do_as_hash_key() {
    assert_clean_parse(r#"my %h = (do => 1);"#);
}

#[test]
fn test_eval_as_hash_key() {
    assert_clean_parse(r#"my %h = (eval => 1);"#);
}

#[test]
fn test_require_as_hash_key() {
    assert_clean_parse(r#"my %h = (require => "foo");"#);
}

#[test]
fn test_glob_assignment_special_var() {
    assert_clean_parse(r#"*ARG = *_ ;"#);
}

#[test]
fn test_print_method_call() {
    assert_clean_parse(r#"$fh->print("hello");"#);
}

#[test]
fn test_postfix_if_after_increment() {
    assert_clean_parse(r#"$count++ if $enabled;"#);
}
