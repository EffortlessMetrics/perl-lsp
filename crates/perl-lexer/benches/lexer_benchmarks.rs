use criterion::{Criterion, criterion_group, criterion_main};
use perl_lexer::{PerlLexer, Token};
use serde_json::{Map, json};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

const SCORECARD_SCHEMA_VERSION: u32 = 1;
const SCORECARD_SAMPLE_COUNT: usize = 200;
const SCORECARD_WARMUP_RUNS: usize = 20;

struct BenchScenario {
    name: &'static str,
    input: String,
}

fn collect_all_tokens(mut lexer: PerlLexer) -> Vec<Token> {
    lexer.collect_tokens()
}

fn scorecard_path() -> PathBuf {
    if let Ok(path) = std::env::var("PERL_LEXER_SCORECARD_PATH") {
        return PathBuf::from(path);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("criterion")
        .join("lexer_scorecard.json")
}

fn bench_scenarios() -> Vec<BenchScenario> {
    let mut large_file = String::new();
    for i in 0..1000 {
        large_file.push_str(&format!("my $var{} = {};\n", i, i));
        large_file.push_str(&format!("print \"Value: $var{}\\n\";\n", i));
        if i % 10 == 0 {
            large_file.push_str(&format!("if ($var{} =~ /\\d+/) {{\n", i));
            large_file.push_str(&format!("    $var{} = $var{} / 2;\n", i, i));
            large_file.push_str("}\n");
        }
    }

    let keyword_heavy =
        "if else while until for foreach return last next redo package require default continue"
            .repeat(100);

    vec![
        BenchScenario { name: "simple_tokens", input: "my $x = 42; print $x;".to_string() },
        BenchScenario {
            name: "slash_disambiguation",
            input: r#"
                my $x = 10 / 2;
                if ($str =~ /pattern/) {
                    $str =~ s/foo/bar/g;
                }
                print 1/ /abc/;
            "#
            .to_string(),
        },
        BenchScenario {
            name: "string_interpolation",
            input: r#"
                my $name = "World";
                print "Hello, $name!\n";
                print "The answer is ${count + 1}\n";
                print "Array: @items\n";
            "#
            .to_string(),
        },
        BenchScenario { name: "large_file", input: large_file },
        BenchScenario {
            name: "whitespace_heavy",
            input: r#"
            # This is a comment
            my   $x   =   42  ;  # Another comment

            print    $x    ;

            # More comments
            "#
            .to_string(),
        },
        BenchScenario {
            name: "operator_heavy",
            input: "$a += $b -= $c *= $d /= $e %= $f **= $g &&= $h ||= $i //= $j".to_string(),
        },
        BenchScenario {
            name: "number_parsing",
            input: "123 456.789 1_234_567 1.23e45 0xFF 0377 0b1010".to_string(),
        },
        BenchScenario { name: "keyword_heavy", input: keyword_heavy },
    ]
}

fn emit_scorecard() {
    let mut families = Map::new();
    let scenarios = bench_scenarios();

    for scenario in &scenarios {
        for _ in 0..SCORECARD_WARMUP_RUNS {
            let _ = collect_all_tokens(PerlLexer::new(&scenario.input));
        }

        let mut sample_time_ns = Vec::with_capacity(SCORECARD_SAMPLE_COUNT);
        let mut sample_tokens = Vec::with_capacity(SCORECARD_SAMPLE_COUNT);
        for _ in 0..SCORECARD_SAMPLE_COUNT {
            let start = Instant::now();
            let tokens = collect_all_tokens(PerlLexer::new(&scenario.input));
            sample_time_ns.push(start.elapsed().as_nanos() as f64);
            sample_tokens.push(tokens.len() as f64);
        }

        let total_time_ns: f64 = sample_time_ns.iter().sum();
        let total_tokens: f64 = sample_tokens.iter().sum();
        let mean_time_ns = total_time_ns / SCORECARD_SAMPLE_COUNT as f64;
        let variance = sample_time_ns
            .iter()
            .map(|sample| {
                let delta = *sample - mean_time_ns;
                delta * delta
            })
            .sum::<f64>()
            / SCORECARD_SAMPLE_COUNT as f64;
        let std_dev_ns = variance.sqrt();
        let min_time_ns = sample_time_ns.iter().copied().fold(f64::INFINITY, f64::min);
        let max_time_ns = sample_time_ns.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let tokens_per_second = if total_time_ns > 0.0 {
            total_tokens / (total_time_ns / 1_000_000_000.0)
        } else {
            0.0
        };
        let throughput_bytes_per_second = if total_time_ns > 0.0 {
            (scenario.input.len() as f64 * SCORECARD_SAMPLE_COUNT as f64)
                / (total_time_ns / 1_000_000_000.0)
        } else {
            0.0
        };

        families.insert(
            scenario.name.to_string(),
            json!({
                "input_bytes": scenario.input.len(),
                "sample_count": SCORECARD_SAMPLE_COUNT,
                "total_time_ns": total_time_ns,
                "mean_time_ns": mean_time_ns,
                "std_dev_ns": std_dev_ns,
                "min_time_ns": min_time_ns,
                "max_time_ns": max_time_ns,
                "total_tokens": total_tokens,
                "tokens_per_second": tokens_per_second,
                "throughput_bytes_per_second": throughput_bytes_per_second
            }),
        );
    }

    let output = json!({
        "schema_version": SCORECARD_SCHEMA_VERSION,
        "tool": "perl-lexer criterion benchmark",
        "benchmark": "lexer_benchmarks",
        "sample_count": SCORECARD_SAMPLE_COUNT,
        "families": families
    });

    let output_path = scorecard_path();
    let Some(parent) = output_path.parent() else {
        eprintln!("failed to emit lexer scorecard: missing parent dir");
        return;
    };

    if let Err(error) = fs::create_dir_all(parent) {
        eprintln!("failed to emit lexer scorecard directory: {error}");
        return;
    }

    let encoded = match serde_json::to_string_pretty(&output) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to serialize lexer scorecard: {error}");
            return;
        }
    };

    if let Err(error) = fs::write(&output_path, encoded) {
        eprintln!("failed to write lexer scorecard {}: {error}", output_path.display());
        return;
    }

    println!("lexer scorecard written to {}", output_path.display());
}

fn configure_criterion() -> Criterion {
    emit_scorecard();
    Criterion::default()
}

fn bench_simple_tokens(c: &mut Criterion) {
    let input = "my $x = 42; print $x;";

    c.bench_function("simple_tokens", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_slash_disambiguation(c: &mut Criterion) {
    let input = r#"
        my $x = 10 / 2;
        if ($str =~ /pattern/) {
            $str =~ s/foo/bar/g;
        }
        print 1/ /abc/;
    "#;

    c.bench_function("slash_disambiguation", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_string_interpolation(c: &mut Criterion) {
    let input = r#"
        my $name = "World";
        print "Hello, $name!\n";
        print "The answer is ${count + 1}\n";
        print "Array: @items\n";
    "#;

    c.bench_function("string_interpolation", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_large_file(c: &mut Criterion) {
    let mut input = String::new();
    for i in 0..1000 {
        input.push_str(&format!("my $var{} = {};\n", i, i));
        input.push_str(&format!("print \"Value: $var{}\n\";\n", i));
        if i % 10 == 0 {
            input.push_str(&format!("if ($var{} =~ /\\d+/) {{\n", i));
            input.push_str(&format!("    $var{} = $var{} / 2;\n", i, i));
            input.push_str("}\n");
        }
    }

    c.bench_function("large_file", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(&input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_whitespace_heavy(c: &mut Criterion) {
    let input = r#"
    # This is a comment
    my   $x   =   42  ;  # Another comment

    print    $x    ;

    # More comments
    "#;

    c.bench_function("whitespace_heavy", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_operator_heavy(c: &mut Criterion) {
    let input = "$a += $b -= $c *= $d /= $e %= $f **= $g &&= $h ||= $i //= $j";

    c.bench_function("operator_heavy", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_number_parsing(c: &mut Criterion) {
    let input = "123 456.789 1_234_567 1.23e45 0xFF 0377 0b1010";

    c.bench_function("number_parsing", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
}

fn bench_keyword_heavy(c: &mut Criterion) {
    let base =
        "if else while until for foreach return last next redo package require default continue";
    let input = base.repeat(100);

    c.bench_function("keyword_heavy", |b| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(&input));
            collect_all_tokens(lexer)
        });
    });
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets =
        bench_simple_tokens,
        bench_slash_disambiguation,
        bench_string_interpolation,
        bench_large_file,
        bench_whitespace_heavy,
        bench_operator_heavy,
        bench_number_parsing,
        bench_keyword_heavy
}
criterion_main!(benches);
