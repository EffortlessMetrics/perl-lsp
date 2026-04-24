use perl_token::{Token, TokenCategory, TokenKind};

const EXPECTED_TOKEN_KIND_COUNT: usize = 132;
const README_COUNT_MARKER: &str = "TokenKind variant count: 132";
const ROADMAP_COUNT_MARKER: &str = "TokenKind variant count: 132";

#[test]
fn token_and_token_kind_public_api_contract_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let token = Token::new(TokenKind::Identifier, "name", 4, 8);
    let Token { kind, text, start, end } = token;

    assert_eq!(kind, TokenKind::Identifier);
    assert_eq!(&*text, "name");
    assert_eq!(start, 4);
    assert_eq!(end, 8);

    assert_eq!(TokenKind::all().len(), EXPECTED_TOKEN_KIND_COUNT);
    assert_eq!(TokenKind::all_metadata().len(), EXPECTED_TOKEN_KIND_COUNT);
    Ok(())
}

#[test]
fn token_kind_metadata_is_complete_and_counted() -> Result<(), Box<dyn std::error::Error>> {
    for kind in TokenKind::all() {
        let metadata = kind.metadata();
        assert_eq!(metadata.kind, *kind, "metadata kind mismatch for {kind:?}");
        assert!(!metadata.display_name.is_empty(), "display_name missing for {kind:?}");
        assert_eq!(
            kind.display_name(),
            metadata.display_name,
            "display_name mismatch for {kind:?}"
        );
        assert_eq!(kind.category(), metadata.category, "category mismatch for {kind:?}");
    }

    let category_count = TokenKind::all()
        .iter()
        .filter(|kind| {
            matches!(
                kind.category(),
                TokenCategory::Keyword
                    | TokenCategory::Operator
                    | TokenCategory::Delimiter
                    | TokenCategory::Literal
                    | TokenCategory::IdentifierOrSigil
                    | TokenCategory::Special
            )
        })
        .count();

    assert_eq!(category_count, EXPECTED_TOKEN_KIND_COUNT);
    assert_eq!(TokenKind::all().len(), TokenKind::all_metadata().len());
    Ok(())
}

#[test]
fn token_kind_docs_and_conformance_markers_stay_in_sync() -> Result<(), Box<dyn std::error::Error>>
{
    let readme = std::fs::read_to_string("README.md")?;
    assert!(
        readme.contains(README_COUNT_MARKER),
        "README must include `{README_COUNT_MARKER}` and be updated when TokenKind changes"
    );

    let roadmap = std::fs::read_to_string("ROADMAP.md")?;
    assert!(
        roadmap.contains(ROADMAP_COUNT_MARKER),
        "ROADMAP must include `{ROADMAP_COUNT_MARKER}` and be updated when TokenKind changes"
    );

    Ok(())
}

#[test]
fn perl_token_has_no_runtime_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::fs::read_to_string("Cargo.toml")?;
    let mut in_dependencies = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }

        if in_dependencies && !trimmed.is_empty() && !trimmed.starts_with('#') {
            return Err(
                format!("runtime dependency is not allowed in perl-token: `{trimmed}`").into()
            );
        }
    }

    Ok(())
}
