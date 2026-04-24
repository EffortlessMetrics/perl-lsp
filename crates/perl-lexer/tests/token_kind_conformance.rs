use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};
use std::error::Error;

fn lex_to_parser_kinds(source: &str) -> Result<Vec<TokenKind>, Box<dyn Error>> {
    let mut stream = TokenStream::new(source);
    let mut kinds = Vec::new();

    loop {
        let token = stream.next()?;
        kinds.push(token.kind);
        if token.kind == TokenKind::Eof {
            break;
        }
    }

    Ok(kinds)
}

fn assert_first_kind(source: &str, expected: TokenKind) -> Result<(), Box<dyn Error>> {
    let kinds = lex_to_parser_kinds(source)?;
    assert_eq!(kinds.first().copied(), Some(expected), "source={source:?} kinds={kinds:?}");
    Ok(())
}

fn assert_identifier_bridge_kind(source: &str, expected: TokenKind) {
    let mapped = TokenStream::lexer_tokens_to_parser_tokens(vec![LexerToken::new(
        LexerTokenType::Identifier(source.into()),
        source,
        0,
        source.len(),
    )]);
    assert_eq!(mapped.first().map(|token| token.kind), Some(expected));
}

fn operator_sample(operator: &str) -> &'static str {
    match operator {
        "!" => "!$a;",
        "~" => "~$a;",
        "++" => "++$a;",
        "--" => "--$a;",
        "?" | ":" => "$a ? $b : $c;",
        "->" => "$obj->method;",
        "=>" => "{ a => 1 };",
        "::" => "::",
        "\\" => "\\$a;",
        "/" => "4 / 2;",
        "//" => "$a // $b;",
        "//=" => "$a //= $b;",
        "cmp" => "$a cmp $b;",
        "and" => "$a and $b;",
        "or" => "$a or $b;",
        "xor" => "$a xor $b;",
        "not" => "not $a;",
        _ => "$a OP $b;",
    }
}

#[test]
fn keywords_map_to_parser_token_kinds() -> Result<(), Box<dyn Error>> {
    for (kind, keyword) in TokenKind::keyword_spellings() {
        assert_first_kind(keyword, *kind)?;
    }
    Ok(())
}

#[test]
fn operators_map_to_parser_token_kinds() -> Result<(), Box<dyn Error>> {
    for (kind, operator) in TokenKind::operator_spellings() {
        let source = operator_sample(operator).replace("OP", operator);
        let kinds = lex_to_parser_kinds(&source)?;
        assert!(kinds.contains(kind), "operator {operator:?} did not map to {kind:?}: {kinds:?}");
    }
    Ok(())
}

#[test]
fn delimiters_and_sigils_map_to_parser_token_kinds() -> Result<(), Box<dyn Error>> {
    for (kind, delimiter) in TokenKind::delimiter_spellings() {
        assert_first_kind(delimiter, *kind)?;
    }

    for (kind, sigil) in TokenKind::sigil_spellings() {
        let expected = if *kind == TokenKind::GlobSigil { TokenKind::Star } else { *kind };
        assert_identifier_bridge_kind(sigil, expected);
    }
    Ok(())
}

#[test]
fn quote_like_regex_heredoc_data_and_unknown_tokens_map() -> Result<(), Box<dyn Error>> {
    let quote_like = [
        ("q/alpha/", TokenKind::QuoteSingle),
        ("qq/alpha/", TokenKind::QuoteDouble),
        ("qw(alpha beta)", TokenKind::QuoteWords),
        ("qx/echo hi/", TokenKind::QuoteCommand),
    ];
    for (source, expected) in quote_like {
        assert_first_kind(source, expected)?;
    }

    let regex_family = [
        ("m/a/", TokenKind::Regex),
        ("qr/a/", TokenKind::Regex),
        ("s/a/b/", TokenKind::Substitution),
        ("tr/a/b/", TokenKind::Transliteration),
        ("y/a/b/", TokenKind::Transliteration),
    ];
    for (source, expected) in regex_family {
        assert_first_kind(source, expected)?;
    }

    let heredoc_kinds = lex_to_parser_kinds("my $x = <<EOF;\nline\nEOF\n")?;
    assert!(heredoc_kinds.contains(&TokenKind::HeredocStart));
    assert!(heredoc_kinds.contains(&TokenKind::HeredocBody));

    let data_kinds = lex_to_parser_kinds("__DATA__\ntrailing\n")?;
    assert!(data_kinds.contains(&TokenKind::DataMarker));
    assert!(data_kinds.contains(&TokenKind::DataBody));

    assert_eq!(lex_to_parser_kinds("")?.last().copied(), Some(TokenKind::Eof));

    let unknown_kinds = lex_to_parser_kinds("§")?;
    assert!(unknown_kinds.contains(&TokenKind::Unknown));
    Ok(())
}

#[test]
fn conformance_cases_cover_metadata_categories() -> Result<(), Box<dyn Error>> {
    let mut covered = Vec::new();

    for (kind, keyword) in TokenKind::keyword_spellings() {
        if !covered.contains(kind) {
            covered.push(*kind);
        }
        assert_first_kind(keyword, *kind)?;
    }

    for (kind, operator) in TokenKind::operator_spellings() {
        let source = operator_sample(operator).replace("OP", operator);
        let kinds = lex_to_parser_kinds(&source)?;
        assert!(kinds.contains(kind), "missing operator coverage for {operator}");
        if !covered.contains(kind) {
            covered.push(*kind);
        }
    }

    for (kind, delimiter) in TokenKind::delimiter_spellings() {
        if !covered.contains(kind) {
            covered.push(*kind);
        }
        assert_first_kind(delimiter, *kind)?;
    }

    for (kind, sigil) in TokenKind::sigil_spellings() {
        if !covered.contains(kind) {
            covered.push(*kind);
        }
        let expected = if *kind == TokenKind::GlobSigil { TokenKind::Star } else { *kind };
        assert_identifier_bridge_kind(sigil, expected);
    }

    let required: Vec<TokenKind> = TokenKind::keyword_spellings()
        .iter()
        .chain(TokenKind::operator_spellings())
        .chain(TokenKind::delimiter_spellings())
        .chain(TokenKind::sigil_spellings())
        .map(|(kind, _)| *kind)
        .collect();

    let missing: Vec<TokenKind> =
        required.iter().copied().filter(|kind| !covered.contains(kind)).collect();
    assert!(missing.is_empty(), "missing metadata conformance coverage: {missing:?}");
    Ok(())
}
