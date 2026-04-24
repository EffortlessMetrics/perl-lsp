use perl_token::{Token, TokenKind, TokenRef};
use std::sync::Arc;

#[test]
fn token_ref_construction_and_helpers() -> Result<(), Box<dyn std::error::Error>> {
    let token = TokenRef::new(TokenKind::Identifier, "alpha", 4, 9);

    assert_eq!(token.kind, TokenKind::Identifier);
    assert_eq!(token.text, "alpha");
    assert_eq!(token.len(), 5);
    assert!(!token.is_empty());
    assert_eq!(token.span(), (4, 9));
    assert_eq!(token.display_name(), "identifier");

    Ok(())
}

#[test]
fn token_ref_to_owned_explicit_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let borrowed = TokenRef::new(TokenKind::My, "my", 0, 2);

    let owned = borrowed.to_owned_token();

    assert_eq!(owned.kind, TokenKind::My);
    assert_eq!(&*owned.text, "my");
    assert_eq!(owned.span(), (0, 2));
    assert_eq!(owned.display_name(), "'my'");

    Ok(())
}

#[test]
fn token_ref_from_impl_matches_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let borrowed = TokenRef::new(TokenKind::Number, "42", 11, 13);

    let from_impl: Token = borrowed.into();
    let explicit = borrowed.to_owned_token();

    assert_eq!(from_impl, explicit);

    Ok(())
}

#[test]
fn owned_token_as_ref_token_preserves_fields() -> Result<(), Box<dyn std::error::Error>> {
    let shared_text = Arc::<str>::from("value");
    let owned = Token::new(TokenKind::Identifier, Arc::clone(&shared_text), 30, 35);

    let borrowed = owned.as_ref_token();

    assert_eq!(borrowed.kind, TokenKind::Identifier);
    assert_eq!(borrowed.text, "value");
    assert_eq!(borrowed.span(), (30, 35));
    assert_eq!(borrowed.display_name(), "identifier");

    let round_trip = borrowed.to_owned_token();
    assert_eq!(round_trip, owned);

    Ok(())
}

#[test]
fn borrowed_token_len_saturates_for_malformed_span() -> Result<(), Box<dyn std::error::Error>> {
    let malformed = TokenRef::new(TokenKind::Unknown, "", 10, 3);

    assert_eq!(malformed.len(), 0);
    assert!(malformed.is_empty());

    Ok(())
}
