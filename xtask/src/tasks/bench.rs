//! Benchmark task implementation
//!
//! This module provides comprehensive benchmarking capabilities to compare
//! the legacy C implementation with the modern Rust implementation.

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub implementation: String,
    pub duration: Duration,
    pub memory_usage: Option<u64>,
    pub iterations: u64,
}

#[derive(Debug, Clone)]
pub struct BenchmarkComparison {
    pub name: String,
    pub rust_result: BenchmarkResult,
    pub c_result: BenchmarkResult,
    pub speedup: f64,
    pub memory_improvement: Option<f64>,
}

pub fn run(name: Option<String>, save: bool) -> Result<()> {
    let multi_progress = MultiProgress::new();
    let main_spinner = multi_progress.add(ProgressBar::new_spinner());
    main_spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    main_spinner.set_message("Setting up benchmark environment");

    // Ensure both implementations are built
    build_implementations(&multi_progress)?;

    // Run comprehensive benchmarks
    let all_results = run_comprehensive_benchmarks(&multi_progress, name.as_deref())?;

    main_spinner.finish_with_message("✅ All benchmarks completed");

    // Generate comparison report
    let comparisons = generate_comparisons(&all_results);
    display_results(&comparisons);

    if save {
        save_results(&all_results, &comparisons)?;
    }

    Ok(())
}

fn build_implementations(multi_progress: &MultiProgress) -> Result<()> {
    let rust_spinner = multi_progress.add(ProgressBar::new_spinner());
    rust_spinner.set_message("Building Rust implementation");
    
    let status = cmd("cargo", &["build", "--release", "--package", "tree-sitter-perl"])
        .dir("crates/tree-sitter-perl")
        .run()
        .context("Failed to build Rust implementation")?;

    if !status.status.success() {
        return Err(color_eyre::eyre::eyre!("Rust build failed"));
    }
    rust_spinner.finish_with_message("✅ Rust implementation built");

    let c_spinner = multi_progress.add(ProgressBar::new_spinner());
    c_spinner.set_message("Building C implementation");
    
    let status = cmd("make", &["clean", "release"])
        .dir("tree-sitter-perl")
        .run()
        .context("Failed to build C implementation")?;

    if !status.status.success() {
        return Err(color_eyre::eyre::eyre!("C build failed"));
    }
    c_spinner.finish_with_message("✅ C implementation built");

    Ok(())
}

fn run_comprehensive_benchmarks(multi_progress: &MultiProgress, name_filter: Option<&str>) -> Result<Vec<BenchmarkResult>> {
    let mut all_results = Vec::new();
    
    // Run Rust benchmarks using criterion
    let rust_spinner = multi_progress.add(ProgressBar::new_spinner());
    rust_spinner.set_message("Running Rust benchmarks");
    
    let rust_results = run_rust_benchmarks(name_filter)?;
    all_results.extend(rust_results);
    rust_spinner.finish_with_message("✅ Rust benchmarks completed");
    
    // Run C benchmarks using Node.js
    let c_spinner = multi_progress.add(ProgressBar::new_spinner());
    c_spinner.set_message("Running C benchmarks");
    
    let c_results = run_c_benchmarks(name_filter)?;
    all_results.extend(c_results);
    c_spinner.finish_with_message("✅ C benchmarks completed");
    
    Ok(all_results)
}

fn run_rust_benchmarks(name_filter: Option<&str>) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    // Run scanner benchmarks
    if name_filter.is_none() || name_filter.unwrap().contains("scanner") {
        let scanner_results = run_rust_scanner_benchmarks()?;
        results.extend(scanner_results);
    }
    
    // Run parser benchmarks
    if name_filter.is_none() || name_filter.unwrap().contains("parser") {
        let parser_results = run_rust_parser_benchmarks()?;
        results.extend(parser_results);
    }
    
    Ok(results)
}

fn run_rust_scanner_benchmarks() -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    // Define test cases for scanner benchmarks
    let test_cases = vec![
        ("simple_variable", "my $var = 42;"),
        ("simple_print", "print 'Hello, World!';"),
        ("simple_sub", "sub foo { return 1; }"),
        ("simple_conditional", "if ($x) { $y = 1; }"),
        ("simple_loop", "for my $i (1..10) { print $i; }"),
    ];
    
    for (name, code) in test_cases {
        let result = benchmark_rust_implementation(name, code, 1000)?;
        results.push(result);
    }
    
    Ok(results)
}

fn run_rust_parser_benchmarks() -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    // Define test cases for parser benchmarks
    let test_cases = vec![
        ("complex_class", r#"
package MyClass;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    bless \%args, $class;
}

sub method {
    my ($self, @params) = @_;
    return $self->{value} + @params;
}
"#),
        ("unicode_code", r#"
my $変数 = "値";
my $über = "cool";
my $naïve = "simple";
sub 関数 { return "関数です"; }
"#),
        ("large_file", &generate_large_perl_file(1000)),
    ];
    
    for (name, code) in test_cases {
        let result = benchmark_rust_implementation(name, code, 100)?;
        results.push(result);
    }
    
    Ok(results)
}

fn run_c_benchmarks(name_filter: Option<&str>) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    // Define test cases for C benchmarks (same as Rust for fair comparison)
    let test_cases = vec![
        ("simple_variable", "my $var = 42;"),
        ("simple_print", "print 'Hello, World!';"),
        ("simple_sub", "sub foo { return 1; }"),
        ("simple_conditional", "if ($x) { $y = 1; }"),
        ("simple_loop", "for my $i (1..10) { print $i; }"),
        ("complex_class", r#"
package MyClass;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    bless \%args, $class;
}

sub method {
    my ($self, @params) = @_;
    return $self->{value} + @params;
}
"#),
        ("unicode_code", r#"
my $変数 = "値";
my $über = "cool";
my $naïve = "simple";
sub 関数 { return "関数です"; }
"#),
        ("large_file", &generate_large_perl_file(1000)),
    ];
    
    for (name, code) in test_cases {
        if let Some(filter) = name_filter {
            if !name.contains(filter) {
                continue;
            }
        }
        
        let result = benchmark_c_implementation(name, code, 100)?;
        results.push(result);
    }
    
    Ok(results)
}

fn benchmark_rust_implementation(name: &str, code: &str, iterations: u64) -> Result<BenchmarkResult> {
    let start = Instant::now();
    
    // Run the benchmark using the Rust implementation
    for _ in 0..iterations {
        let status = cmd("cargo", &["run", "--release", "--bin", "bench_parser"])
            .dir("crates/tree-sitter-perl")
            .env("TEST_CODE", code)
            .run()
            .context("Failed to run Rust benchmark")?;
            
        if !status.status.success() {
            return Err(color_eyre::eyre::eyre!("Rust benchmark failed for {}", name));
        }
    }
    
    let duration = start.elapsed();
    
    Ok(BenchmarkResult {
        name: name.to_string(),
        implementation: "rust".to_string(),
        duration,
        memory_usage: None, // TODO: Add memory measurement
        iterations,
    })
}

fn benchmark_c_implementation(name: &str, code: &str, iterations: u64) -> Result<BenchmarkResult> {
    let start = Instant::now();
    
    // Run the benchmark using the C implementation
    for _ in 0..iterations {
        let status = cmd("node", &["test/benchmark.js"])
            .dir("tree-sitter-perl")
            .env("TEST_CODE", code)
            .run()
            .context("Failed to run C benchmark")?;
            
        if !status.status.success() {
            return Err(color_eyre::eyre::eyre!("C benchmark failed for {}", name));
        }
    }
    
    let duration = start.elapsed();
    
    Ok(BenchmarkResult {
        name: name.to_string(),
        implementation: "c".to_string(),
        duration,
        memory_usage: None, // TODO: Add memory measurement
        iterations,
    })
}

fn generate_large_perl_file(size: usize) -> String {
    let mut code = String::new();

    // Add package declaration
    code.push_str("package LargeFile;\n");
    code.push_str("use strict;\n");
    code.push_str("use warnings;\n\n");

    // Add variables
    for i in 0..size {
        code.push_str(&format!("my $var{} = {};\n", i, i));
    }

    code.push_str("\n");

    // Add functions
    for i in 0..(size / 10) {
        code.push_str(&format!("sub func{} {{\n", i));
        code.push_str(&format!("    my ($param) = @_;\n"));
        code.push_str(&format!("    return $param + {};\n", i));
        code.push_str("}\n\n");
    }

    // Add main logic
    code.push_str("sub main {\n");
    for i in 0..(size / 20) {
        code.push_str(&format!("    print \"Processing variable {}\";\n", i));
        code.push_str(&format!("    my $result = func{}($var{});\n", i, i));
        code.push_str(&format!("    print \"Result: $result\";\n"));
    }
    code.push_str("}\n\n");

    code.push_str("main();\n");

    code
}

fn generate_comparisons(results: &[BenchmarkResult]) -> Vec<BenchmarkComparison> {
    let mut comparisons = Vec::new();
    let mut grouped: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
    
    // Group results by test name
    for result in results {
        grouped.entry(result.name.clone()).or_default().push(result);
    }
    
    for (name, test_results) in grouped {
        if test_results.len() != 2 {
            continue; // Skip if we don't have both implementations
        }
        
        let rust_result = test_results.iter().find(|r| r.implementation == "rust").unwrap();
        let c_result = test_results.iter().find(|r| r.implementation == "c").unwrap();
        
        let speedup = c_result.duration.as_nanos() as f64 / rust_result.duration.as_nanos() as f64;
        
        comparisons.push(BenchmarkComparison {
            name,
            rust_result: (*rust_result).clone(),
            c_result: (*c_result).clone(),
            speedup,
            memory_improvement: None, // TODO: Add memory comparison
        });
    }
    
    comparisons
}

fn display_results(comparisons: &[BenchmarkComparison]) {
    println!("\n📊 Benchmark Results\n");
    println!("{:<30} {:<15} {:<15} {:<15} {:<15}", "Test", "Rust (ms)", "C (ms)", "Speedup", "Winner");
    println!("{:-<90}", "");
    
    for comparison in comparisons {
        let rust_ms = comparison.rust_result.duration.as_millis();
        let c_ms = comparison.c_result.duration.as_millis();
        let winner = if comparison.speedup > 1.0 { "Rust" } else { "C" };
        
        println!(
            "{:<30} {:<15} {:<15} {:<15.2}x {:<15}",
            comparison.name,
            rust_ms,
            c_ms,
            comparison.speedup,
            winner
        );
    }
    
    println!("\n📈 Summary");
    let rust_wins = comparisons.iter().filter(|c| c.speedup > 1.0).count();
    let total = comparisons.len();
    let avg_speedup: f64 = comparisons.iter().map(|c| c.speedup).sum::<f64>() / total as f64;
    
    println!("Rust wins: {}/{} tests ({:.1}%)", rust_wins, total, (rust_wins as f64 / total as f64) * 100.0);
    println!("Average speedup: {:.2}x", avg_speedup);
    
    if rust_wins > total / 2 {
        println!("🎉 Rust implementation shows better performance overall!");
    } else {
        println!("⚠️  C implementation shows better performance overall");
    }
}

fn save_results(results: &[BenchmarkResult], comparisons: &[BenchmarkComparison]) -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("benchmark_results_{}.json", timestamp);
    
    let output = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "results": results,
        "comparisons": comparisons,
    });
    
    std::fs::write(&filename, serde_json::to_string_pretty(&output)?)
        .context("Failed to save benchmark results")?;
    
    println!("💾 Results saved to {}", filename);
    
    Ok(())
}
