use perl_token::TokenKind;
use std::collections::HashSet;

#[test]
fn all_token_kinds_are_unique() {
    let mut seen = HashSet::new();
    for kind in TokenKind::ALL {
        assert!(seen.insert(*kind as u16), "duplicate TokenKind in TokenKind::ALL: {kind:?}");
    }
}

#[test]
fn all_token_kinds_have_display_names() {
    for kind in TokenKind::ALL {
        assert!(!kind.display_name().is_empty(), "missing display_name coverage for {kind:?}");
    }
}

#[test]
fn expected_conformance_targets_are_present_in_metadata() {
    let must_exist = [
        TokenKind::HeredocStart,
        TokenKind::HeredocBody,
        TokenKind::DataMarker,
        TokenKind::DataBody,
        TokenKind::Unknown,
        TokenKind::UnknownRest,
        TokenKind::Eof,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
    ];

    for target in must_exist {
        assert!(TokenKind::ALL.contains(&target), "TokenKind::ALL missing {target:?}");
    }
}
