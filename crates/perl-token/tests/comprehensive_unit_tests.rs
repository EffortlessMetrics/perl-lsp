//! Comprehensive unit tests for the `perl-token` crate.

use perl_token::{Token, TokenKind};
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
fn token_new_accepts_owned_and_arc_text() {
    let owned = Token::new(TokenKind::String, String::from("hello"), 5, 10);
    assert_eq!(&*owned.text, "hello");

    let shared: Arc<str> = Arc::from("world");
    let arc_tok = Token::new(TokenKind::Identifier, shared.clone(), 0, 5);
    assert_eq!(&*arc_tok.text, "world");
    assert_eq!(Arc::strong_count(&shared), 2);
}

#[test]
fn token_len_and_empty_behaviors() {
    let normal = Token::new(TokenKind::Identifier, "foo", 1, 4);
    assert_eq!(normal.len(), 3);
    assert!(!normal.is_empty());

    let eof = Token::new(TokenKind::Eof, "", 8, 8);
    assert_eq!(eof.len(), 0);
    assert!(eof.is_empty());

    let malformed = Token::new(TokenKind::Unknown, "?", 12, 7);
    assert_eq!(malformed.len(), 0);
    assert!(malformed.is_empty());
}

#[test]
fn token_clone_shares_backing_arc() {
    let token = Token::new(TokenKind::Identifier, "foo_bar", 0, 7);
    let cloned = token.clone();
    assert_eq!(token, cloned);
    assert!(Arc::ptr_eq(&token.text, &cloned.text));
}

#[test]
fn token_debug_includes_kind_and_text() {
    let t = Token::new(TokenKind::Return, "return", 0, 6);
    let dbg = format!("{t:?}");
    assert!(dbg.contains("Return"));
    assert!(dbg.contains("return"));
}

#[test]
fn token_kind_inventory_and_metadata_stay_in_sync() {
    assert_eq!(TokenKind::all().len(), TokenKind::VARIANT_COUNT);

    for &kind in TokenKind::all() {
        let info = kind.info();
        assert_eq!(info.kind, kind);
        assert_eq!(kind.display_name(), info.display_name);
    }
}
