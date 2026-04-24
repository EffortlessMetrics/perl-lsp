use perl_lexer::{Token as LexerToken, TokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};
use perl_tdd_support::must;

fn first_kind(source: &str) -> TokenKind {
    let mut stream = TokenStream::new(source);
    must(stream.peek()).kind
}

#[test]
fn token_stream_canonical_keywords_map_to_parser_token_kinds() {
    for (kind, spelling) in TokenKind::KEYWORD_SPELLINGS {
        let observed = first_kind(spelling);
        assert_eq!(
            observed, *kind,
            "keyword mapping mismatch for `{spelling}`: expected {:?}, got {:?}",
            kind, observed
        );
    }
}

#[test]
fn token_stream_canonical_operators_map_to_parser_token_kinds() {
    for (kind, spelling) in TokenKind::OPERATOR_SPELLINGS {
        let snippet = match *spelling {
            "!" | "not" => format!("{spelling} 1"),
            "++" | "--" => format!("$x {spelling}"),
            "->" => "$obj->method".to_string(),
            "?" => "1 ? 2 : 3".to_string(),
            ":" => "1 ? 2 : 3".to_string(),
            "..." => "sub f { ... }".to_string(),
            _ => format!("1 {spelling} 2"),
        };

        let mut stream = TokenStream::new(&snippet);
        let mut found = None;
        loop {
            let token = must(stream.next());
            if token.kind == TokenKind::Eof {
                break;
            }
            if token.kind == *kind {
                found = Some(token.kind);
                break;
            }
        }

        assert!(
            found.is_some(),
            "operator mapping mismatch for `{spelling}`: never observed {:?} in `{snippet}`",
            kind
        );
    }
}

#[test]
fn token_stream_conformance_for_delimiters_sigils_and_special_tokens() {
    for (kind, spelling) in TokenKind::DELIMITER_SPELLINGS {
        let observed = first_kind(spelling);
        assert_eq!(observed, *kind, "delimiter mapping mismatch for `{spelling}`");
    }

    let sigil_cases = [
        (TokenKind::ScalarSigil, "$"),
        (TokenKind::ArraySigil, "@"),
        (TokenKind::HashSigil, "%"),
        (TokenKind::SubSigil, "&"),
    ];

    for (kind, sigil) in sigil_cases {
        let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(vec![LexerToken::new(
            TokenType::Identifier(sigil.into()),
            sigil,
            0,
            1,
        )]);
        assert_eq!(parser_tokens[0].kind, kind, "sigil mapping mismatch for `{sigil}`");
    }

    let quote_like_cases = [
        (TokenKind::QuoteSingle, "q{abc}"),
        (TokenKind::QuoteDouble, "qq{abc}"),
        (TokenKind::QuoteWords, "qw(one two)"),
        (TokenKind::QuoteCommand, "qx{echo hi}"),
        (TokenKind::Regex, "qr/abc/"),
        (TokenKind::Regex, "m/abc/"),
        (TokenKind::Substitution, "s/a/b/"),
        (TokenKind::Transliteration, "tr/a/b/"),
    ];

    for (kind, snippet) in quote_like_cases {
        assert_eq!(first_kind(snippet), kind, "mapping mismatch for `{snippet}`");
    }

    let heredoc_tokens = TokenStream::lexer_tokens_to_parser_tokens(vec![
        LexerToken::new(TokenType::HeredocStart, "<<'END'", 0, 7),
        LexerToken::new(TokenType::HeredocBody("hello".into()), "hello", 8, 13),
    ]);
    assert_eq!(heredoc_tokens[0].kind, TokenKind::HeredocStart);
    assert_eq!(heredoc_tokens[1].kind, TokenKind::HeredocBody);

    let data_source = "__DATA__\nline\n";
    let mut stream = TokenStream::new(data_source);
    assert_eq!(must(stream.next()).kind, TokenKind::DataMarker);
    assert_eq!(must(stream.next()).kind, TokenKind::DataBody);

    let mut eof_stream = TokenStream::new("");
    assert_eq!(must(eof_stream.peek()).kind, TokenKind::Eof);

    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(vec![
        LexerToken::new(TokenType::UnknownRest, "???", 0, 3),
        LexerToken::new(TokenType::Error("bad".into()), "@", 3, 4),
    ]);
    assert_eq!(parser_tokens[0].kind, TokenKind::UnknownRest);
    assert_eq!(parser_tokens[1].kind, TokenKind::Unknown);
}
