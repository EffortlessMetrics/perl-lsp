use perl_token::{
    DELIMITER_SPELLINGS, KEYWORD_SPELLINGS, OPERATOR_SPELLINGS, SIGIL_SPELLINGS, TokenKind,
};

#[test]
fn keyword_mapping_covers_canonical_spellings() {
    for &(spelling, expected) in KEYWORD_SPELLINGS {
        assert_eq!(TokenKind::from_keyword(spelling), Some(expected), "keyword: {spelling}");
    }
}

#[test]
fn operator_mapping_covers_canonical_spellings() {
    for &(spelling, expected) in OPERATOR_SPELLINGS {
        assert_eq!(TokenKind::from_operator(spelling), Some(expected), "operator: {spelling}");
    }
}

#[test]
fn delimiter_mapping_covers_canonical_spellings() {
    for &(spelling, expected) in DELIMITER_SPELLINGS {
        assert_eq!(TokenKind::from_delimiter(spelling), Some(expected), "delimiter: {spelling}");
    }
}

#[test]
fn sigil_mapping_covers_canonical_spellings() {
    for &(spelling, expected) in SIGIL_SPELLINGS {
        assert_eq!(TokenKind::from_sigil(spelling), Some(expected), "sigil: {spelling}");
    }
}

#[test]
fn canonical_spelling_covers_fixed_spelling_tables() {
    for &(spelling, kind) in KEYWORD_SPELLINGS {
        assert_eq!(kind.canonical_spelling(), Some(spelling), "keyword: {kind:?}");
    }

    for &(spelling, kind) in OPERATOR_SPELLINGS {
        assert_eq!(kind.canonical_spelling(), Some(spelling), "operator: {kind:?}");
    }

    for &(spelling, kind) in DELIMITER_SPELLINGS {
        assert_eq!(kind.canonical_spelling(), Some(spelling), "delimiter: {kind:?}");
    }

    for &(spelling, kind) in SIGIL_SPELLINGS {
        assert_eq!(kind.canonical_spelling(), Some(spelling), "sigil: {kind:?}");
    }
}

#[test]
fn canonical_spelling_is_none_for_value_carrying_tokens() {
    let value_carrying = [
        TokenKind::Identifier,
        TokenKind::Number,
        TokenKind::String,
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::HeredocStart,
        TokenKind::HeredocBody,
        TokenKind::FormatBody,
        TokenKind::DataMarker,
        TokenKind::DataBody,
        TokenKind::VString,
        TokenKind::UnknownRest,
        TokenKind::HeredocDepthLimit,
        TokenKind::Eof,
        TokenKind::Unknown,
    ];

    for kind in value_carrying {
        assert_eq!(kind.canonical_spelling(), None, "value-carrying token: {kind:?}");
    }
}

#[test]
fn canonical_spelling_preserves_contextual_sigil_kinds() {
    assert_eq!(TokenKind::Percent.canonical_spelling(), Some("%"));
    assert_eq!(TokenKind::HashSigil.canonical_spelling(), Some("%"));
    assert_eq!(TokenKind::BitwiseAnd.canonical_spelling(), Some("&"));
    assert_eq!(TokenKind::SubSigil.canonical_spelling(), Some("&"));
    assert_eq!(TokenKind::Star.canonical_spelling(), Some("*"));
    assert_eq!(TokenKind::GlobSigil.canonical_spelling(), Some("*"));
}

#[test]
fn mappings_are_case_sensitive_and_contextual() {
    assert_eq!(TokenKind::from_keyword("My"), None);
    assert_eq!(TokenKind::from_keyword("begin"), None);
    assert_eq!(TokenKind::from_keyword("qw"), None);

    assert_eq!(TokenKind::from_operator("AND"), None);
    assert_eq!(TokenKind::from_delimiter("<"), None);
    assert_eq!(TokenKind::from_sigil("+"), None);
}

#[test]
fn category_helpers_align_with_category_method() {
    for kind in TokenKind::all() {
        assert_eq!(kind.is_keyword(), kind.category() == perl_token::TokenCategory::Keyword);
        assert_eq!(kind.is_operator(), kind.category() == perl_token::TokenCategory::Operator);
        assert_eq!(kind.is_literal(), kind.category() == perl_token::TokenCategory::Literal);
        assert_eq!(kind.is_delimiter(), kind.category() == perl_token::TokenCategory::Delimiter);
        assert_eq!(kind.is_identifier(), kind.category() == perl_token::TokenCategory::Identifier);
        assert_eq!(kind.is_special(), kind.category() == perl_token::TokenCategory::Special);
    }
}
