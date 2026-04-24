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
