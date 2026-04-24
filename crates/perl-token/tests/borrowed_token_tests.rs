//! Tests for borrowed token views.

use perl_token::{Token, TokenKind, TokenRef};

#[test]
fn borrowed_token_fields_and_helpers() -> Result<(), Box<dyn std::error::Error>> {
    let tok = TokenRef::new(TokenKind::Identifier, "hello", 3, 8);

    assert_eq!(tok.kind, TokenKind::Identifier);
    assert_eq!(tok.text, "hello");
    assert_eq!(tok.len(), 5);
    assert!(!tok.is_empty());
    assert_eq!(tok.span(), (3, 8));
    assert_eq!(tok.display_name(), "identifier");

    Ok(())
}

#[test]
fn borrowed_token_handles_empty_and_malformed_spans() -> Result<(), Box<dyn std::error::Error>> {
    let empty = TokenRef::new(TokenKind::Eof, "", 10, 10);
    assert!(empty.is_empty());

    let malformed = TokenRef::new(TokenKind::Unknown, "?", 20, 10);
    assert_eq!(malformed.len(), 0);

    Ok(())
}

#[test]
fn borrowed_token_to_owned_conversion_is_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let borrowed = TokenRef::new(TokenKind::My, "my", 0, 2);

    let owned = borrowed.to_owned_token();
    assert_eq!(owned, Token::new(TokenKind::My, "my", 0, 2));

    let via_from: Token = borrowed.into();
    assert_eq!(via_from, Token::new(TokenKind::My, "my", 0, 2));

    Ok(())
}

#[test]
fn owned_token_can_be_viewed_as_borrowed() -> Result<(), Box<dyn std::error::Error>> {
    let owned = Token::new(TokenKind::String, "\"abc\"", 11, 16);

    let borrowed = owned.as_ref_token();
    assert_eq!(borrowed.kind, TokenKind::String);
    assert_eq!(borrowed.text, "\"abc\"");
    assert_eq!(borrowed.span(), (11, 16));
    assert_eq!(borrowed.display_name(), "string");

    Ok(())
}

#[test]
fn owned_token_helpers_still_match_kind_and_span() -> Result<(), Box<dyn std::error::Error>> {
    let tok = Token::new(TokenKind::LeftBrace, "{", 42, 43);
    assert_eq!(tok.span(), (42, 43));
    assert_eq!(tok.display_name(), "'{'");

    Ok(())
}
