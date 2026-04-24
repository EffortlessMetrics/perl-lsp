use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};

#[test]
fn token_stream_uses_shared_keyword_mapping() {
    let raw = vec![LexerToken::new(LexerTokenType::Keyword("defer".into()), "defer", 0, 5)];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    assert_eq!(parser_tokens.len(), 1);
    assert_eq!(parser_tokens[0].kind, TokenKind::Defer);
}

#[test]
fn token_stream_uses_shared_operator_mapping() {
    let raw = vec![LexerToken::new(LexerTokenType::Operator("//=".into()), "//=", 0, 3)];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    assert_eq!(parser_tokens.len(), 1);
    assert_eq!(parser_tokens[0].kind, TokenKind::DefinedOrAssign);
}

#[test]
fn token_stream_uses_shared_delimiter_mapping() {
    let raw = vec![LexerToken::new(LexerTokenType::LeftBracket, "[", 0, 1)];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    assert_eq!(parser_tokens.len(), 1);
    assert_eq!(parser_tokens[0].kind, TokenKind::LeftBracket);
}

#[test]
fn token_stream_keeps_contextual_qw_as_identifier() {
    let raw = vec![LexerToken::new(LexerTokenType::Keyword("qw".into()), "qw", 0, 2)];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    assert_eq!(parser_tokens.len(), 1);
    assert_eq!(parser_tokens[0].kind, TokenKind::Identifier);
}

#[test]
fn token_stream_maps_identifier_sigils_via_shared_helper() {
    let raw = vec![LexerToken::new(LexerTokenType::Identifier("%".into()), "%", 0, 1)];
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    assert_eq!(parser_tokens.len(), 1);
    assert_eq!(parser_tokens[0].kind, TokenKind::HashSigil);
}
