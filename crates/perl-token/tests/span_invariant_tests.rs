use perl_token::{Token, TokenKind, TokenSpan, TokenSpanError};

#[test]
fn try_new_rejects_end_before_start() -> Result<(), Box<dyn std::error::Error>> {
    let err = match Token::try_new(TokenKind::Identifier, "x", 8, 3) {
        Ok(_) => return Err("checked constructor should reject invalid ordering".into()),
        Err(err) => err,
    };
    assert_eq!(err, TokenSpanError::EndBeforeStart { start: 8, end: 3 });
    Ok(())
}

#[test]
fn new_checked_rejects_empty_non_eof_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let err = match Token::new_checked(TokenKind::Identifier, "", 4, 4) {
        Ok(_) => return Err("empty non-EOF token should be rejected".into()),
        Err(err) => err,
    };
    assert_eq!(err, TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 4 });
    Ok(())
}

#[test]
fn new_checked_allows_empty_eof_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let tok = Token::new_checked(TokenKind::Eof, "", 9, 9)?;
    assert_eq!(tok.start, 9);
    assert_eq!(tok.end, 9);
    assert!(tok.is_empty());
    Ok(())
}

#[test]
fn eof_at_preserves_position() -> Result<(), Box<dyn std::error::Error>> {
    let eof = Token::eof_at(123);
    assert_eq!(eof.kind, TokenKind::Eof);
    assert_eq!(eof.start, 123);
    assert_eq!(eof.end, 123);
    assert_eq!(&*eof.text, "");
    Ok(())
}

#[test]
fn unknown_at_supports_synthetic_empty_spans() -> Result<(), Box<dyn std::error::Error>> {
    let unknown = Token::unknown_at("<synthetic>", 17, 17);
    assert_eq!(unknown.kind, TokenKind::Unknown);
    assert_eq!(unknown.start, 17);
    assert_eq!(unknown.end, 17);
    assert!(unknown.is_empty());
    Ok(())
}

#[test]
fn span_helpers_use_byte_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let tok = Token::new(TokenKind::Identifier, "foo", 5, 8);
    let span = tok.span();

    assert_eq!(span, TokenSpan::new(5, 8));
    assert_eq!(span.len(), 3);
    assert!(!span.is_empty());
    assert_eq!(tok.range(), 5..8);
    Ok(())
}

#[test]
fn with_helpers_preserve_invariants() -> Result<(), Box<dyn std::error::Error>> {
    let base = Token::new(TokenKind::Identifier, "name", 10, 14);

    let changed_kind = base.with_kind(TokenKind::Sub);
    assert_eq!(changed_kind.kind, TokenKind::Sub);
    assert_eq!(changed_kind.range(), 10..14);

    let changed_span = changed_kind.with_span(20, 24)?;
    assert_eq!(changed_span.kind, TokenKind::Sub);
    assert_eq!(changed_span.range(), 20..24);

    let err = match changed_span.with_span(24, 20) {
        Ok(_) => return Err("with_span should reject reversed spans".into()),
        Err(err) => err,
    };
    assert_eq!(err, TokenSpanError::EndBeforeStart { start: 24, end: 20 });
    Ok(())
}
