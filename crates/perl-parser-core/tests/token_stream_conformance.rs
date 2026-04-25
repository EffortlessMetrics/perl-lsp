use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::token_stream::{TokenKind, TokenStream};

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
    use perl_lexer::Token as LexerToken;
    use perl_lexer::TokenType as LexerTokenType;
    use std::sync::Arc;

    let raw = vec![
        LexerToken {
            token_type: LexerTokenType::Identifier(Arc::from("%")),
            text: Arc::from("%"),
            start: 0,
            end: 1,
        },
        LexerToken {
            token_type: LexerTokenType::Identifier(Arc::from("&")),
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

