use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};

fn parser_kinds_for(source: &str) -> Vec<TokenKind> {
    let mut lexer = PerlLexer::new(source);
    let mut raw = Vec::new();
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
        raw.push(token);
    }

    TokenStream::lexer_tokens_to_parser_tokens(raw).into_iter().map(|token| token.kind).collect()
}

fn parser_kind_for_operator(op: &str) -> TokenKind {
    TokenStream::lexer_tokens_to_parser_tokens(vec![perl_lexer::Token::new(
        TokenType::Operator(op.into()),
        op,
        0,
        op.len(),
    )])[0]
        .kind
}

#[test]
fn token_kind_keyword_operator_and_delimiter_tables_round_trip_to_parser_kinds() {
    for (kind, spelling) in TokenKind::KEYWORD_SPELLINGS {
        assert_eq!(
            parser_kinds_for(spelling).first().copied(),
            Some(*kind),
            "keyword mismatch for `{spelling}`"
        );
    }

    for (kind, spelling) in TokenKind::DELIMITER_SPELLINGS {
        assert_eq!(
            parser_kinds_for(spelling).first().copied(),
            Some(*kind),
            "delimiter mismatch for `{spelling}`"
        );
    }

    for (kind, spelling) in TokenKind::OPERATOR_SPELLINGS {
        if *spelling == "cmp"
            || *spelling == "and"
            || *spelling == "or"
            || *spelling == "not"
            || *spelling == "xor"
        {
            let snippet = match *spelling {
                "not" => "not 1",
                "cmp" => "a cmp b",
                "and" => "1 and 2",
                "or" => "1 or 2",
                "xor" => "1 xor 2",
                _ => unreachable!(),
            };
            assert!(parser_kinds_for(snippet).contains(kind), "operator mismatch for `{spelling}`");
            continue;
        }
        assert_eq!(parser_kind_for_operator(spelling), *kind, "operator mismatch for `{spelling}`");
    }
}

#[test]
fn token_kind_quote_like_regex_heredoc_and_data_tokens_conform() {
    let quote_like_cases = [
        (TokenKind::QuoteSingle, "q{abc}"),
        (TokenKind::QuoteDouble, "qq{abc}"),
        (TokenKind::QuoteWords, "qw(one two)"),
        (TokenKind::QuoteCommand, "qx{echo hi}"),
        (TokenKind::Regex, "m/abc/"),
        (TokenKind::Regex, "qr/abc/"),
        (TokenKind::Substitution, "s/a/b/"),
        (TokenKind::Transliteration, "y/a/b/"),
    ];

    for (kind, snippet) in quote_like_cases {
        assert_eq!(
            parser_kinds_for(snippet).first().copied(),
            Some(kind),
            "quote-like mismatch for `{snippet}`"
        );
    }

    let heredoc = parser_kinds_for("print <<'END';\nbody\nEND\n");
    assert!(heredoc.contains(&TokenKind::HeredocStart));

    let data = parser_kinds_for("__DATA__\nbody\n");
    assert_eq!(data.first().copied(), Some(TokenKind::DataMarker));
    assert!(data.contains(&TokenKind::DataBody));
}
