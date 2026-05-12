use std::sync::Arc;

use perl_lexer::{PerlLexer, Token as LexerToken, TokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};
use perl_token::{DELIMITER_SPELLINGS, KEYWORD_SPELLINGS, OPERATOR_SPELLINGS};

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

fn converted_kind(token_type: TokenType, text: &str) -> Option<TokenKind> {
    TokenStream::lexer_tokens_to_parser_tokens(vec![LexerToken {
        token_type,
        text: Arc::from(text),
        start: 0,
        end: text.len(),
    }])
    .into_iter()
    .next()
    .map(|token| token.kind)
}

fn delimiter_token_type(kind: TokenKind) -> Option<TokenType> {
    match kind {
        TokenKind::LeftParen => Some(TokenType::LeftParen),
        TokenKind::RightParen => Some(TokenType::RightParen),
        TokenKind::LeftBrace => Some(TokenType::LeftBrace),
        TokenKind::RightBrace => Some(TokenType::RightBrace),
        TokenKind::LeftBracket => Some(TokenType::LeftBracket),
        TokenKind::RightBracket => Some(TokenType::RightBracket),
        TokenKind::Semicolon => Some(TokenType::Semicolon),
        TokenKind::Comma => Some(TokenType::Comma),
        _ => None,
    }
}

#[test]
fn canonical_keyword_spellings_convert_from_raw_lexer_tokens() {
    for &(spelling, expected) in KEYWORD_SPELLINGS {
        assert_eq!(
            converted_kind(TokenType::Keyword(Arc::from(spelling)), spelling),
            Some(expected),
            "keyword {spelling:?}"
        );
    }
}

#[test]
fn canonical_operator_spellings_convert_from_raw_lexer_tokens() {
    for &(spelling, expected) in OPERATOR_SPELLINGS {
        assert_eq!(
            converted_kind(TokenType::Operator(Arc::from(spelling)), spelling),
            Some(expected),
            "operator {spelling:?}"
        );
    }
}

#[test]
fn canonical_delimiter_spellings_convert_from_raw_lexer_tokens() {
    for &(spelling, expected) in DELIMITER_SPELLINGS {
        assert!(
            delimiter_token_type(expected).is_some(),
            "delimiter {expected:?} needs lexer token mapping"
        );
        if let Some(token_type) = delimiter_token_type(expected) {
            assert_eq!(
                converted_kind(token_type, spelling),
                Some(expected),
                "delimiter {spelling}"
            );
        }
    }
}

#[test]
fn parser_specific_literal_tokens_convert_from_raw_lexer_tokens() {
    let cases = [
        (TokenType::QuoteWords, "qw(foo bar)", TokenKind::QuoteWords),
        (TokenType::HeredocStart, "<<'END'", TokenKind::HeredocStart),
        (TokenType::HeredocBody(Arc::from("body\n")), "body\n", TokenKind::HeredocBody),
        (TokenType::DataMarker(Arc::from("__DATA__")), "__DATA__", TokenKind::DataMarker),
        (TokenType::DataBody(Arc::from("payload\n")), "payload\n", TokenKind::DataBody),
        (TokenType::UnknownRest, "unterminated", TokenKind::UnknownRest),
    ];

    for (token_type, text, expected) in cases {
        assert_eq!(converted_kind(token_type, text), Some(expected), "literal {expected:?}");
    }
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
    // This test constructs the Identifier path directly via lexer_tokens_to_parser_tokens
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
