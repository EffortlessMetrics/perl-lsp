mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_lexer::{PerlLexer, StringPart, TokenType};
use perl_tdd_support::must_some;

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
    assert_clean_parse(r#"my $nested = "Nested: $obj->{field";"#);
}

#[test]
fn double_quote_mixed_incomplete_array_index_expr() {
    assert_clean_parse(r#"my $mixed = "Mixed: $array[$i";"#);
}

#[test]
fn double_quote_complete_interpolation_still_parses() {
    assert_clean_parse(
        r#"my $ok = "Key: $hash{complete} Element: $array[0] Nested: $obj->{field} Mixed: $array[$i]";"#,
    );
}

#[test]
fn incomplete_interpolation_still_emits_variable_parts() {
    let source = r#"my $msg = "Key: $hash{incomplete"; my $item = "Element: $array[0";"#;
    let mut lexer = PerlLexer::new(source);
    let tokens = lexer.collect_tokens();
    let token_texts: Vec<String> = tokens.iter().map(|t| t.text.to_string()).collect();

    let hash_token =
        must_some(tokens.iter().find(|t| t.text.as_ref() == "\"Key: $hash{incomplete\""));
    let array_token = must_some(tokens.iter().find(|t| t.text.as_ref() == "\"Element: $array[0\""));

    assert!(
        matches!(&hash_token.token_type, TokenType::InterpolatedString(_)),
        "Expected interpolated string token for hash case. token stream: {:?}",
        token_texts
    );
    if let TokenType::InterpolatedString(parts) = &hash_token.token_type {
        assert!(
            parts
                .iter()
                .any(|part| matches!(part, StringPart::Variable(v) if v.as_ref() == "$hash")),
            "Expected $hash interpolation part. token stream: {:?}",
            token_texts
        );
    }

    assert!(
        matches!(&array_token.token_type, TokenType::InterpolatedString(_)),
        "Expected interpolated string token for array case. token stream: {:?}",
        token_texts
    );
    if let TokenType::InterpolatedString(parts) = &array_token.token_type {
        assert!(
            parts
                .iter()
                .any(|part| matches!(part, StringPart::Variable(v) if v.as_ref() == "$array")),
            "Expected $array interpolation part. token stream: {:?}",
            token_texts
        );
    }
}
