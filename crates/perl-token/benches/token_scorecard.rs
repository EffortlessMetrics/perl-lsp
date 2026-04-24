use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_token::{Token, TokenKind};
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const ARTIFACT_RELATIVE_PATH: &str = "docs/project/status/token_performance_scorecard.json";
const WARMUP_ROUNDS: usize = 2;

#[derive(Debug, Clone)]
struct ScoreMetric {
    name: &'static str,
    iterations: usize,
    median_ns: u128,
    p95_ns: u128,
}

fn main() {
    let metrics = vec![
        run_case("token_new_short", 2_000, || {
            let token = Token::new(TokenKind::Identifier, "foo", 0, 3);
            black_box(token);
        }),
        run_case("token_new_long", 800, || {
            let token = Token::new(TokenKind::String, "x".repeat(4 * 1024), 0, 4 * 1024);
            black_box(token);
        }),
        run_case("token_clone", 3_000, || {
            let token =
                Token::new(TokenKind::Identifier, Arc::<str>::from("variable_name"), 10, 23);
            black_box(token.clone());
        }),
        run_case("token_equality", 3_000, || {
            let lhs = Token::new(TokenKind::Number, Arc::<str>::from("12345"), 20, 25);
            let rhs = Token::new(TokenKind::Number, Arc::<str>::from("12345"), 20, 25);
            black_box(lhs == rhs);
        }),
        run_case("token_kind_display_name", 6_000, || {
            black_box(TokenKind::HeredocDepthLimit.display_name());
        }),
        run_case("token_kind_category_predicates", 6_000, || {
            let kind = black_box(TokenKind::Substitution);
            black_box((
                kind.is_keyword(),
                kind.is_operator(),
                kind.is_delimiter(),
                kind.is_literal(),
            ));
        }),
        run_case("lexer_to_parser_token_conversion", 2_000, || {
            let lexer_token = LexerToken::new(LexerTokenType::Keyword("my".into()), "my", 0, 2);
            black_box(convert_lexer_token(lexer_token));
        }),
        run_case("eof_synthetic_token", 8_000, || {
            black_box(Token::new(TokenKind::Eof, Arc::<str>::from(""), 0, 0));
        }),
    ];

    for metric in &metrics {
        println!(
            "name={} iterations={} median_ns={} p95_ns={}",
            metric.name, metric.iterations, metric.median_ns, metric.p95_ns
        );
    }

    if let Some(path) = find_artifact_path() {
        let _ = write_scorecard(&path, &metrics);
    }
}

fn run_case<F>(name: &'static str, iterations: usize, mut run: F) -> ScoreMetric
where
    F: FnMut(),
{
    for _ in 0..WARMUP_ROUNDS {
        run();
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        run();
        samples.push(start.elapsed().as_nanos());
    }

    samples.sort_unstable();
    let median_idx = samples.len() / 2;
    let p95_idx = p95_index(samples.len());

    ScoreMetric {
        name,
        iterations,
        median_ns: samples.get(median_idx).copied().unwrap_or_default(),
        p95_ns: samples.get(p95_idx).copied().unwrap_or_default(),
    }
}

fn p95_index(sample_count: usize) -> usize {
    (sample_count * 95).div_ceil(100).saturating_sub(1).min(sample_count.saturating_sub(1))
}

fn convert_lexer_token(token: LexerToken) -> Token {
    let kind = match token.token_type {
        LexerTokenType::Keyword(kw) => match kw.as_ref() {
            "my" => TokenKind::My,
            "sub" => TokenKind::Sub,
            "if" => TokenKind::If,
            "return" => TokenKind::Return,
            _ => TokenKind::Identifier,
        },
        LexerTokenType::Identifier(_) => TokenKind::Identifier,
        LexerTokenType::Number(_) => TokenKind::Number,
        LexerTokenType::Operator(op) => match op.as_ref() {
            "+" => TokenKind::Plus,
            "=" => TokenKind::Assign,
            _ => TokenKind::Unknown,
        },
        LexerTokenType::EOF => TokenKind::Eof,
        _ => TokenKind::Unknown,
    };

    Token::new(kind, token.text, token.start, token.end)
}

fn find_artifact_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(ARTIFACT_RELATIVE_PATH);
        if candidate.parent().is_some_and(|parent| parent.exists()) {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn write_scorecard(path: &Path, metrics: &[ScoreMetric]) -> std::io::Result<()> {
    let generated_at_epoch_s =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs());

    let mut out = String::from("{\n");
    out.push_str("  \"schema_version\": 1,\n");
    out.push_str(&format!("  \"generated_at_epoch_s\": {},\n", generated_at_epoch_s));
    out.push_str("  \"metrics\": {\n");

    for (index, metric) in metrics.iter().enumerate() {
        let comma = if index + 1 == metrics.len() { "" } else { "," };
        out.push_str(&format!(
            "    \"{}\": {{\"iterations\": {}, \"median_ns\": {}, \"p95_ns\": {}}}{}\n",
            metric.name, metric.iterations, metric.median_ns, metric.p95_ns, comma
        ));
    }

    out.push_str("  }\n");
    out.push('}');
    out.push('\n');

    fs::write(path, out)
}
