use criterion::Criterion;
use perl_lexer::{PerlLexer, Token};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn collect_all_tokens(mut lexer: PerlLexer) -> Vec<Token> {
    lexer.collect_tokens()
}

fn simple_tokens_input() -> String {
    "my $x = 42; print $x;".to_string()
}

fn slash_disambiguation_input() -> String {
    r#"
        my $x = 10 / 2;
        if ($str =~ /pattern/) {
            $str =~ s/foo/bar/g;
        }
        print 1/ /abc/;
    "#
    .to_string()
}

fn string_interpolation_input() -> String {
    r#"
        my $name = "World";
        print "Hello, $name!\n";
        print "The answer is ${count + 1}\n";
        print "Array: @items\n";
    "#
    .to_string()
}

fn large_file_input() -> String {
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
    input
}

fn whitespace_heavy_input() -> String {
    r#"
    # This is a comment
    my   $x   =   42  ;  # Another comment

    print    $x    ;

    # More comments
    "#
    .to_string()
}

fn operator_heavy_input() -> String {
    "$a += $b -= $c *= $d /= $e %= $f **= $g &&= $h ||= $i //= $j".to_string()
}

fn number_parsing_input() -> String {
    "123 456.789 1_234_567 1.23e45 0xFF 0377 0b1010".to_string()
}

fn keyword_heavy_input() -> String {
    "if else while until for foreach return last next redo package require default continue"
        .repeat(100)
}

#[derive(Debug, Serialize)]
struct BenchScorecardEntry {
    total_time_ns: f64,
    tokens_per_second: f64,
    sample_count: u64,
    mean_ns: f64,
    lower_bound_ns: f64,
    upper_bound_ns: f64,
}

#[derive(Debug, Serialize)]
struct BenchScorecard {
    schema_version: u8,
    generated_at_unix_seconds: u64,
    source: &'static str,
    benches: BTreeMap<String, BenchScorecardEntry>,
}

struct BenchFixture {
    name: &'static str,
    input: String,
    token_count: u64,
}

fn build_fixture(name: &'static str, input: String) -> BenchFixture {
    let token_count = collect_all_tokens(PerlLexer::new(input.as_str())).len() as u64;
    BenchFixture { name, input, token_count }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn criterion_estimates_path(bench_name: &str) -> PathBuf {
    workspace_root()
        .join("target")
        .join("criterion")
        .join(bench_name)
        .join("new")
        .join("estimates.json")
}

fn criterion_sample_path(bench_name: &str) -> PathBuf {
    workspace_root()
        .join("target")
        .join("criterion")
        .join(bench_name)
        .join("new")
        .join("sample.json")
}

fn read_estimates(path: &Path) -> Option<(f64, f64, f64)> {
    let raw = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&raw).ok()?;
    let mean = json.get("mean")?;
    let mean_ns = mean.get("point_estimate")?.as_f64()?;
    let ci = mean.get("confidence_interval")?;
    let lower = ci.get("lower_bound")?.as_f64()?;
    let upper = ci.get("upper_bound")?.as_f64()?;
    Some((mean_ns, lower, upper))
}

fn read_sample_count(path: &Path) -> Option<u64> {
    let raw = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&raw).ok()?;
    json.get("sampling_mode")
        .and_then(|_| json.get("iters"))
        .and_then(Value::as_array)
        .map(|iters| iters.len() as u64)
}

fn emit_scorecard(fixtures: &[BenchFixture]) {
    let mut benches = BTreeMap::new();
    for fixture in fixtures {
        let estimates_path = criterion_estimates_path(fixture.name);
        let Some((mean_ns, lower_bound_ns, upper_bound_ns)) = read_estimates(&estimates_path)
        else {
            continue;
        };
        let sample_count = read_sample_count(&criterion_sample_path(fixture.name)).unwrap_or(0);
        let total_time_ns = mean_ns;
        let tokens_per_second = if mean_ns > 0.0 {
            (fixture.token_count as f64) * 1_000_000_000.0 / mean_ns
        } else {
            0.0
        };

        benches.insert(
            fixture.name.to_string(),
            BenchScorecardEntry {
                total_time_ns,
                tokens_per_second,
                sample_count,
                mean_ns,
                lower_bound_ns,
                upper_bound_ns,
            },
        );
    }

    let generated_at_unix_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let scorecard = BenchScorecard {
        schema_version: 1,
        generated_at_unix_seconds,
        source: "criterion",
        benches,
    };

    let out_path = workspace_root().join("target/criterion/lexer_scorecard.json");
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&scorecard) {
        let _ = fs::write(&out_path, serialized);
    }
}

fn run_all_benchmarks(c: &mut Criterion, fixtures: &[BenchFixture]) {
    for fixture in fixtures {
        c.bench_function(fixture.name, |b| {
            b.iter(|| {
                let lexer = PerlLexer::new(black_box(fixture.input.as_str()));
                collect_all_tokens(lexer)
            });
        });
    }
}

fn main() {
    let fixtures = vec![
        build_fixture("simple_tokens", simple_tokens_input()),
        build_fixture("slash_disambiguation", slash_disambiguation_input()),
        build_fixture("string_interpolation", string_interpolation_input()),
        build_fixture("large_file", large_file_input()),
        build_fixture("whitespace_heavy", whitespace_heavy_input()),
        build_fixture("operator_heavy", operator_heavy_input()),
        build_fixture("number_parsing", number_parsing_input()),
        build_fixture("keyword_heavy", keyword_heavy_input()),
    ];

    let started = Instant::now();
    let mut criterion = Criterion::default().configure_from_args();
    run_all_benchmarks(&mut criterion, &fixtures);
    criterion.final_summary();
    emit_scorecard(&fixtures);
    let _elapsed = started.elapsed();
}
