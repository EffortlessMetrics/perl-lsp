use criterion::{BenchmarkId, Criterion, Throughput};
use perl_lexer::{PerlLexer, Token};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};

const BENCHMARKS: &[&str] = &[
    "simple_tokens",
    "slash_disambiguation",
    "string_interpolation",
    "large_file",
    "whitespace_heavy",
    "operator_heavy",
    "number_parsing",
    "keyword_heavy",
];

fn collect_all_tokens(mut lexer: PerlLexer) -> Vec<Token> {
    lexer.collect_tokens()
}

fn run_benchmark(c: &mut Criterion, benchmark_name: &str, input: &str) {
    let token_count = collect_all_tokens(PerlLexer::new(input)).len() as u64;
    let mut group = c.benchmark_group("lexer");
    group.throughput(Throughput::Elements(token_count));
    group.bench_with_input(BenchmarkId::new(benchmark_name, "input"), input, |b, input| {
        b.iter(|| {
            let lexer = PerlLexer::new(black_box(input));
            collect_all_tokens(lexer)
        });
    });
    group.finish();
}

fn bench_simple_tokens(c: &mut Criterion) {
    run_benchmark(c, "simple_tokens", "my $x = 42; print $x;");
}

fn bench_slash_disambiguation(c: &mut Criterion) {
    let input = r#"
        my $x = 10 / 2;
        if ($str =~ /pattern/) {
            $str =~ s/foo/bar/g;
        }
        print 1/ /abc/;
    "#;
    run_benchmark(c, "slash_disambiguation", input);
}

fn bench_string_interpolation(c: &mut Criterion) {
    let input = r#"
        my $name = "World";
        print "Hello, $name!\n";
        print "The answer is ${count + 1}\n";
        print "Array: @items\n";
    "#;
    run_benchmark(c, "string_interpolation", input);
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
    run_benchmark(c, "large_file", &input);
}

fn bench_whitespace_heavy(c: &mut Criterion) {
    let input = r#"
    # This is a comment
    my   $x   =   42  ;  # Another comment

    print    $x    ;

    # More comments
    "#;
    run_benchmark(c, "whitespace_heavy", input);
}

fn bench_operator_heavy(c: &mut Criterion) {
    let input = "$a += $b -= $c *= $d /= $e %= $f **= $g &&= $h ||= $i //= $j";
    run_benchmark(c, "operator_heavy", input);
}

fn bench_number_parsing(c: &mut Criterion) {
    let input = "123 456.789 1_234_567 1.23e45 0xFF 0377 0b1010";
    run_benchmark(c, "number_parsing", input);
}

fn bench_keyword_heavy(c: &mut Criterion) {
    let base =
        "if else while until for foreach return last next redo package require default continue";
    let input = base.repeat(100);
    run_benchmark(c, "keyword_heavy", &input);
}

fn benchmark_token_counts() -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();

    counts.insert(
        "simple_tokens".to_string(),
        collect_all_tokens(PerlLexer::new("my $x = 42; print $x;")).len() as u64,
    );

    let slash_input = r#"
        my $x = 10 / 2;
        if ($str =~ /pattern/) {
            $str =~ s/foo/bar/g;
        }
        print 1/ /abc/;
    "#;
    counts.insert(
        "slash_disambiguation".to_string(),
        collect_all_tokens(PerlLexer::new(slash_input)).len() as u64,
    );

    let interpolation_input = r#"
        my $name = "World";
        print "Hello, $name!\n";
        print "The answer is ${count + 1}\n";
        print "Array: @items\n";
    "#;
    counts.insert(
        "string_interpolation".to_string(),
        collect_all_tokens(PerlLexer::new(interpolation_input)).len() as u64,
    );

    let mut large_file_input = String::new();
    for i in 0..1000 {
        large_file_input.push_str(&format!("my $var{} = {};\n", i, i));
        large_file_input.push_str(&format!("print \"Value: $var{}\n\";\n", i));
        if i % 10 == 0 {
            large_file_input.push_str(&format!("if ($var{} =~ /\\d+/) {{\n", i));
            large_file_input.push_str(&format!("    $var{} = $var{} / 2;\n", i, i));
            large_file_input.push_str("}\n");
        }
    }
    counts.insert(
        "large_file".to_string(),
        collect_all_tokens(PerlLexer::new(&large_file_input)).len() as u64,
    );

    let whitespace_input = r#"
    # This is a comment
    my   $x   =   42  ;  # Another comment

    print    $x    ;

    # More comments
    "#;
    counts.insert(
        "whitespace_heavy".to_string(),
        collect_all_tokens(PerlLexer::new(whitespace_input)).len() as u64,
    );

    counts.insert(
        "operator_heavy".to_string(),
        collect_all_tokens(PerlLexer::new(
            "$a += $b -= $c *= $d /= $e %= $f **= $g &&= $h ||= $i //= $j",
        ))
        .len() as u64,
    );

    counts.insert(
        "number_parsing".to_string(),
        collect_all_tokens(PerlLexer::new("123 456.789 1_234_567 1.23e45 0xFF 0377 0b1010")).len()
            as u64,
    );

    let base =
        "if else while until for foreach return last next redo package require default continue";
    let keyword_heavy = base.repeat(100);
    counts.insert(
        "keyword_heavy".to_string(),
        collect_all_tokens(PerlLexer::new(&keyword_heavy)).len() as u64,
    );

    counts
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn extract_ci_bound_ns(estimates: &Value, key: &str, fallback: u64) -> u64 {
    estimates
        .get("mean")
        .and_then(|mean| mean.get("confidence_interval"))
        .and_then(|interval| interval.get(key))
        .and_then(Value::as_f64)
        .map(|value| value.max(0.0) as u64)
        .unwrap_or(fallback)
}

fn extract_mean_ns(estimates: &Value) -> Option<u64> {
    estimates
        .get("mean")
        .and_then(|mean| mean.get("point_estimate"))
        .and_then(Value::as_f64)
        .map(|value| value.max(0.0) as u64)
}

fn sample_count(sample: &Value) -> Option<u64> {
    sample
        .get("iters")
        .and_then(Value::as_array)
        .map(|iters| iters.len() as u64)
        .or_else(|| sample.get("times").and_then(Value::as_array).map(|times| times.len() as u64))
}

fn benchmark_dir_for(name: &str) -> PathBuf {
    criterion_root().join("lexer").join(name).join("input").join("new")
}

fn criterion_root() -> PathBuf {
    let local_target = PathBuf::from("target").join("criterion");
    let workspace_target = PathBuf::from("..").join("..").join("target").join("criterion");
    let env_target = env::var("CARGO_TARGET_DIR")
        .map_or_else(|_| local_target.clone(), |dir| PathBuf::from(dir).join("criterion"));

    let candidates = [local_target.clone(), workspace_target, env_target];
    for candidate in candidates {
        let probe = candidate
            .join("lexer")
            .join("simple_tokens")
            .join("input")
            .join("new")
            .join("estimates.json");
        if probe.exists() {
            return candidate;
        }
    }

    local_target
}

fn emit_lexer_scorecard() {
    let token_counts = benchmark_token_counts();
    let mut benches = serde_json::Map::new();

    for benchmark_name in BENCHMARKS {
        let dir = benchmark_dir_for(benchmark_name);
        let estimates_path = dir.join("estimates.json");
        let sample_path = dir.join("sample.json");

        let Some(estimates) = read_json(&estimates_path) else {
            continue;
        };
        let Some(mean_ns) = extract_mean_ns(&estimates) else {
            continue;
        };

        let low_ns = extract_ci_bound_ns(&estimates, "lower_bound", mean_ns);
        let high_ns = extract_ci_bound_ns(&estimates, "upper_bound", mean_ns);
        let samples = read_json(&sample_path).as_ref().and_then(sample_count).unwrap_or(0);
        let tokens = *token_counts.get(*benchmark_name).unwrap_or(&0);
        let tokens_per_second =
            if mean_ns == 0 { 0.0 } else { (tokens as f64 * 1_000_000_000.0) / mean_ns as f64 };

        benches.insert(
            benchmark_name.to_string(),
            serde_json::json!({
                "tokens": tokens,
                "mean_ns": mean_ns,
                "low_ns": low_ns,
                "high_ns": high_ns,
                "sample_count": samples,
                "total_time_ns": mean_ns.saturating_mul(samples),
                "tokens_per_second": tokens_per_second,
            }),
        );
    }

    let payload = serde_json::json!({
        "schema_version": 1,
        "benchmark_group": "lexer",
        "artifact": "lexer_scorecard",
        "benchmarks": benches,
    });

    let output_path = criterion_root().join("lexer_scorecard.json");
    if let Some(parent) = output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(serialized) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(output_path, serialized);
    }
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    bench_simple_tokens(&mut criterion);
    bench_slash_disambiguation(&mut criterion);
    bench_string_interpolation(&mut criterion);
    bench_large_file(&mut criterion);
    bench_whitespace_heavy(&mut criterion);
    bench_operator_heavy(&mut criterion);
    bench_number_parsing(&mut criterion);
    bench_keyword_heavy(&mut criterion);

    criterion.final_summary();
    emit_lexer_scorecard();
}
