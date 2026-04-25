use perl_lexer::{PerlLexer, Token as LexerToken, TokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};
use perl_tdd_support::must;
use std::sync::Arc;

fn first_kind(source: &str) -> TokenKind {
    let mut stream = TokenStream::new(source);
    must(stream.peek()).kind
}

fn parser_kinds_for(input: &str) -> Vec<TokenKind> {
    let mut lexer = PerlLexer::new(input);
    let mut raw = Vec::new();

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
        raw.push(token);
    }

    TokenStream::lexer_tokens_to_parser_tokens(raw).into_iter().map(|t| t.kind).collect()
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

#[test]
fn keyword_and_word_operator_tokens_flow_through_shared_mapping() {
    let kinds = parser_kinds_for("my and or not xor cmp no");
    assert_eq!(
        kinds,
        vec![
            TokenKind::My,
            TokenKind::WordAnd,
            TokenKind::WordOr,
            TokenKind::WordNot,
            TokenKind::WordXor,
            TokenKind::StringCompare,
            TokenKind::No,
        ]
    );
}

#[test]
fn quote_words_keyword_stays_identifier_for_parser_specific_handling() {
    let kinds = parser_kinds_for("qw");
    assert_eq!(kinds, vec![TokenKind::Identifier]);
}

#[test]
fn sigils_can_arrive_via_operator_or_identifier_paths() {
    let kinds = parser_kinds_for("$ @ % & *");
    assert_eq!(
        kinds,
        vec![
            TokenKind::ScalarSigil,
            TokenKind::ArraySigil,
            TokenKind::Percent,
            TokenKind::BitwiseAnd,
            TokenKind::Star,
        ]
    );
}

#[test]
fn delimiter_error_recovery_uses_shared_delimiter_mapping() {
    let kinds = parser_kinds_for("{ }");
    assert_eq!(kinds, vec![TokenKind::LeftBrace, TokenKind::RightBrace]);
}

#[test]
fn hash_and_sub_sigils_as_identifier_tokens_keep_sigil_kind() {
    // The lexer emits bare '%' and '&' as Identifier tokens when they appear
    // as postfix-dereference sigils (e.g. ->%{key} or %{$ref}).  The token-stream
    // conversion must produce HashSigil/SubSigil, NOT Percent/BitwiseAnd.
    // This test constructs Identifier path directly via lexer_tokens_to_parser_tokens
    // to avoid lexer mode ambiguity.
    let raw = vec![
        LexerToken {
            token_type: TokenType::Identifier(Arc::from("%")),
            text: Arc::from("%"),
            start: 0,
            end: 1,
        },
        LexerToken {
            token_type: TokenType::Identifier(Arc::from("&")),
            text: Arc::from("&"),
            start: 2,
            end: 3,
        },
    ];
    let kinds = TokenStream::lexer_tokens_to_parser_tokens(raw)
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![TokenKind::HashSigil, TokenKind::SubSigil],
        "bare %/& as Identifier tokens must map to sigil kinds, not operator kinds"
    );
}
