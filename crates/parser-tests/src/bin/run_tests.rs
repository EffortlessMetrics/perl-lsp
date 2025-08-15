//! Run unified parser tests
//!
//! This binary runs the same test suite against all three Perl parsers

use anyhow::Result;
use colored::*;
use parser_tests::{TestCase, corpus, corpus_converter, run_test_on_all_parsers};
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("{}", "Perl Parser Test Suite".bold().blue());
    println!("{}", "======================".blue());
    println!();

    // Load tests
    let mut tests = vec![];

    // Add some basic tests
    tests.extend(basic_tests());

    // Load converted corpus tests
    match corpus_converter::convert_perl_parser_tests() {
        Ok(converted_tests) => {
            println!("Loaded {} converted tests", converted_tests.len());
            tests.extend(converted_tests);
        }
        Err(e) => {
            eprintln!("Warning: Failed to load converted tests: {}", e);
        }
    }

    // Load corpus tests if available
    let corpus_path = PathBuf::from("test/corpus");
    if corpus_path.exists() {
        match corpus::load_corpus_directory(&corpus_path) {
            Ok(corpus_tests) => {
                println!("Loaded {} corpus tests from {:?}", corpus_tests.len(), corpus_path);
                tests.extend(corpus_tests);
            }
            Err(e) => {
                eprintln!("Warning: Failed to load corpus tests: {}", e);
            }
        }
    }

    // Run tests
    let mut results_by_parser: HashMap<String, Vec<bool>> = HashMap::new();

    println!("\nRunning {} tests...\n", tests.len());

    for test in &tests {
        println!("Test: {}", test.name.bold());
        if let Some(desc) = &test.description {
            println!("  {}", desc.dimmed());
        }

        let results = run_test_on_all_parsers(test);

        for result in results {
            let status = if result.success { "PASS".green() } else { "FAIL".red() };

            println!(
                "  {} {}: {} ({:?})",
                status,
                result.parser_name.cyan(),
                status,
                result.parse_time
            );

            if !result.success {
                if let Some(error) = &result.error {
                    println!("    Error: {}", error.red());
                }
            }

            // Track results
            results_by_parser.entry(result.parser_name).or_default().push(result.success);
        }
        println!();
    }

    // Summary
    println!("{}", "Summary".bold().blue());
    println!("{}", "-------".blue());

    for (parser, results) in &results_by_parser {
        let passed = results.iter().filter(|&&x| x).count();
        let total = results.len();
        let percentage = (passed as f64 / total as f64) * 100.0;

        let status = if percentage >= 95.0 {
            format!("{:.1}%", percentage).green()
        } else if percentage >= 80.0 {
            format!("{:.1}%", percentage).yellow()
        } else {
            format!("{:.1}%", percentage).red()
        };

        println!("{}: {}/{} tests passed ({})", parser.cyan(), passed, total, status);
    }

    Ok(())
}

/// Basic test cases to get started
fn basic_tests() -> Vec<TestCase> {
    vec![
        TestCase {
            name: "simple_variable".to_string(),
            input: "my $x = 42;".to_string(),
            description: Some("Simple scalar variable declaration".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "print_statement".to_string(),
            input: r#"print "Hello, world!\n";"#.to_string(),
            description: Some("Basic print statement".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "if_statement".to_string(),
            input: r#"if ($x > 0) { print "positive\n"; }"#.to_string(),
            description: Some("Simple if statement".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "subroutine".to_string(),
            input: r#"sub hello { my $name = shift; print "Hello, $name!\n"; }"#.to_string(),
            description: Some("Basic subroutine definition".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "regex_match".to_string(),
            input: r#"if ($text =~ /pattern/) { print "matched\n"; }"#.to_string(),
            description: Some("Regular expression matching".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "array_ops".to_string(),
            input: r#"my @array = (1, 2, 3); push @array, 4; my $len = @array;"#.to_string(),
            description: Some("Array operations".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "hash_ops".to_string(),
            input: r#"my %hash = (name => "John", age => 30); my $name = $hash{name};"#.to_string(),
            description: Some("Hash operations".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "foreach_loop".to_string(),
            input: r#"foreach my $item (@list) { print "$item\n"; }"#.to_string(),
            description: Some("Foreach loop".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "heredoc_simple".to_string(),
            input: r#"my $text = <<EOF;
Hello, world!
This is a heredoc.
EOF"#
                .to_string(),
            description: Some("Simple heredoc".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
        TestCase {
            name: "modern_perl".to_string(),
            input: r#"use v5.36; my $x = 42; say "The answer is $x";"#.to_string(),
            description: Some("Modern Perl features".to_string()),
            should_parse: true,
            expected_sexp: None,
        },
    ]
}
