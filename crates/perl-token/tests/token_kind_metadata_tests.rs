use perl_token::TokenKind;

#[test]
fn keyword_operator_delimiter_and_sigil_metadata_is_unique() {
    let mut seen = Vec::new();

    for (kind, spelling) in TokenKind::keyword_spellings()
        .iter()
        .chain(TokenKind::operator_spellings())
        .chain(TokenKind::delimiter_spellings())
        .chain(TokenKind::sigil_spellings())
    {
        assert!(!seen.contains(kind), "duplicate metadata entry for {kind:?}");
        seen.push(*kind);
        assert!(!spelling.is_empty(), "empty canonical spelling for {kind:?}");
    }
}

#[test]
fn canonical_spellings_align_with_display_names() {
    for (kind, spelling) in TokenKind::keyword_spellings()
        .iter()
        .chain(TokenKind::operator_spellings())
        .chain(TokenKind::delimiter_spellings())
        .chain(TokenKind::sigil_spellings())
    {
        let expected = format!("'{spelling}'");
        assert_eq!(kind.display_name(), expected, "display mismatch for {kind:?}");
    }
}

#[test]
fn all_kinds_contains_no_duplicates() {
    let mut seen = Vec::new();
    for kind in TokenKind::all() {
        assert!(!seen.contains(kind), "TokenKind::all duplicate {kind:?}");
        seen.push(*kind);
    }
}
