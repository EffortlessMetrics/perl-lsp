use perl_token::{Token, TokenKind};
use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

const EXPECTED_TOKEN_KIND_COUNT: usize = 132;
const README_TOKEN_COUNT_MARKER: &str = "TokenKind variants: 132";
const ROADMAP_TOKEN_COUNT_MARKER: &str = "TokenKind variants: 132";

#[test]
fn tokenkind_metadata_is_complete_and_in_sync_with_all() -> Result<(), Box<dyn Error>> {
    let all = TokenKind::all();
    assert_eq!(all.len(), EXPECTED_TOKEN_KIND_COUNT);

    let mut seen = BTreeSet::new();
    for &kind in all {
        let metadata = kind.metadata();
        assert_eq!(metadata.kind, kind);
        assert_eq!(metadata.display_name, kind.display_name());
        assert!(!metadata.display_name.is_empty());
        assert!(seen.insert(format!("{kind:?}")), "duplicate TokenKind entry in all(): {kind:?}");
    }

    assert_eq!(seen.len(), all.len());
    Ok(())
}

#[test]
fn public_token_api_shape_is_stable() -> Result<(), Box<dyn Error>> {
    let from_struct_literal =
        Token { kind: TokenKind::Identifier, text: Arc::from("name"), start: 10, end: 14 };
    assert_eq!(from_struct_literal.len(), 4);

    let from_constructor = Token::new(TokenKind::Identifier, "name", 10, 14);
    assert_eq!(from_constructor.kind, from_struct_literal.kind);
    assert_eq!(from_constructor.text, from_struct_literal.text);
    assert_eq!(from_constructor.start, from_struct_literal.start);
    assert_eq!(from_constructor.end, from_struct_literal.end);

    Ok(())
}

#[test]
fn docs_include_tokenkind_stability_markers() -> Result<(), Box<dyn Error>> {
    let readme = include_str!("../README.md");
    let roadmap = include_str!("../ROADMAP.md");

    assert!(
        readme.contains(README_TOKEN_COUNT_MARKER),
        "README must include updated TokenKind count marker: {README_TOKEN_COUNT_MARKER}",
    );
    assert!(
        roadmap.contains(ROADMAP_TOKEN_COUNT_MARKER),
        "ROADMAP must include updated TokenKind count marker: {ROADMAP_TOKEN_COUNT_MARKER}",
    );

    Ok(())
}
