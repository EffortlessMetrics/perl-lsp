#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Benchmarks for the perl-parser crate
//!
//! This benchmark suite measures the performance of the modern two-crate
//! architecture and enables comparison with other implementations.

use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser::{Parser, ScopeAnalyzer};
use serde::{Deserialize, Serialize};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const SIMPLE_SCRIPT: &str = r#"
my $x = 42;
my $y = "Hello, World!";
my @array = (1, 2, 3, 4, 5);
my %hash = (key => "value", foo => "bar");

if ($x > 40) {
    print "$y\n";
}

sub calculate {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = calculate(10, 20);
"#;

const COMPLEX_SCRIPT: &str = r#"
package MyModule;
use strict;
use warnings;

sub new {
    my $class = shift;
    my $self = {
        name => shift,
        value => shift || 0,
    };
    bless $self, $class;
    return $self;
}

sub process {
    my $self = shift;
    my @data = @_;
    
    my @results;
    foreach my $item (@data) {
        if ($item =~ /^(\d+)$/) {
            push @results, $1 * $self->{value};
        } elsif ($item =~ /^(\w+)=(\d+)$/) {
            push @results, { $1 => $2 * $self->{value} };
        }
    }
    
    return \@results;
}

sub fibonacci {
    my $n = shift;
    return $n if $n <= 1;
    
    my ($prev, $curr) = (0, 1);
    for (my $i = 2; $i <= $n; $i++) {
        ($prev, $curr) = ($curr, $prev + $curr);
    }
    return $curr;
}

1;
"#;

const SCORECARD_FILENAME: &str = "parser_performance_scorecard.json";

#[derive(Debug, Clone)]
struct ScorecardSample {
    scenario: &'static str,
    unit: &'static str,
    iterations: usize,
    mean_ns: u64,
    median_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ParserPerformanceScorecard {
    schema_version: u32,
    measured_at_unix_s: u64,
    results: Vec<ScorecardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScorecardEntry {
    scenario: String,
    unit: String,
    iterations: usize,
    mean_ns: u64,
    median_ns: u64,
}

fn run_scorecard_sample(iterations: usize, mut op: impl FnMut()) -> (u64, u64) {
    let mut samples: Vec<u64> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        op();
        let elapsed = started.elapsed().as_nanos();
        let sample = u64::try_from(elapsed).unwrap_or(u64::MAX);
        samples.push(sample);
    }

    samples.sort_unstable();
    let total: u128 = samples.iter().map(|&v| u128::from(v)).sum();
    let mean_ns = u64::try_from(total / iterations as u128).unwrap_or(u64::MAX);
    let median_ns = samples[iterations / 2];
    (mean_ns, median_ns)
}

fn sample_lexer_only() -> ScorecardSample {
    use perl_lexer::{PerlLexer, TokenType};
    let iterations = 25;
    let (mean_ns, median_ns) = run_scorecard_sample(iterations, || {
        let mut lexer = PerlLexer::new(COMPLEX_SCRIPT);
        loop {
            let Some(token) = lexer.next_token() else {
                break;
            };
            if matches!(token.token_type, TokenType::EOF) {
                break;
            }
        }
    });
    ScorecardSample { scenario: "lexer_only", unit: "ns", iterations, mean_ns, median_ns }
}

fn sample_scope_analysis() -> ScorecardSample {
    let mut parser = Parser::new(COMPLEX_SCRIPT);
    let ast = parser.parse().expect("COMPLEX_SCRIPT must parse for scope sample");
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];
    let iterations = 25;
    let (mean_ns, median_ns) = run_scorecard_sample(iterations, || {
        analyzer.analyze(&ast, COMPLEX_SCRIPT, &pragma_map);
    });
    ScorecardSample { scenario: "scope_analysis", unit: "ns", iterations, mean_ns, median_ns }
}

fn emit_scorecard(samples: &[ScorecardSample]) {
    let path = scorecard_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut scorecard = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ParserPerformanceScorecard>(&raw).ok())
        .unwrap_or_default();
    scorecard.schema_version = 1;
    scorecard.measured_at_unix_s =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs());

    for sample in samples {
        let entry = ScorecardEntry {
            scenario: sample.scenario.to_string(),
            unit: sample.unit.to_string(),
            iterations: sample.iterations,
            mean_ns: sample.mean_ns,
            median_ns: sample.median_ns,
        };
        if let Some(existing) =
            scorecard.results.iter_mut().find(|row| row.scenario == entry.scenario)
        {
            *existing = entry;
        } else {
            scorecard.results.push(entry);
        }
    }
    scorecard.results.sort_by(|a, b| a.scenario.cmp(&b.scenario));

    if let Ok(raw) = serde_json::to_string_pretty(&scorecard) {
        let _ = fs::write(path, raw);
    }
}

fn scorecard_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("metrics")
        .join(SCORECARD_FILENAME)
}

fn benchmark_simple_parsing(c: &mut Criterion) {
    c.bench_function("parse_simple_script", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(SIMPLE_SCRIPT));
            let _ = parser.parse();
        });
    });
}

fn benchmark_complex_parsing(c: &mut Criterion) {
    c.bench_function("parse_complex_script", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(COMPLEX_SCRIPT));
            let _ = parser.parse();
        });
    });
}

fn benchmark_ast_generation(c: &mut Criterion) {
    let mut parser = Parser::new(COMPLEX_SCRIPT);
    let ast = parser.parse().expect("COMPLEX_SCRIPT must parse for benchmark");

    c.bench_function("ast_to_sexp", |b| {
        b.iter(|| {
            let _ = black_box(ast.to_sexp());
        });
    });
}

fn benchmark_isolated_components(c: &mut Criterion) {
    // Benchmark just the lexer phase
    c.bench_function("lexer_only", |b| {
        use perl_lexer::{PerlLexer, TokenType};

        b.iter(|| {
            let mut lexer = PerlLexer::new(black_box(COMPLEX_SCRIPT));
            let mut count = 0;

            while let Some(token) = lexer.next_token() {
                if matches!(token.token_type, TokenType::EOF) {
                    break;
                }
                count += 1;
            }

            black_box(count);
        });
    });

    // Benchmark parser with pre-tokenized input (simulated)
    // This would require exposing more internals, so we skip for now
}

fn benchmark_scope_analysis(c: &mut Criterion) {
    let mut parser = Parser::new(COMPLEX_SCRIPT);
    let ast = parser.parse().expect("COMPLEX_SCRIPT must parse for benchmark");
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];

    c.bench_function("scope_analysis", |b| {
        b.iter(|| {
            analyzer.analyze(black_box(&ast), black_box(COMPLEX_SCRIPT), black_box(&pragma_map));
        });
    });
}

fn benchmark_emit_scorecard(_c: &mut Criterion) {
    static SCORECARD_EMITTED: OnceLock<()> = OnceLock::new();
    _c.bench_function("emit_parser_performance_scorecard_parser", |b| {
        b.iter(|| {
            SCORECARD_EMITTED.get_or_init(|| {
                let samples = vec![sample_lexer_only(), sample_scope_analysis()];
                emit_scorecard(&samples);
            });
        });
    });
}

criterion_group!(
    benches,
    benchmark_simple_parsing,
    benchmark_complex_parsing,
    benchmark_ast_generation,
    benchmark_isolated_components,
    benchmark_scope_analysis,
    benchmark_emit_scorecard
);
criterion_main!(benches);
