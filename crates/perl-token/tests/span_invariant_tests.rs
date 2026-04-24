use perl_token::{Token, TokenKind, TokenSpan, TokenSpanError};

#[test]
fn checked_constructor_rejects_inverted_span() {
    let result = Token::try_new(TokenKind::Identifier, "name", 8, 4);
    assert_eq!(result, Err(TokenSpanError::InvertedSpan { start: 8, end: 4 }));
}

#[test]
fn checked_constructor_rejects_empty_non_synthetic_span() {
    let result = Token::new_checked(TokenKind::Identifier, "x", 10, 10);
    assert_eq!(
        result,
        Err(TokenSpanError::EmptySpanDisallowed {
            kind: TokenKind::Identifier,
            start: 10,
            end: 10,
        })
    );
}

#[test]
fn checked_constructor_allows_empty_eof() {
    let eof = Token::new_checked(TokenKind::Eof, "", 12, 12);
    assert!(matches!(&eof, Ok(token) if token.kind == TokenKind::Eof && token.is_empty()));
}

#[test]
fn checked_constructor_allows_empty_explicit_synthetic_unknown() {
    let token = Token::new_checked(TokenKind::Unknown, "", 33, 33);
    assert!(matches!(&token, Ok(t) if t.kind == TokenKind::Unknown && t.is_empty()));
}

#[test]
fn eof_at_preserves_position() {
    let eof = Token::eof_at(77);
    assert_eq!(eof.kind, TokenKind::Eof);
    assert_eq!(eof.start, 77);
    assert_eq!(eof.end, 77);
}

#[test]
fn unknown_at_creates_unknown_token_with_span() {
    let token = Token::unknown_at("???", 9, 12);
    assert_eq!(token.kind, TokenKind::Unknown);
    assert_eq!(&*token.text, "???");
    assert_eq!(token.range(), 9..12);
}

#[test]
fn span_helpers_are_byte_range_consistent() {
    let token = Token::new(TokenKind::Identifier, "hé", 3, 6);
    let span = token.span();
    assert_eq!(span, TokenSpan { start: 3, end: 6 });
    assert_eq!(span.range(), 3..6);
    assert_eq!(token.range(), 3..6);
}

#[test]
fn token_with_span_and_with_kind_update_fields() {
    let token = Token::new(TokenKind::Identifier, "name", 1, 5);
    assert_eq!(TokenSpan::new(10, 14), Ok(TokenSpan { start: 10, end: 14 }));
    let span = TokenSpan { start: 10, end: 14 };
    let updated = token.clone().with_span(span);
    assert_eq!(updated.start, 10);
    assert_eq!(updated.end, 14);
    assert_eq!(updated.kind, TokenKind::Identifier);

    let kind_swapped = token.with_kind(TokenKind::Unknown);
    assert_eq!(kind_swapped.kind, TokenKind::Unknown);
    assert_eq!(kind_swapped.start, 1);
    assert_eq!(kind_swapped.end, 5);
}

#[test]
fn token_len_remains_saturating_for_compatibility() {
    let token = Token::new(TokenKind::Unknown, "?", 20, 3);
    assert_eq!(token.len(), 0);
}
