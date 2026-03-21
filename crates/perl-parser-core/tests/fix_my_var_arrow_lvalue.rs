mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn my_scalar_arrow_hash_subscript() {
    assert_clean_parse(r#"my $cache->{key} = [1,2,3];"#);
}

#[test]
fn my_scalar_arrow_nested_subscript() {
    assert_clean_parse(r#"my $obj->{a}{b} = 'val';"#);
}

#[test]
fn my_scalar_arrow_method_call() {
    // my $foo->method() is unusual but syntactically valid Perl
    assert_clean_parse(r#"my $foo->init();"#);
}
