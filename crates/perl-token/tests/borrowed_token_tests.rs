use perl_token::{Token, TokenKind, TokenRef, TokenSpan};
use std::error::Error;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn token_ref_new_exposes_borrowed_fields() -> TestResult {
    let source = "my $name";
    let token = TokenRef::new(TokenKind::My, &source[0..2], 0, 2);

    assert_eq!(token.kind, TokenKind::My);
    assert_eq!(token.text, "my");
    assert_eq!(token.len(), 2);
    assert!(!token.is_empty());
    assert_eq!(token.span(), (0, 2));
    assert_eq!(token.display_name(), "'my'");
    Ok(())
}

#[test]
fn token_ref_to_owned_token_is_explicit() -> TestResult {
    let borrowed = TokenRef::new(TokenKind::Identifier, "value", 8, 13);
    let owned = borrowed.to_owned_token();

    assert_eq!(owned.kind, TokenKind::Identifier);
    assert_eq!(&*owned.text, "value");
    assert_eq!(owned.span(), TokenSpan::new(8, 13));
    assert_eq!(owned.display_name(), "identifier");
    Ok(())
}

#[test]
fn token_from_token_ref_matches_constructor() -> TestResult {
    let borrowed = TokenRef::new(TokenKind::Number, "42", 20, 22);
    let from_impl: Token = borrowed.into();
    let constructed = Token::new(TokenKind::Number, "42", 20, 22);

    assert_eq!(from_impl, constructed);
    Ok(())
}

#[test]
fn token_as_ref_token_roundtrips_without_changing_span() -> TestResult {
    let token = Token::new(TokenKind::String, "\"abc\"", 4, 9);
    let borrowed = token.as_ref_token();

    assert_eq!(borrowed.kind, TokenKind::String);
    assert_eq!(borrowed.text, "\"abc\"");
    assert_eq!(borrowed.span(), (token.start, token.end));

    let rebuilt = borrowed.to_owned_token();
    assert_eq!(rebuilt, token);
    Ok(())
}

#[test]
fn token_ref_does_not_touch_existing_arc_token_storage() -> TestResult {
    let text: Arc<str> = Arc::from("shared");
    let token = Token::new(TokenKind::Identifier, text.clone(), 0, 6);
    let strong_before = Arc::strong_count(&text);

    let borrowed = token.as_ref_token();
    assert_eq!(borrowed.text, "shared");
    assert_eq!(Arc::strong_count(&text), strong_before);
    Ok(())
}

#[test]
fn token_ref_len_saturates_for_malformed_span() -> TestResult {
    // end < start must not underflow — mirrors Token::len saturating behavior
    let borrowed = TokenRef::new(TokenKind::Unknown, "x", 10, 5);
    assert_eq!(borrowed.len(), 0, "saturating_sub should yield 0 for end < start");
    assert!(borrowed.is_empty(), "is_empty should be true when span is malformed");
    Ok(())
}

#[test]
fn token_ref_is_empty_for_zero_length_span() -> TestResult {
    // EOF tokens have equal start and end; should be empty regardless of text content
    let borrowed = TokenRef::new(TokenKind::Eof, "", 8, 8);
    assert_eq!(borrowed.len(), 0);
    assert!(borrowed.is_empty());
    assert_eq!(borrowed.span(), (8, 8));
    Ok(())
}
