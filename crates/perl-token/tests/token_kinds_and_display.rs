//! Tests for `TokenKind` metadata table coverage and classification helpers.

use perl_token::{Token, TokenCategory, TokenKind};

#[test]
fn metadata_table_covers_every_variant() {
    let all = TokenKind::all();
    assert_eq!(all.len(), TokenKind::VARIANT_COUNT);

    for &kind in all {
        let info = kind.info();
        assert_eq!(info.kind, kind, "metadata row mismatch for {kind:?}");
        assert!(!info.display_name.is_empty(), "empty display_name for {kind:?}");
    }
}

#[test]
fn every_variant_has_exactly_one_category() {
    for &kind in TokenKind::all() {
        let categories = [
            kind.is_keyword(),
            kind.is_operator(),
            kind.is_delimiter(),
            kind.is_literal(),
            kind.is_identifier_like() && !kind.is_sigil(),
            kind.is_sigil(),
            kind.is_special(),
        ];

        let category_count = categories.iter().filter(|is_set| **is_set).count();
        assert_eq!(category_count, 1, "{kind:?} belongs to {category_count} categories");
    }
}

#[test]
fn display_name_delegates_to_info_table() {
    for &kind in TokenKind::all() {
        assert_eq!(kind.display_name(), kind.info().display_name);
    }
}

#[test]
fn category_helpers_match_category_field() {
    for &kind in TokenKind::all() {
        match kind.category() {
            TokenCategory::Keyword => assert!(kind.is_keyword()),
            TokenCategory::Operator => assert!(kind.is_operator()),
            TokenCategory::Delimiter => assert!(kind.is_delimiter()),
            TokenCategory::Literal => assert!(kind.is_literal()),
            TokenCategory::Identifier => assert!(kind.is_identifier_like() && !kind.is_sigil()),
            TokenCategory::Sigil => {
                assert!(kind.is_sigil());
                assert!(kind.is_identifier_like());
            }
            TokenCategory::Special => assert!(kind.is_special()),
        }
    }
}

#[test]
fn canonical_lexeme_for_common_tokens() {
    assert_eq!(TokenKind::Sub.canonical_lexeme(), Some("sub"));
    assert_eq!(TokenKind::Arrow.canonical_lexeme(), Some("->"));
    assert_eq!(TokenKind::Semicolon.canonical_lexeme(), Some(";"));
    assert_eq!(TokenKind::Identifier.canonical_lexeme(), None);
    assert_eq!(TokenKind::Number.canonical_lexeme(), None);
}

#[test]
fn token_span_tracking_still_works() {
    let tok = Token::new(TokenKind::Identifier, "hello", 10, 15);
    assert_eq!(tok.len(), 5);
    assert!(!tok.is_empty());
}
