//! Tests for TokenKind metadata and categorization invariants.

use perl_token::{ALL_TOKEN_KINDS, Token, TokenCategory, TokenKind};
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn display_name_is_non_empty_for_every_token_kind() {
    for kind in ALL_TOKEN_KINDS {
        assert!(!kind.display_name().is_empty(), "missing display_name for {kind:?}");
    }
}

#[test]
fn every_kind_has_exactly_one_category() {
    for kind in ALL_TOKEN_KINDS {
        let flags = [
            kind.is_keyword(),
            kind.is_operator(),
            kind.is_delimiter(),
            kind.is_literal(),
            kind.is_sigil(),
            kind.is_special(),
            kind.category() == TokenCategory::Identifier,
        ];
        let true_count = flags.into_iter().filter(|flag| *flag).count();
        assert_eq!(true_count, 1, "expected exactly one category for {kind:?}, got {true_count}");
    }
}

#[test]
fn identifier_like_covers_identifier_and_sigils() {
    for kind in ALL_TOKEN_KINDS {
        assert_eq!(
            kind.is_identifier_like(),
            matches!(kind.category(), TokenCategory::Identifier | TokenCategory::Sigil),
            "identifier-like mismatch for {kind:?}"
        );
    }
}

#[test]
fn token_kind_info_round_trips_kind() {
    for kind in ALL_TOKEN_KINDS {
        assert_eq!(kind.info().kind, *kind, "info().kind mismatch for {kind:?}");
    }
}

#[test]
fn all_token_kinds_has_no_duplicates() {
    let mut seen = HashSet::new();
    for kind in ALL_TOKEN_KINDS {
        assert!(seen.insert(*kind), "duplicate variant listed in ALL_TOKEN_KINDS: {kind:?}");
    }
}

#[test]
fn source_location_tracking_consistency() {
    let t = Token::new(TokenKind::Identifier, Arc::<str>::from("var_name"), 10, 18);
    assert_eq!(t.start, 10);
    assert_eq!(t.end, 18);
    assert_eq!(t.len(), 8);
    assert!(!t.is_empty());
}
