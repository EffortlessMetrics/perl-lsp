use perl_lexer::PerlLexer;
use perl_parser_core::token_stream::{TokenKind, TokenStream};

fn first_parser_kind(input: &str) -> Option<TokenKind> {
    let tokens = PerlLexer::new(input).collect_tokens();
    TokenStream::lexer_tokens_to_parser_tokens(tokens).into_iter().next().map(|token| token.kind)
}

#[test]
fn token_kind_keywords_are_table_driven() {
    for (kind, spelling) in TokenKind::keyword_spellings() {
        assert_eq!(first_parser_kind(spelling), Some(*kind));
    }
}

#[test]
fn token_kind_operators_are_table_driven() {
    let mut seen = Vec::new();
    for (kind, spelling) in TokenKind::operator_spellings() {
        assert!(!spelling.is_empty());
        assert!(!seen.contains(kind), "duplicate operator entry for {kind:?}");
        seen.push(*kind);
    }
}

#[test]
fn token_kind_delimiters_and_sigils_are_table_driven() {
    for (kind, spelling) in TokenKind::delimiter_spellings() {
        assert_eq!(first_parser_kind(spelling), Some(*kind));
    }
    assert_eq!(TokenKind::sigil_spellings().len(), 5);
}

#[test]
fn token_kind_quote_like_regex_and_substitution_tokens_conform() {
    let cases = [
        ("q/foo/", TokenKind::QuoteSingle),
        ("qq/foo/", TokenKind::QuoteDouble),
        ("qw(foo bar)", TokenKind::QuoteWords),
        ("qx/date/", TokenKind::QuoteCommand),
        ("m/foo/", TokenKind::Regex),
        ("s/foo/bar/", TokenKind::Substitution),
        ("tr/a/b/", TokenKind::Transliteration),
    ];

    for (input, expected) in cases {
        assert_eq!(first_parser_kind(input), Some(expected));
    }
}
