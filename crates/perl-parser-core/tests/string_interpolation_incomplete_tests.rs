mod cpan_test_helpers;

use cpan_test_helpers::{assert_clean_parse, parse};

#[test]
fn double_quote_incomplete_hash_key() {
    let source = "my $msg = \"Key: $hash{incomplete\";";
    assert_clean_parse(source);

    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(sexp.contains("$hash"), "expected $hash to remain present in parse output: {sexp}");

    let mut parser = perl_parser_core::Parser::new(source);
    let _ = parser.parse();
    assert!(
        !parser.get_errors().is_empty(),
        "expected a diagnostic for incomplete hash interpolation"
    );
}

#[test]
fn double_quote_incomplete_array_index() {
    let source = "my $item = \"Element: $array[0\";";
    assert_clean_parse(source);

    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(sexp.contains("$array"), "expected $array to remain present in parse output: {sexp}");

    let mut parser = perl_parser_core::Parser::new(source);
    let _ = parser.parse();
    assert!(
        !parser.get_errors().is_empty(),
        "expected a diagnostic for incomplete array interpolation"
    );
}

#[test]
fn double_quote_incomplete_arrow_hash_field() {
    let source = "my $msg = \"Nested: $obj->{field\";";
    assert_clean_parse(source);

    let mut parser = perl_parser_core::Parser::new(source);
    let _ = parser.parse();
    assert!(
        !parser.get_errors().is_empty(),
        "expected a diagnostic for incomplete arrow hash interpolation"
    );
}

#[test]
fn double_quote_incomplete_mixed_array_index() {
    let source = "my $msg = \"Mixed: $array[$i\";";
    assert_clean_parse(source);

    let mut parser = perl_parser_core::Parser::new(source);
    let _ = parser.parse();
    assert!(
        !parser.get_errors().is_empty(),
        "expected a diagnostic for incomplete mixed array interpolation"
    );
}

#[test]
fn double_quote_complete_interpolation_still_clean() {
    assert_clean_parse("my $msg = \"Key: $hash{complete}\";");
    assert_clean_parse("my $item = \"Element: $array[0]\";");
    assert_clean_parse("my $msg = \"Nested: $obj->{field}\";");
    assert_clean_parse("my $msg = \"Mixed: $array[$i]\";");
}
