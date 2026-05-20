use std::collections::BTreeSet;

use perl_token::{
    DELIMITER_SPELLINGS, KEYWORD_SPELLINGS, OPERATOR_SPELLINGS, SIGIL_SPELLINGS, TokenCategory,
    TokenKind,
};

fn assert_unique_table(name: &str, table: &[(&str, TokenKind)]) {
    let mut spellings = BTreeSet::new();
    let mut kinds = BTreeSet::new();

    for &(spelling, kind) in table {
        assert!(spellings.insert(spelling), "{name} repeats spelling {spelling:?}");
        assert!(kinds.insert(format!("{kind:?}")), "{name} repeats kind {kind:?}");
    }
}

fn table_contains_kind(table: &[(&str, TokenKind)], kind: TokenKind) -> bool {
    table.iter().any(|(_, candidate)| *candidate == kind)
}

#[test]
fn canonical_spelling_tables_are_nonempty_and_unique() {
    for (name, table) in [
        ("keywords", KEYWORD_SPELLINGS),
        ("operators", OPERATOR_SPELLINGS),
        ("delimiters", DELIMITER_SPELLINGS),
        ("sigils", SIGIL_SPELLINGS),
    ] {
        assert!(!table.is_empty(), "{name} spelling table must not be empty");
        assert_unique_table(name, table);
    }
}

#[test]
fn canonical_spelling_tables_cover_parser_facing_categories() {
    for kind in TokenKind::all() {
        match kind.category() {
            TokenCategory::Keyword => assert!(
                table_contains_kind(KEYWORD_SPELLINGS, *kind),
                "keyword {kind:?} needs canonical spelling metadata"
            ),
            TokenCategory::Operator => assert!(
                table_contains_kind(OPERATOR_SPELLINGS, *kind)
                    || table_contains_kind(KEYWORD_SPELLINGS, *kind),
                "operator {kind:?} needs canonical spelling metadata"
            ),
            TokenCategory::Delimiter => assert!(
                table_contains_kind(DELIMITER_SPELLINGS, *kind),
                "delimiter {kind:?} needs canonical spelling metadata"
            ),
            TokenCategory::Identifier
                if matches!(
                    kind,
                    TokenKind::ScalarSigil
                        | TokenKind::ArraySigil
                        | TokenKind::HashSigil
                        | TokenKind::SubSigil
                        | TokenKind::GlobSigil
                ) =>
            {
                assert!(
                    table_contains_kind(SIGIL_SPELLINGS, *kind),
                    "sigil {kind:?} needs canonical spelling metadata"
                );
            }
            // Required by #[non_exhaustive]: catch any future category variants.
            _ => {}
        }
    }
}

#[test]
fn canonical_spelling_tables_round_trip_through_mapping_helpers() {
    for &(spelling, kind) in KEYWORD_SPELLINGS {
        assert_eq!(TokenKind::from_keyword(spelling), Some(kind), "keyword {spelling}");
    }
    for &(spelling, kind) in OPERATOR_SPELLINGS {
        assert_eq!(TokenKind::from_operator(spelling), Some(kind), "operator {spelling}");
    }
    for &(spelling, kind) in DELIMITER_SPELLINGS {
        assert_eq!(TokenKind::from_delimiter(spelling), Some(kind), "delimiter {spelling}");
    }
    for &(spelling, kind) in SIGIL_SPELLINGS {
        assert_eq!(TokenKind::from_sigil(spelling), Some(kind), "sigil {spelling}");
    }
}
