use perl_token::{Token, TokenKind, TokenSpan, TokenSpanError};
use std::ops::Range;

#[test]
fn checked_constructor_rejects_inverted_span() {
    let err = Token::try_new(TokenKind::Identifier, "foo", 5, 2)
        .expect_err("checked constructors should reject end < start");
    assert_eq!(err, TokenSpanError::Inverted { start: 5, end: 2 });
}

#[test]
fn checked_constructor_rejects_empty_non_synthetic_token() {
    let err = Token::new_checked(TokenKind::Identifier, "", 4, 4)
        .expect_err("checked constructors should reject empty non-synthetic tokens");
    assert_eq!(err, TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, pos: 4 });
}

#[test]
fn checked_constructor_allows_empty_for_eof() {
    let tok =
        Token::new_checked(TokenKind::Eof, "", 9, 9).expect("EOF should allow empty checked spans");
    assert_eq!(tok.start, 9);
    assert_eq!(tok.end, 9);
    assert!(tok.is_empty());
}

#[test]
fn eof_at_preserves_position() {
    let tok = Token::eof_at(27);
    assert_eq!(tok.kind, TokenKind::Eof);
    assert_eq!(tok.start, 27);
    assert_eq!(tok.end, 27);
    assert_eq!(tok.range(), Range { start: 27, end: 27 });
}

#[test]
fn unknown_at_supports_synthetic_tokens() {
    let tok = Token::unknown_at("???", 12, 12);
    assert_eq!(tok.kind, TokenKind::Unknown);
    assert_eq!(&*tok.text, "???");
    assert!(tok.is_empty());
}

#[test]
fn span_and_range_use_byte_offsets() {
    let tok = Token::new(TokenKind::String, "hé", 10, 14);
    let span = tok.span();
    assert_eq!(span, TokenSpan { start: 10, end: 14 });
    assert_eq!(span.len(), 4);
    assert_eq!(tok.range(), 10..14);
}

#[test]
fn with_span_returns_checked_copy() {
    let tok = Token::new(TokenKind::Identifier, "name", 0, 4);
    let shifted = tok.with_span(8, 12).expect("valid span should succeed");
    assert_eq!(shifted.kind, TokenKind::Identifier);
    assert_eq!(&*shifted.text, "name");
    assert_eq!(shifted.range(), 8..12);
}

#[test]
fn with_span_rejects_invalid_range() {
    let tok = Token::new(TokenKind::Identifier, "name", 0, 4);
    let err = tok.with_span(3, 1).expect_err("invalid range must fail");
    assert_eq!(err, TokenSpanError::Inverted { start: 3, end: 1 });
}

#[test]
fn with_kind_preserves_text_and_span() {
    let tok = Token::new(TokenKind::Identifier, "foo", 2, 5);
    let retyped = tok.with_kind(TokenKind::String);
    assert_eq!(retyped.kind, TokenKind::String);
    assert_eq!(&*retyped.text, "foo");
    assert_eq!(retyped.start, 2);
    assert_eq!(retyped.end, 5);
}

#[test]
fn token_span_new_rejects_inverted_ranges() {
    let err = TokenSpan::new(11, 3).expect_err("inverted span should be rejected");
    assert_eq!(err, TokenSpanError::Inverted { start: 11, end: 3 });
}
