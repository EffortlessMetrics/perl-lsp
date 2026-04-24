use perl_lexer::{PerlLexer, Token as RawLexerToken, TokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};

fn first_kind(input: &str) -> Result<TokenKind, Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new(input);
    Ok(stream.next()?.kind)
}

fn parser_kinds_from_lexer(mut lexer: PerlLexer<'_>) -> Vec<TokenKind> {
    let raw_tokens = lexer.collect_tokens();
    TokenStream::lexer_tokens_to_parser_tokens(raw_tokens)
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

fn kinds_from_input(input: &str) -> Vec<TokenKind> {
    parser_kinds_from_lexer(PerlLexer::new(input))
}

fn operator_probes(kind: TokenKind, spelling: &str) -> Vec<String> {
    match kind {
        TokenKind::Question | TokenKind::Colon => vec!["1 ? 2 : 3".to_string()],
        TokenKind::Arrow => vec!["$obj->method".to_string()],
        TokenKind::Backslash => vec!["\\$x".to_string()],
        TokenKind::Increment | TokenKind::Decrement => vec!["$x++".to_string(), "$x--".to_string()],
        TokenKind::StringCompare => vec!["'a' cmp 'b'".to_string()],
        TokenKind::WordAnd | TokenKind::WordOr | TokenKind::WordNot | TokenKind::WordXor => {
            vec![format!("1 {spelling} 1")]
        }
        _ => vec![format!("$x {spelling} 1"), spelling.to_string()],
    }
}

#[test]
fn keyword_spellings_map_to_expected_token_kind() -> Result<(), Box<dyn std::error::Error>> {
    for (kind, spelling) in TokenKind::keyword_spellings() {
        assert_eq!(first_kind(spelling)?, *kind, "keyword {spelling:?} did not map to {kind:?}");
    }

    Ok(())
}

#[test]
fn operator_spellings_map_to_expected_token_kind() -> Result<(), Box<dyn std::error::Error>> {
    for (kind, spelling) in TokenKind::operator_spellings() {
        let mapped = operator_probes(*kind, spelling)
            .iter()
            .any(|input| kinds_from_input(input).contains(kind));
        assert!(mapped, "operator {spelling:?} did not map to {kind:?}");
    }

    Ok(())
}

#[test]
fn delimiters_and_sigils_map_to_expected_token_kind() -> Result<(), Box<dyn std::error::Error>> {
    for (kind, spelling) in TokenKind::delimiter_spellings() {
        assert_eq!(first_kind(spelling)?, *kind, "symbol {spelling:?} did not map to {kind:?}");
    }
    assert_eq!(TokenKind::sigil_spellings().len(), 5);

    Ok(())
}

#[test]
fn quote_like_and_regex_family_map_to_expected_token_kind() -> Result<(), Box<dyn std::error::Error>>
{
    let cases = [
        ("q/abc/", TokenKind::QuoteSingle),
        ("qq/abc/", TokenKind::QuoteDouble),
        ("qw(a b c)", TokenKind::QuoteWords),
        ("qx/echo hi/", TokenKind::QuoteCommand),
        ("m/abc/", TokenKind::Regex),
        ("s/a/b/", TokenKind::Substitution),
        ("tr/a/b/", TokenKind::Transliteration),
        ("y/a/b/", TokenKind::Transliteration),
    ];

    for (input, expected) in cases {
        assert_eq!(first_kind(input)?, expected, "{input:?} did not map to {expected:?}");
    }

    Ok(())
}

#[test]
fn heredoc_data_marker_and_unknown_rest_have_conformance_coverage() {
    let heredoc_input = "my $x = <<EOF;\nbody\nEOF\n";
    let heredoc_kinds = parser_kinds_from_lexer(PerlLexer::with_body_tokens(heredoc_input));
    assert!(heredoc_kinds.contains(&TokenKind::HeredocStart));
    assert!(heredoc_kinds.contains(&TokenKind::HeredocBody));

    let data_input = "__DATA__\nmy raw bytes\n";
    let data_kinds = parser_kinds_from_lexer(PerlLexer::new(data_input));
    assert!(data_kinds.contains(&TokenKind::DataMarker));
    assert!(data_kinds.contains(&TokenKind::DataBody));

    let unknown_rest_tokens = vec![RawLexerToken::new(TokenType::UnknownRest, "??", 0, 2)];
    let unknown_rest_kinds = TokenStream::lexer_tokens_to_parser_tokens(unknown_rest_tokens)
        .into_iter()
        .map(|token| token.kind)
        .collect::<Vec<_>>();
    assert!(unknown_rest_kinds.contains(&TokenKind::UnknownRest));
}

#[test]
fn eof_and_unknown_map_to_expected_token_kind() {
    let eof_kinds = parser_kinds_from_lexer(PerlLexer::new(""));
    assert!(eof_kinds.is_empty(), "EOF is filtered from parser token buffers");

    let unknown_kinds = parser_kinds_from_lexer(PerlLexer::new("\u{00A7}"));
    assert!(unknown_kinds.contains(&TokenKind::Unknown));

    let mut stream = TokenStream::new("");
    assert_eq!(stream.next().map(|token| token.kind), Ok(TokenKind::Eof));
}

#[test]
fn lexer_to_parser_conversion_is_covered_for_all_lexed_kinds() {
    let mut covered = Vec::new();

    covered.extend(TokenKind::keyword_spellings().iter().map(|(kind, _)| *kind));
    covered.extend(TokenKind::operator_spellings().iter().map(|(kind, _)| *kind));
    covered.extend(TokenKind::delimiter_spellings().iter().map(|(kind, _)| *kind));
    covered.extend(TokenKind::sigil_spellings().iter().map(|(kind, _)| *kind));
    covered.extend([
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
    ]);

    for kind in TokenKind::all_kinds() {
        assert!(covered.contains(kind), "missing conformance coverage for {kind:?}");
    }
}
