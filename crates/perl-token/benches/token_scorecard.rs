use perl_lexer::{PerlLexer, Token as LexerToken, TokenType};
use perl_parser_core::tokens::token_stream::TokenStream;
use perl_token::{Token, TokenKind};
use std::hint::black_box;
use std::sync::Arc;

#[path = "support/perf_scorecard.rs"]
mod perf_scorecard;

const LONG_TOKEN_TEXT: &str = "this_is_a_long_token_text_value_used_for_scorecard_measurement_0123456789_abcdefghijklmnopqrstuvwxyz";
const SAMPLE_SOURCE: &str = r#"
my $x = 42;
my $name = "perl";
if ($x > 0) {
    print $name;
}
"#;

fn main() {
    let token_new_short = perf_scorecard::sample_metric("token_new_short", 80, || {
        let t = Token::new(TokenKind::Identifier, black_box("id"), black_box(0), black_box(2));
        black_box(t);
    });
    perf_scorecard::record_metric(token_new_short);

    let token_new_long = perf_scorecard::sample_metric("token_new_long", 80, || {
        let t = Token::new(
            TokenKind::Identifier,
            black_box(LONG_TOKEN_TEXT),
            black_box(0),
            black_box(LONG_TOKEN_TEXT.len()),
        );
        black_box(t);
    });
    perf_scorecard::record_metric(token_new_long);

    let base_clone = Token::new(
        TokenKind::Identifier,
        Arc::<str>::from(LONG_TOKEN_TEXT),
        0,
        LONG_TOKEN_TEXT.len(),
    );
    let token_clone = perf_scorecard::sample_metric("token_clone", 100, || {
        let cloned = black_box(base_clone.clone());
        black_box(cloned);
    });
    perf_scorecard::record_metric(token_clone);

    let left = Token::new(TokenKind::Identifier, "equal_me", 10, 18);
    let right = Token::new(TokenKind::Identifier, "equal_me", 10, 18);
    let token_equality = perf_scorecard::sample_metric("token_equality", 100, || {
        let is_equal = black_box(left == right);
        black_box(is_equal);
    });
    perf_scorecard::record_metric(token_equality);

    let display_name = perf_scorecard::sample_metric("tokenkind_display_name", 100, || {
        let name = black_box(TokenKind::Sub.display_name());
        black_box(name);
    });
    perf_scorecard::record_metric(display_name);

    let category_predicates =
        perf_scorecard::sample_metric("tokenkind_category_predicates", 100, || {
            let kind = black_box(TokenKind::String);
            let _ = black_box(kind.is_keyword());
            let _ = black_box(kind.is_operator());
            let _ = black_box(kind.is_delimiter());
            let _ = black_box(kind.is_literal());
        });
    perf_scorecard::record_metric(category_predicates);

    let lexer_tokens = collect_lexer_tokens(SAMPLE_SOURCE);
    let conversion = perf_scorecard::sample_metric("lexer_to_parser_token_conversion", 50, || {
        let converted = TokenStream::lexer_tokens_to_parser_tokens(black_box(lexer_tokens.clone()));
        black_box(converted);
    });
    perf_scorecard::record_metric(conversion);

    let eof_synthetic =
        perf_scorecard::sample_metric("eof_and_synthetic_token_construction", 100, || {
            let eof = Token::new(TokenKind::Eof, "", 128, 128);
            let synthetic = Token::new(TokenKind::Unknown, "<synthetic>", 128, 128);
            black_box((eof, synthetic));
        });
    perf_scorecard::record_metric(eof_synthetic);
}

fn collect_lexer_tokens(source: &str) -> Vec<LexerToken> {
    let mut lexer = PerlLexer::new(source);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        let is_eof = token.token_type == TokenType::EOF;
        tokens.push(token);
        if is_eof {
            break;
        }
    }
    tokens
}
