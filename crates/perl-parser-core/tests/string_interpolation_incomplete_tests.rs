mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;

#[test]
fn double_quote_incomplete_hash_key() {
    assert_clean_parse(r#"my $msg = "Key: $hash{incomplete";"#);
}

#[test]
fn double_quote_incomplete_array_index() {
    assert_clean_parse(r#"my $item = "Element: $array[0";"#);
}

#[test]
fn double_quote_incomplete_arrow_hash_key() {
    assert_clean_parse(r#"my $msg = "Nested: $obj->{field";"#);
}

#[test]
fn double_quote_incomplete_mixed_index() {
    assert_clean_parse(r#"my $msg = "Mixed: $array[$i";"#);
}

#[test]
fn double_quote_complete_interpolation_still_clean() {
    assert_clean_parse(r#"my $msg = "Key: $hash{complete} and $array[0]";"#);
    assert_clean_parse(r#"my $msg = "Nested: $obj->{field}[$i]";"#);
}
