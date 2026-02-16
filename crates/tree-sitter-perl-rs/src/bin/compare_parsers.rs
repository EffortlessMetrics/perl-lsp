//! Compare C/tree-sitter parser with pure Rust parser

use std::env;
use std::fs;
use std::time::Duration;
use tree_sitter_perl::comparison_harness::ComparisonHarness;

fn format_duration(d: Duration) -> String {
    if d.as_secs() > 0 {
        format!("{:.3}s", d.as_secs_f64())
    } else if d.as_millis() > 0 {
        format!("{:.3}ms", d.as_secs_f64() * 1000.0)
    } else {
        format!("{:.3}µs", d.as_secs_f64() * 1_000_000.0)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <perl_file> [iterations]", args[0]);
        eprintln!("       {} --test", args[0]);
        std::process::exit(1);
    }

    let mut harness = ComparisonHarness::new();

    if args[1] == "--test" {
        // Run basic tests
        println!("Running basic comparison tests...\n");

        let test_cases = vec![
            ("Basic variable", "$var"),
            ("Assignment", "$var = 42;"),
            ("Function declaration", "sub hello { print 'Hello'; }"),
            ("If statement", "if ($x > 0) { print 'positive'; }"),
            ("Array and hash", "@array = (1, 2, 3); %hash = (a => 1);"),
        ];

        for (name, source) in test_cases {
            println!("Test: {}", name);
            println!("Source: {}", source);

            let (tree_sitter_result, pure_rust_result) = harness.compare_parsers(source);

            println!("Tree-sitter:");
            if tree_sitter_result.success {
                println!("  ✓ Success ({})", format_duration(tree_sitter_result.parse_time));
                println!("  S-expr: {}", tree_sitter_result.s_expression);
            } else {
                println!("  ✗ Failed: {}", tree_sitter_result.error.unwrap_or_default());
            }

            println!("Pure Rust:");
            if pure_rust_result.success {
                println!("  ✓ Success ({})", format_duration(pure_rust_result.parse_time));
                println!("  S-expr: {}", pure_rust_result.s_expression);
            } else {
                println!("  ✗ Failed: {}", pure_rust_result.error.unwrap_or_default());
            }

            println!();
        }
    } else {
        // Parse a file
        let filename = &args[1];
        let iterations = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

        let source = match fs::read_to_string(filename) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file {}: {}", filename, e);
                std::process::exit(1);
            }
        };

        println!("Parsing file: {} ({} bytes)", filename, source.len());
        println!("Iterations: {}\n", iterations);

        if iterations == 1 {
            // Single parse with detailed results
            let (tree_sitter_result, pure_rust_result) = harness.compare_parsers(&source);

            println!("Tree-sitter parser:");
            if tree_sitter_result.success {
                println!("  ✓ Success");
                println!("  Parse time: {}", format_duration(tree_sitter_result.parse_time));
                if source.len() < 1000 {
                    println!("  S-expression: {}", tree_sitter_result.s_expression);
                }
            } else {
                println!("  ✗ Failed: {}", tree_sitter_result.error.unwrap_or_default());
            }

            println!("\nPure Rust parser:");
            if pure_rust_result.success {
                println!("  ✓ Success");
                println!("  Parse time: {}", format_duration(pure_rust_result.parse_time));
                if source.len() < 1000 {
                    println!("  S-expression: {}", pure_rust_result.s_expression);
                }
            } else {
                println!("  ✗ Failed: {}", pure_rust_result.error.unwrap_or_default());
            }

            // Compare results
            if tree_sitter_result.success && pure_rust_result.success {
                println!("\nComparison:");
                if tree_sitter_result.s_expression == pure_rust_result.s_expression {
                    println!("  ✓ S-expressions match");
                } else {
                    println!("  ✗ S-expressions differ");
                }

                let speedup = tree_sitter_result.parse_time.as_secs_f64()
                    / pure_rust_result.parse_time.as_secs_f64();
                if speedup > 1.0 {
                    println!("  Pure Rust is {:.2}x faster", speedup);
                } else {
                    println!("  Tree-sitter is {:.2}x faster", 1.0 / speedup);
                }
            }
        } else {
            // Benchmark mode
            println!("Running benchmark...");
            let results = harness.run_benchmark(&source, iterations);

            for (parser, times) in &results {
                if !times.is_empty() {
                    let total: Duration = times.iter().sum();
                    let avg = total / times.len() as u32;
                    let min = times.iter().min().cloned().unwrap_or(Duration::from_secs(0));
                    let max = times.iter().max().cloned().unwrap_or(Duration::from_secs(0));

                    println!("\n{} parser:", parser);
                    println!("  Successful parses: {}/{}", times.len(), iterations);
                    println!("  Average time: {}", format_duration(avg));
                    println!("  Min time: {}", format_duration(*min));
                    println!("  Max time: {}", format_duration(*max));
                } else {
                    println!("\n{} parser: No successful parses", parser);
                }
            }

            // Compare average times
            if let (Some(ts_times), Some(pr_times)) =
                (results.get("tree-sitter"), results.get("pure-rust"))
            {
                if !ts_times.is_empty() && !pr_times.is_empty() {
                    let ts_avg: Duration =
                        ts_times.iter().sum::<Duration>() / ts_times.len() as u32;
                    let pr_avg: Duration =
                        pr_times.iter().sum::<Duration>() / pr_times.len() as u32;

                    println!("\nPerformance comparison:");
                    let speedup = ts_avg.as_secs_f64() / pr_avg.as_secs_f64();
                    if speedup > 1.0 {
                        println!("  Pure Rust is {:.2}x faster on average", speedup);
                    } else {
                        println!("  Tree-sitter is {:.2}x faster on average", 1.0 / speedup);
                    }
                }
            }
        }
    }
}
