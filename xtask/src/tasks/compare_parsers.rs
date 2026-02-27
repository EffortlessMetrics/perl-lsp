//! Three-way parser comparison tool
//!
//! Compares Pure Rust, Legacy C, and Modern parser implementations

use color_eyre::eyre::{Result, bail};
use color_eyre::owo_colors::OwoColorize;
use perl_parser::Parser as ModernParser;
use serde_json;
use std::time::{Duration, Instant};
use tree_sitter_perl::PureRustPerlParser;

pub fn run_three_way(verbose: bool, format: &str) -> Result<()> {
    match format {
        "table" => run_table_format(verbose),
        "json" => run_json_format(verbose),
        "markdown" => run_markdown_format(verbose),
        _ => {
            eprintln!("Invalid format: {}. Using table format.", format);
            run_table_format(verbose)
        }
    }
}

fn run_table_format(_verbose: bool) -> Result<()> {
    println!("\n{}", "=== Three-Way Parser Comparison ===".bright_blue().bold());
    println!("{}", "Comparing: Pure Rust vs Legacy C vs Modern Parser".yellow());

    // Test cases
    let test_cases = vec![
        ("Simple", "$x = 42;"),
        ("Expression", "my $result = ($a + $b) * $c;"),
        (
            "Control Flow",
            r#"
if ($x > 10) {
    while ($y < 100) {
        $y = $y * 2;
    }
}"#,
        ),
        ("Method Call", "$obj->method($arg1, $arg2);"),
        ("For Loop", "for (my $i = 0; $i < 10; $i++) { print $i; }"),
        (
            "Complex",
            r#"
package Test;
use strict;
sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}
1;"#,
        ),
    ];

    // Warm up
    println!("\n{}", "Warming up parsers...".dimmed());
    for (_, code) in &test_cases {
        let _ = bench_pure_rust(code);
        let _ = bench_legacy_c(code);
        let _ = bench_modern(code);
    }

    // Run benchmarks
    println!("\n{}", "Running benchmarks...".green());
    let mut results = Vec::new();

    for (name, code) in &test_cases {
        println!("\n{} {}", "Testing:".cyan(), name.bold());
        println!("{}", format!("Code size: {} bytes", code.len()).dimmed());

        let pure_rust_time = bench_pure_rust(code)?;
        let legacy_c_time = bench_legacy_c(code)?;
        let modern_time = bench_modern(code)?;

        results.push((name, pure_rust_time, legacy_c_time, modern_time));

        // Display results
        println!(
            "  {} {:>8.2} µs",
            "Pure Rust:".bright_yellow(),
            pure_rust_time.as_secs_f64() * 1_000_000.0
        );
        println!(
            "  {} {:>8.2} µs",
            "Legacy C: ".bright_cyan(),
            legacy_c_time.as_secs_f64() * 1_000_000.0
        );
        println!(
            "  {} {:>8.2} µs",
            "Modern:   ".bright_green(),
            modern_time.as_secs_f64() * 1_000_000.0
        );

        // Calculate relative performance
        let c_vs_rust = pure_rust_time.as_secs_f64() / legacy_c_time.as_secs_f64();
        let modern_vs_rust = pure_rust_time.as_secs_f64() / modern_time.as_secs_f64();
        let modern_vs_c = modern_time.as_secs_f64() / legacy_c_time.as_secs_f64();

        println!("\n  {}", "Relative Performance:".underline());
        println!(
            "  - Legacy C is {:.1}x {} than Pure Rust",
            c_vs_rust,
            if c_vs_rust > 1.0 { "faster" } else { "slower" }
        );
        println!(
            "  - Modern is {:.1}x {} than Pure Rust",
            modern_vs_rust,
            if modern_vs_rust > 1.0 { "faster" } else { "slower" }
        );
        println!(
            "  - Modern is {:.1}x {} than Legacy C",
            modern_vs_c,
            if modern_vs_c < 1.0 { "faster" } else { "slower" }
        );
    }

    // Summary
    println!("\n{}", "=== Summary ===".bright_blue().bold());

    let avg_pure_rust: f64 =
        results.iter().map(|(_, pr, _, _)| pr.as_secs_f64()).sum::<f64>() / results.len() as f64;
    let avg_legacy_c: f64 =
        results.iter().map(|(_, _, lc, _)| lc.as_secs_f64()).sum::<f64>() / results.len() as f64;
    let avg_modern: f64 =
        results.iter().map(|(_, _, _, m)| m.as_secs_f64()).sum::<f64>() / results.len() as f64;

    println!("\n{}", "Average Parse Times:".underline());
    println!("  {} {:>8.2} µs", "Pure Rust:".bright_yellow(), avg_pure_rust * 1_000_000.0);
    println!("  {} {:>8.2} µs", "Legacy C: ".bright_cyan(), avg_legacy_c * 1_000_000.0);
    println!("  {} {:>8.2} µs", "Modern:   ".bright_green(), avg_modern * 1_000_000.0);

    println!("\n{}", "Performance Characteristics:".underline());
    println!("  - Pure Rust: {} with rich error messages", "Feature-complete".green());
    println!("  - Legacy C:  {} but limited features", "Fastest".cyan());
    println!("  - Modern:    {} between speed and features", "Best balance".bright_green());

    Ok(())
}

fn bench_pure_rust(code: &str) -> Result<Duration> {
    const ITERATIONS: u32 = 100;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let mut parser = PureRustPerlParser::new();
        let result = parser.parse(code);
        if result.is_err() {
            bail!("Pure Rust parser failed");
        }
    }

    Ok(start.elapsed() / ITERATIONS)
}

fn bench_legacy_c(code: &str) -> Result<Duration> {
    use tree_sitter::Parser;

    const ITERATIONS: u32 = 100;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_perl::language())?;

        let tree = parser.parse(code, None);
        if tree.is_none() {
            bail!("Legacy C parser failed");
        }
    }

    Ok(start.elapsed() / ITERATIONS)
}

fn bench_modern(code: &str) -> Result<Duration> {
    const ITERATIONS: u32 = 100;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let mut parser = ModernParser::new(code);
        let result = parser.parse();
        if result.is_err() {
            bail!("Modern parser failed");
        }
    }

    Ok(start.elapsed() / ITERATIONS)
}

fn run_json_format(verbose: bool) -> Result<()> {
    let test_cases = get_test_cases();
    let mut results = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_cases": []
    });

    for (name, code) in &test_cases {
        if verbose {
            eprintln!("Testing: {}", name);
        }

        let pure_rust_time = bench_pure_rust(code)?;
        let legacy_c_time = bench_legacy_c(code)?;
        let modern_time = bench_modern(code)?;

        if let Some(arr) = results["test_cases"].as_array_mut() {
            arr.push(serde_json::json!({
                "name": name,
                "code_size": code.len(),
                "pure_rust_µs": pure_rust_time.as_secs_f64() * 1_000_000.0,
                "legacy_c_µs": legacy_c_time.as_secs_f64() * 1_000_000.0,
                "modern_µs": modern_time.as_secs_f64() * 1_000_000.0,
            }));
        } else {
            bail!("Failed to access test_cases array in results");
        }
    }

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

fn run_markdown_format(verbose: bool) -> Result<()> {
    println!("# Three-Way Parser Comparison Report\n");
    println!("**Generated:** {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

    let test_cases = get_test_cases();
    let mut all_results = Vec::new();

    println!("## Test Results\n");
    println!("| Test Case | Code Size | Pure Rust (µs) | Legacy C (µs) | Modern (µs) | Fastest |");
    println!("|-----------|-----------|----------------|---------------|-------------|---------|");

    for (name, code) in &test_cases {
        if verbose {
            eprintln!("Testing: {}", name);
        }

        let pure_rust_time = bench_pure_rust(code)?;
        let legacy_c_time = bench_legacy_c(code)?;
        let modern_time = bench_modern(code)?;

        let pr_us = pure_rust_time.as_secs_f64() * 1_000_000.0;
        let lc_us = legacy_c_time.as_secs_f64() * 1_000_000.0;
        let m_us = modern_time.as_secs_f64() * 1_000_000.0;

        let fastest = if lc_us <= pr_us && lc_us <= m_us {
            "Legacy C"
        } else if m_us <= pr_us && m_us <= lc_us {
            "Modern"
        } else {
            "Pure Rust"
        };

        println!(
            "| {} | {} | {:.2} | {:.2} | {:.2} | {} |",
            name,
            code.len(),
            pr_us,
            lc_us,
            m_us,
            fastest
        );

        all_results.push((name, pr_us, lc_us, m_us));
    }

    // Summary
    let avg_pure_rust: f64 =
        all_results.iter().map(|(_, pr, _, _)| pr).sum::<f64>() / all_results.len() as f64;
    let avg_legacy_c: f64 =
        all_results.iter().map(|(_, _, lc, _)| lc).sum::<f64>() / all_results.len() as f64;
    let avg_modern: f64 =
        all_results.iter().map(|(_, _, _, m)| m).sum::<f64>() / all_results.len() as f64;

    println!("\n## Summary\n");
    println!("| Parser | Average Time (µs) | Relative to C |");
    println!("|--------|-------------------|---------------|");
    println!("| Pure Rust | {:.2} | {:.1}x |", avg_pure_rust, avg_pure_rust / avg_legacy_c);
    println!("| Legacy C | {:.2} | 1.0x |", avg_legacy_c);
    println!("| Modern | {:.2} | {:.1}x |", avg_modern, avg_modern / avg_legacy_c);

    println!("\n## Analysis\n");
    println!("- **Pure Rust**: Full-featured parser with comprehensive error handling");
    println!("- **Legacy C**: Fastest but limited feature set");
    println!("- **Modern**: Balanced performance with clean architecture");

    Ok(())
}

fn get_test_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Simple", "$x = 42;"),
        ("Expression", "my $result = ($a + $b) * $c;"),
        (
            "Control Flow",
            r#"
if ($x > 10) {
    while ($y < 100) {
        $y = $y * 2;
    }
}"#,
        ),
        ("Method Call", "$obj->method($arg1, $arg2);"),
        ("For Loop", "for (my $i = 0; $i < 10; $i++) { print $i; }"),
        (
            "Complex",
            r#"
package Test;
use strict;
sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}
1;"#,
        ),
    ]
}
