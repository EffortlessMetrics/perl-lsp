mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_use_vstring() {
    let source = r#"use v5.38.0;"#;
    assert_clean_parse(source);
}

#[test]
fn test_vstring_in_expression() {
    let source = r#"my $v = v1.2.3;"#;
    assert_clean_parse(source);
}

#[test]
fn test_vstring_comparison() {
    let source = r#"$^V ge v5.10.0"#;
    assert_clean_parse(source);
}
