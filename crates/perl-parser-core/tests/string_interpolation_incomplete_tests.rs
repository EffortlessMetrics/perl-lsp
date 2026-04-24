mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;

#[test]
fn double_quote_incomplete_hash_key() {
    assert_clean_parse("my $msg = \"Key: $hash{incomplete\";");
}

#[test]
fn double_quote_incomplete_array_index() {
    assert_clean_parse("my $item = \"Element: $array[0\";");
}

#[test]
fn double_quote_incomplete_nested_arrow_hash_field() {
    assert_clean_parse("my $msg = \"Nested: $obj->{field\";");
}

#[test]
fn double_quote_incomplete_mixed_array_index_expr() {
    assert_clean_parse("my $msg = \"Mixed: $array[$i\";");
}

#[test]
fn double_quote_complete_interpolation_still_parses_cleanly() {
    assert_clean_parse("my $msg = \"Key: $hash{complete}; Element: $array[0]; Nested: $obj->{field}; Mixed: $array[$i]\";");
}
