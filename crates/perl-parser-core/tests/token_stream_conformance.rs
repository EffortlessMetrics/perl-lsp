use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};
use std::error::Error;

fn stream_kinds(source: &str) -> Result<Vec<TokenKind>, Box<dyn Error>> {
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

fn identifier_bridge_kind(source: &str) -> Option<TokenKind> {
    TokenStream::lexer_tokens_to_parser_tokens(vec![LexerToken::new(
        LexerTokenType::Identifier(source.into()),
        source,
        0,
        source.len(),
    )])
    .first()
    .map(|token| token.kind)
}

#[test]
fn keyword_and_operator_mappings_are_table_driven() -> Result<(), Box<dyn Error>> {
    for (kind, keyword) in TokenKind::keyword_spellings() {
        let kinds = stream_kinds(keyword)?;
        assert_eq!(kinds.first().copied(), Some(*kind), "keyword={keyword:?} kinds={kinds:?}");
    }

    for (kind, operator) in TokenKind::operator_spellings() {
        let source = match *operator {
            "!" => "!$a;".to_string(),
            "~" => "~$a;".to_string(),
            "++" => "++$a;".to_string(),
            "--" => "--$a;".to_string(),
            "?" | ":" => "$a ? $b : $c;".to_string(),
            "->" => "$obj->method;".to_string(),
            "=>" => "{ a => 1 };".to_string(),
            "::" => "::".to_string(),
            "\\" => "\\$a;".to_string(),
            "/" => "4 / 2;".to_string(),
            "//" => "$a // $b;".to_string(),
            "//=" => "$a //= $b;".to_string(),
            "cmp" => "$a cmp $b;".to_string(),
            "and" => "$a and $b;".to_string(),
            "or" => "$a or $b;".to_string(),
            "not" => "not $a;".to_string(),
            "xor" => "$a xor $b;".to_string(),
            _ => format!("$a {operator} $b;"),
        };
        let kinds = stream_kinds(&source)?;
        assert!(kinds.contains(kind), "operator={operator:?} expected={kind:?} kinds={kinds:?}");
    }
    Ok(())
}

#[test]
fn delimiter_sigil_quote_regex_and_data_mappings_conform() -> Result<(), Box<dyn Error>> {
    for (kind, delimiter) in TokenKind::delimiter_spellings() {
        let kinds = stream_kinds(delimiter)?;
        assert_eq!(kinds.first().copied(), Some(*kind));
    }

    for (kind, sigil) in TokenKind::sigil_spellings() {
        let expected = if *kind == TokenKind::GlobSigil { TokenKind::Star } else { *kind };
        assert_eq!(identifier_bridge_kind(sigil), Some(expected));
    }

    for (source, expected) in [
        ("q/a/", TokenKind::QuoteSingle),
        ("qq/a/", TokenKind::QuoteDouble),
        ("qw(a b)", TokenKind::QuoteWords),
        ("qx/echo/", TokenKind::QuoteCommand),
        ("m/a/", TokenKind::Regex),
        ("qr/a/", TokenKind::Regex),
        ("s/a/b/", TokenKind::Substitution),
        ("tr/a/b/", TokenKind::Transliteration),
    ] {
        assert_eq!(stream_kinds(source)?.first().copied(), Some(expected));
    }

    let mapped = TokenStream::lexer_tokens_to_parser_tokens(vec![
        LexerToken::new(LexerTokenType::HeredocStart, "<<EOF", 0, 5),
        LexerToken::new(LexerTokenType::HeredocBody("body".into()), "body", 5, 9),
        LexerToken::new(LexerTokenType::DataMarker("__DATA__".into()), "__DATA__", 9, 17),
        LexerToken::new(LexerTokenType::DataBody("payload".into()), "payload", 17, 24),
    ]);
    let mapped_kinds: Vec<TokenKind> = mapped.into_iter().map(|token| token.kind).collect();
    assert!(mapped_kinds.contains(&TokenKind::HeredocStart));
    assert!(mapped_kinds.contains(&TokenKind::HeredocBody));
    assert!(mapped_kinds.contains(&TokenKind::DataMarker));
    assert!(mapped_kinds.contains(&TokenKind::DataBody));

    assert_eq!(stream_kinds("")?.last().copied(), Some(TokenKind::Eof));
    Ok(())
}

#[test]
fn unknown_and_unknown_rest_are_mapped() {
    let mapped = TokenStream::lexer_tokens_to_parser_tokens(vec![
        LexerToken::new(LexerTokenType::Error("unknown".into()), "§", 0, 1),
        LexerToken::new(LexerTokenType::UnknownRest, "...", 1, 4),
        LexerToken::new(LexerTokenType::EOF, "", 4, 4),
    ]);

    assert_eq!(mapped[0].kind, TokenKind::Unknown);
    assert_eq!(mapped[1].kind, TokenKind::UnknownRest);
}
