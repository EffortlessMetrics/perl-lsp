mod cpan_test_helpers;

use std::sync::Arc;

use cpan_test_helpers::assert_clean_parse;
use perl_lexer::{PerlLexer, StringPart, TokenType};
use perl_tdd_support::must_some;

#[test]
fn double_quote_incomplete_hash_key() {
    let source = r#"my $msg = "Key: $hash{incomplete";"#;
    assert_clean_parse(source);
}

#[test]
fn double_quote_incomplete_array_index() {
    let source = r#"my $item = "Element: $array[0";"#;
    assert_clean_parse(source);
}

#[test]
fn double_quote_incomplete_nested_arrow_hash_deref() {
    let source = r#"my $nested = "Nested: $obj->{field";"#;
    assert_clean_parse(source);
}

#[test]
fn double_quote_incomplete_mixed_index() {
    let source = r#"my $mixed = "Mixed: $array[$i";"#;
    assert_clean_parse(source);
}

#[test]
fn double_quote_complete_interpolation_still_works() {
    let source = r#"my $ok = "Key: $hash{complete} Element: $array[0] Nested: $obj->{field} Mixed: $array[$i]";"#;
    assert_clean_parse(source);
}

#[test]
fn lexer_preserves_base_variable_parts_for_incomplete_indexing() {
    let src = r#""Key: $hash{incomplete""#;
    let token = must_some(PerlLexer::new(src).next_token());

    assert!(
        matches!(
            token.token_type,
            TokenType::InterpolatedString(ref parts)
                if parts.contains(&StringPart::Variable(Arc::from("$hash")))
        ),
        "expected variable part for $hash, got {:?}",
        token.token_type
    );

    let src = r#""Element: $array[0""#;
    let token = must_some(PerlLexer::new(src).next_token());

    assert!(
        matches!(
            token.token_type,
            TokenType::InterpolatedString(ref parts)
                if parts.contains(&StringPart::Variable(Arc::from("$array")))
        ),
        "expected variable part for $array, got {:?}",
        token.token_type
    );
}
