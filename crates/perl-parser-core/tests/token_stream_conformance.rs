use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};
use perl_tdd_support::must;
use std::sync::Arc;

#[test]
fn keyword_and_operator_tokens_use_canonical_mappings() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = vec![
        LexerToken::new(LexerTokenType::Keyword(Arc::from("my")), "my", 0, 2),
        LexerToken::new(LexerTokenType::Operator(Arc::from("=")), "=", 3, 4),
        LexerToken::new(LexerTokenType::Keyword(Arc::from("and")), "and", 5, 8),
    ];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(tokens);

    assert_eq!(parser_tokens[0].kind, TokenKind::My);
    assert_eq!(parser_tokens[1].kind, TokenKind::Assign);
    assert_eq!(parser_tokens[2].kind, TokenKind::WordAnd);
    Ok(())
}

#[test]
fn delimiter_and_sigil_fallbacks_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = vec![
        LexerToken::new(LexerTokenType::Identifier(Arc::from("$")), "$", 0, 1),
        LexerToken::new(LexerTokenType::Identifier(Arc::from("&")), "&", 1, 2),
        LexerToken::new(LexerTokenType::Error(Arc::from("unexpected")), "{", 2, 3),
    ];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(tokens);

    assert_eq!(parser_tokens[0].kind, TokenKind::ScalarSigil);
    assert_eq!(parser_tokens[1].kind, TokenKind::SubSigil);
    assert_eq!(parser_tokens[2].kind, TokenKind::LeftBrace);
    Ok(())
}

#[test]
fn quote_like_keyword_remains_identifier_in_stream() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = vec![LexerToken::new(LexerTokenType::Keyword(Arc::from("qw")), "qw", 0, 2)];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(tokens);
    assert_eq!(parser_tokens[0].kind, TokenKind::Identifier);

    let mut stream = TokenStream::new("qw");
    assert_eq!(must(stream.next()).kind, TokenKind::Identifier);
    Ok(())
}
