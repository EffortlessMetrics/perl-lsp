use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_token::{Token, TokenKind};
use serde::Serialize;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const SCORECARD_PATH: &str = "target/metrics/token_scorecard.json";
const WARMUP_ROUNDS: usize = 2;
const SCORE_ROUNDS: usize = 20_000;

#[derive(Debug, Clone, Serialize)]
struct ScoreMetric {
    benchmark: String,
    iterations: usize,
    median_ns: u128,
    p95_ns: u128,
}

#[derive(Debug, Clone, Serialize)]
struct TokenScorecard {
    schema_version: u32,
    generated_at_epoch_s: u64,
    warmup_rounds: usize,
    metrics: Vec<ScoreMetric>,
}

fn main() {
    let metrics = vec![
        sample("token_new_short", || {
            black_box(Token::new(TokenKind::Identifier, "x", 0, 1));
        }),
        sample("token_new_long", || {
            black_box(Token::new(TokenKind::String, "long_token_text_".repeat(32), 0, 512));
        }),
        sample("token_clone", || {
            let token = Token::new(TokenKind::Identifier, "shared", 12, 18);
            black_box(token.clone());
        }),
        sample("token_equality", || {
            let left = Token::new(TokenKind::Identifier, "value", 2, 7);
            let right = left.clone();
            black_box(left == right);
        }),
        sample("token_kind_display_name", || {
            black_box(TokenKind::LeftBrace.display_name());
        }),
        sample("token_kind_category_predicates", || {
            black_box(TokenKind::My.is_keyword());
            black_box(TokenKind::Assign.is_operator());
            black_box(TokenKind::LeftParen.is_delimiter());
            black_box(TokenKind::String.is_literal());
        }),
        sample("lexer_to_parser_token_conversion", || {
            let lexer_token =
                LexerToken::new(LexerTokenType::Keyword(Arc::from("my")), Arc::from("my"), 0, 2);
            black_box(convert_lexer_token_to_parser_token(lexer_token));
        }),
        sample("eof_synthetic_token_construction", || {
            black_box(Token::new(TokenKind::Eof, "", 99, 99));
        }),
    ];

    let scorecard = TokenScorecard {
        schema_version: 1,
        generated_at_epoch_s: now_epoch_seconds(),
        warmup_rounds: WARMUP_ROUNDS,
        metrics,
    };

    if let Ok(json) = serde_json::to_string_pretty(&scorecard) {
        println!("{json}");
        if let Some(path) = find_artifact_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, json);
        }
    }
}

fn sample<F>(benchmark: &str, mut run: F) -> ScoreMetric
where
    F: FnMut(),
{
    for _ in 0..WARMUP_ROUNDS {
        run();
    }

    let mut samples = Vec::with_capacity(SCORE_ROUNDS);
    for _ in 0..SCORE_ROUNDS {
        let start = Instant::now();
        run();
        samples.push(start.elapsed().as_nanos());
    }

    samples.sort_unstable();
    let n = samples.len();

    let median_idx = n / 2;
    let p95_idx = percentile_index(n, 95);

    ScoreMetric {
        benchmark: benchmark.to_string(),
        iterations: n,
        median_ns: samples.get(median_idx).copied().unwrap_or_default(),
        p95_ns: samples.get(p95_idx).copied().unwrap_or_default(),
    }
}

fn percentile_index(sample_count: usize, percentile: usize) -> usize {
    ((sample_count * percentile + 99) / 100).saturating_sub(1).min(sample_count.saturating_sub(1))
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn find_artifact_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(SCORECARD_PATH);
        if dir.join("Cargo.toml").exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn convert_lexer_token_to_parser_token(token: LexerToken) -> Token {
    let kind = match &token.token_type {
        LexerTokenType::Keyword(keyword) => match keyword.as_ref() {
            "my" => TokenKind::My,
            "sub" => TokenKind::Sub,
            _ => TokenKind::Identifier,
        },
        LexerTokenType::Operator(operator) => match operator.as_ref() {
            "=" => TokenKind::Assign,
            "->" => TokenKind::Arrow,
            _ => TokenKind::Unknown,
        },
        LexerTokenType::Identifier(_) => TokenKind::Identifier,
        LexerTokenType::Number(_) => TokenKind::Number,
        LexerTokenType::StringLiteral => TokenKind::String,
        LexerTokenType::LeftParen => TokenKind::LeftParen,
        LexerTokenType::RightParen => TokenKind::RightParen,
        LexerTokenType::LeftBrace => TokenKind::LeftBrace,
        LexerTokenType::RightBrace => TokenKind::RightBrace,
        LexerTokenType::LeftBracket => TokenKind::LeftBracket,
        LexerTokenType::RightBracket => TokenKind::RightBracket,
        LexerTokenType::Semicolon => TokenKind::Semicolon,
        LexerTokenType::Comma => TokenKind::Comma,
        LexerTokenType::EOF => TokenKind::Eof,
        _ => TokenKind::Unknown,
    };

    Token::new(kind, token.text, token.start, token.end)
}
