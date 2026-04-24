use perl_token::{Token, TokenKind, TokenSpan};

#[test]
fn checked_constructors_reject_reversed_spans() -> Result<(), Box<dyn std::error::Error>> {
    assert!(Token::try_new(TokenKind::Identifier, "x", 5, 2).is_err());
    assert!(Token::new_checked(TokenKind::Identifier, "x", 5, 2).is_err());
    assert!(TokenSpan::try_new(5, 2).is_err());
    Ok(())
}

#[test]
fn checked_constructors_reject_empty_non_synthetic_tokens() -> Result<(), Box<dyn std::error::Error>>
{
    assert!(Token::try_new(TokenKind::Identifier, "", 9, 9).is_err());
    assert!(Token::new_checked(TokenKind::Semicolon, ";", 1, 1).is_err());
    Ok(())
}

#[test]
fn checked_constructors_allow_empty_eof_and_unknown_tokens()
-> Result<(), Box<dyn std::error::Error>> {
    let eof = Token::try_new(TokenKind::Eof, "", 11, 11)?;
    let unknown = Token::new_checked(TokenKind::Unknown, "", 11, 11)?;
    assert!(eof.is_empty());
    assert!(unknown.is_empty());
    Ok(())
}

#[test]
fn eof_at_preserves_requested_position() -> Result<(), Box<dyn std::error::Error>> {
    let eof = Token::eof_at(123);
    assert_eq!(eof.kind, TokenKind::Eof);
    assert_eq!(eof.start, 123);
    assert_eq!(eof.end, 123);
    assert!(eof.range().is_empty());
    Ok(())
}

#[test]
fn unknown_at_supports_synthetic_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let synthetic = Token::unknown_at("<synthetic>", 20, 31);
    assert_eq!(synthetic.kind, TokenKind::Unknown);
    assert_eq!(synthetic.range(), 20..31);
    assert_eq!(synthetic.span(), TokenSpan { start: 20, end: 31 });
    Ok(())
}

#[test]
fn with_span_and_with_kind_are_non_destructive_helpers() -> Result<(), Box<dyn std::error::Error>> {
    let token = Token::new(TokenKind::Identifier, "foo", 2, 5);
    let moved = token.clone().with_span(TokenSpan::new_checked(30, 33)?);
    let retagged = token.clone().with_kind(TokenKind::Unknown);

    assert_eq!(moved.text, token.text);
    assert_eq!(moved.range(), 30..33);
    assert_eq!(retagged.kind, TokenKind::Unknown);
    assert_eq!(retagged.range(), token.range());
    Ok(())
}

#[test]
fn token_span_range_and_len_use_byte_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let span = TokenSpan::new_checked(4, 9)?;
    assert_eq!(span.range(), 4..9);
    assert_eq!(span.len(), 5);
    assert!(!span.is_empty());
    Ok(())
}
