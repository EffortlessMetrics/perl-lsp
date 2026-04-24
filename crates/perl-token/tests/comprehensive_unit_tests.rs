//! Comprehensive unit tests for the `perl-token` crate.

use perl_token::{ALL_TOKEN_KINDS, Token, TokenKind};
use std::sync::Arc;

#[test]
fn token_new_basic_fields() {
    let t = Token::new(TokenKind::My, "my", 0, 2);
    assert_eq!(t.kind, TokenKind::My);
    assert_eq!(&*t.text, "my");
    assert_eq!(t.start, 0);
    assert_eq!(t.end, 2);
}

#[test]
fn token_len_and_empty_helpers() {
    let t = Token::new(TokenKind::Identifier, "hello", 7, 12);
    assert_eq!(t.len(), 5);
    assert!(!t.is_empty());

    let malformed = Token::new(TokenKind::Unknown, "?", 12, 7);
    assert_eq!(malformed.len(), 0);
    assert!(malformed.is_empty());
}

#[test]
fn token_clone_shares_arc() {
    let t = Token::new(TokenKind::Identifier, "foo_bar", 0, 7);
    let c = t.clone();
    assert!(Arc::ptr_eq(&t.text, &c.text));
}

#[test]
fn token_debug_contains_kind_and_text() {
    let t = Token::new(TokenKind::Return, "return", 0, 6);
    let dbg = format!("{t:?}");
    assert!(dbg.contains("Return"), "expected 'Return' in debug: {dbg}");
    assert!(dbg.contains("return"), "expected 'return' in debug: {dbg}");
}

#[test]
fn all_token_kinds_are_metadata_backed() {
    for kind in ALL_TOKEN_KINDS {
        let info = kind.info();
        assert_eq!(info.kind, *kind);
        assert_eq!(kind.display_name(), info.display_name);
    }
}

#[test]
fn tokenkind_traits_copy_clone_eq_debug() {
    let a = TokenKind::While;
    let b = a;
    let c = a;
    assert_eq!(b, c);

    let cloned = a;
    assert_eq!(cloned, TokenKind::While);

    let dbg = format!("{a:?}");
    assert!(dbg.contains("While"));
}
