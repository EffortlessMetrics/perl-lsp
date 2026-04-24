use perl_token::TokenKind;
use std::collections::HashSet;

fn assert_unique_spellings(label: &str, rows: &[(TokenKind, &'static str)]) {
    let mut spellings = HashSet::new();
    for (_, spelling) in rows {
        assert!(spellings.insert(*spelling), "duplicate {label} spelling in metadata: {spelling}");
        assert!(!spelling.is_empty(), "{label} spelling must not be empty");
    }
}

#[test]
fn keyword_metadata_is_unique_and_non_empty() {
    assert!(!TokenKind::KEYWORD_SPELLINGS.is_empty());
    assert_unique_spellings("keyword", TokenKind::KEYWORD_SPELLINGS);
}

#[test]
fn operator_metadata_is_unique_and_non_empty() {
    assert!(!TokenKind::OPERATOR_SPELLINGS.is_empty());
    assert_unique_spellings("operator", TokenKind::OPERATOR_SPELLINGS);
}

#[test]
fn delimiter_metadata_is_unique_and_non_empty() {
    assert!(!TokenKind::DELIMITER_SPELLINGS.is_empty());
    assert_unique_spellings("delimiter", TokenKind::DELIMITER_SPELLINGS);
}

#[test]
fn sigil_metadata_is_unique_and_non_empty() {
    assert!(!TokenKind::SIGIL_SPELLINGS.is_empty());
    assert_unique_spellings("sigil", TokenKind::SIGIL_SPELLINGS);
}

#[test]
fn metadata_spellings_align_with_display_names() {
    for (kind, spelling) in TokenKind::KEYWORD_SPELLINGS
        .iter()
        .chain(TokenKind::OPERATOR_SPELLINGS.iter())
        .chain(TokenKind::DELIMITER_SPELLINGS.iter())
        .chain(TokenKind::SIGIL_SPELLINGS.iter())
    {
        assert_eq!(kind.display_name(), format!("'{spelling}'"));
    }
}
