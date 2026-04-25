use criterion::{Criterion, criterion_group, criterion_main};
#[path = "support/perf_scorecard.rs"]
mod perf_scorecard;
use perl_lexer::{Token as LexerToken, TokenType};
use perl_parser_core::tokens::token_stream::TokenStream;
use perl_token::{Token, TokenKind};
use std::hint::black_box;

const SHORT_TOKEN_TEXT: &str = "my";
const LONG_TOKEN_TEXT: &str =
    "ThisIsALongerTokenPayloadToMeasureArcAllocationAndCopyCostsAcrossParserBoundaries";

fn run_scorecard() {
    perf_scorecard::record_metric(
        "token_new_short",
        perf_scorecard::sample_metric(5000, || {
            let token =
                Token::new(TokenKind::Identifier, SHORT_TOKEN_TEXT, 0, SHORT_TOKEN_TEXT.len());
            black_box(token);
        }),
    );

    perf_scorecard::record_metric(
        "token_new_long",
        perf_scorecard::sample_metric(5000, || {
            let token =
                Token::new(TokenKind::Identifier, LONG_TOKEN_TEXT, 0, LONG_TOKEN_TEXT.len());
            black_box(token);
        }),
    );

    let base_token =
        Token::new(TokenKind::Identifier, LONG_TOKEN_TEXT, 4, 4 + LONG_TOKEN_TEXT.len());
    perf_scorecard::record_metric(
        "token_clone",
        perf_scorecard::sample_metric(5000, || {
            let token = base_token.clone();
            black_box(token);
        }),
    );

    let lhs = Token::new(TokenKind::Identifier, "same", 10, 14);
    let rhs = Token::new(TokenKind::Identifier, "same", 10, 14);
    perf_scorecard::record_metric(
        "token_equality",
        perf_scorecard::sample_metric(5000, || {
            black_box(lhs == rhs);
        }),
    );

    perf_scorecard::record_metric(
        "token_kind_display_name",
        perf_scorecard::sample_metric(5000, || {
            black_box(TokenKind::LeftBrace.display_name());
            black_box(TokenKind::Identifier.display_name());
            black_box(TokenKind::Eof.display_name());
        }),
    );

    perf_scorecard::record_metric(
        "token_kind_category_predicates",
        perf_scorecard::sample_metric(5000, || {
            black_box(TokenKind::If.is_keyword());
            black_box(TokenKind::Plus.is_operator());
            black_box(TokenKind::String.is_literal());
        }),
    );

    let lexer_tokens = vec![
        LexerToken::new(TokenType::Keyword("my".into()), "my", 0, 2),
        LexerToken::new(TokenType::Identifier("$x".into()), "$x", 3, 5),
        LexerToken::new(TokenType::Operator("=".into()), "=", 6, 7),
        LexerToken::new(TokenType::Number("42".into()), "42", 8, 10),
        LexerToken::new(TokenType::Semicolon, ";", 10, 11),
    ];
    perf_scorecard::record_metric(
        "lexer_to_parser_token_conversion",
        perf_scorecard::sample_metric(3000, || {
            let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(lexer_tokens.clone());
            black_box(parser_tokens);
        }),
    );

    perf_scorecard::record_metric(
        "eof_synthetic_token_construction",
        perf_scorecard::sample_metric(5000, || {
            let eof = Token::new(TokenKind::Eof, "", 0, 0);
            let synthetic = Token::new(TokenKind::Unknown, "?", 0, 1);
            black_box((eof, synthetic));
        }),
    );
}

fn benchmark_token_scorecard(c: &mut Criterion) {
    run_scorecard();

    c.bench_function("token_scorecard_recorded", |b| {
        b.iter(|| {
            black_box(0usize);
        });
    });
}

criterion_group!(benches, benchmark_token_scorecard);
criterion_main!(benches);
