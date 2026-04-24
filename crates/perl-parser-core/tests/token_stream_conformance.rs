use perl_parser_core::token_stream::{TokenKind, TokenStream};
use perl_tdd_support::must;

#[test]
fn keyword_and_word_operator_mappings_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x and $y");

    assert_eq!(must(stream.next()).kind, TokenKind::My);
    assert_eq!(must(stream.next()).kind, TokenKind::ScalarSigil);
    assert_eq!(must(stream.next()).kind, TokenKind::Identifier);
    assert_eq!(must(stream.next()).kind, TokenKind::WordAnd);

    Ok(())
}

#[test]
fn quote_like_keyword_stays_identifier_when_not_quote_op() -> Result<(), Box<dyn std::error::Error>>
{
    let mut stream = TokenStream::new("qw => 1");

    assert_eq!(must(stream.next()).kind, TokenKind::Identifier);
    assert_eq!(must(stream.next()).kind, TokenKind::FatArrow);
    assert_eq!(must(stream.next()).kind, TokenKind::Number);

    Ok(())
}

#[test]
fn bare_identifier_sigils_keep_previous_classification() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("$ @ % & *");

    assert_eq!(must(stream.next()).kind, TokenKind::ScalarSigil);
    assert_eq!(must(stream.next()).kind, TokenKind::ArraySigil);
    assert_eq!(must(stream.next()).kind, TokenKind::Percent);
    assert_eq!(must(stream.next()).kind, TokenKind::BitwiseAnd);
    assert_eq!(must(stream.next()).kind, TokenKind::Star);

    Ok(())
}
