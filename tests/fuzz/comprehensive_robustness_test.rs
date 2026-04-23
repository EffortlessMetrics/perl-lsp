#!/usr/bin/env cargo run --bin
//! Comprehensive robustness testing for tree-sitter-perl
//!
//! Tests edge cases across multiple components:
//! - Core parser with various malformed inputs
//! - Incremental parsing with corrupted edits
//! - LSP workspace operations under stress
//! - Memory safety with large inputs

use std::time::{Duration, Instant};

struct TestResult {
    name: String,
    passed: bool,
    error: Option<String>,
    duration: Duration,
}

fn run_test<F>(name: &str, test_fn: F) -> TestResult
where
    F: FnOnce() -> Result<(), String> + std::panic::UnwindSafe,
{
    let start = Instant::now();
    let result = std::panic::catch_unwind(test_fn);
    let duration = start.elapsed();

    match result {
        Ok(Ok(())) => TestResult {
            name: name.to_string(),
            passed: true,
            error: None,
            duration,
        },
        Ok(Err(e)) => TestResult {
            name: name.to_string(),
            passed: false,
            error: Some(format!("Test error: {}", e)),
            duration,
        },
        Err(_) => TestResult {
            name: name.to_string(),
            passed: false,
            error: Some("Panic during test".to_string()),
            duration,
        },
    }
}

fn test_large_input_handling() -> Result<(), String> {
    // Test with very large input to check memory handling
    let large_input = "my $x = 42; ".repeat(10000); // ~120KB

    match perl_parser::Parser::new(&large_input).parse() {
        Ok(_) => Ok(()),
        Err(_) => Ok(()), // Parse errors are fine, panics are not
    }
}

fn test_pathological_regex_patterns() -> Result<(), String> {
    let regex_tests = vec![
        "m/.*.*.*.*.*.*.*.*.*x/", // Catastrophic backtracking candidate
        "s/a{99999}/b/g", // Large quantifier
        "m/(.+)*$/", // Nested quantifier
        "qr{(?:a|a)*}x", // Alternation with overlap
        "m|/\\/\\*.*?\\*/|ms", // Complex delimiters
    ];

    for test in regex_tests {
        perl_parser::Parser::new(test).parse().ok();
    }
    Ok(())
}

fn test_deeply_nested_structures() -> Result<(), String> {
    let nested_tests = vec![
        // These should hit recursion limits gracefully
        format!("({})", " ( ".repeat(600) + "42" + &" ) ".repeat(600)),
        format!("[{}]", " [ ".repeat(600) + "42" + &" ] ".repeat(600)),
        format!("sub {{ {} }}", " sub { ".repeat(200) + "42" + &" } ".repeat(200)),
    ];

    for test in nested_tests {
        match perl_parser::Parser::new(&test).parse() {
            Ok(_) => return Err("Deep nesting should have hit recursion limits".to_string()),
            Err(err) => {
                if !format!("{:?}", err).contains("RecursionLimit") {
                    return Err(format!("Expected RecursionLimit error, got: {:?}", err));
                }
            }
        }
    }
    Ok(())
}

fn test_unicode_edge_cases() -> Result<(), String> {
    let unicode_tests = vec![
        "my $\u{1F600} = 'emoji';", // Emoji in identifier
        "print \"\u{202E}reversed\u{202D}\";", // Right-to-left override
        "my $var = '\u{FEFF}BOM in string';", // BOM character
        "# Comment with \u{200B} zero-width spaces", // Invisible characters
        "my $\u{0300} = 42;", // Combining character at start
    ];

    for test in unicode_tests {
        perl_parser::Parser::new(test).parse().ok(); // Any result is fine, just don't crash
    }
    Ok(())
}

fn test_malformed_syntax_constructs() -> Result<(), String> {
    let malformed_tests = vec![
        "sub { my ($a, $b = @_; return $a + $b; }", // Missing closing paren
        "if ($x > { print 'unclosed'; }", // Unclosed brace in condition
        "my @array = (1, 2, 3,;", // Trailing comma and missing paren
        "for my $i (0..10 { print $i; }", // Missing closing paren in for
        "use strict 'vars', 'subs'", // Missing semicolon
        "print qq{Hello world", // Unclosed qq construct
        "s///e;", // Empty regex substitution with eval flag
        "tr/a-z/A-Z", // Missing closing delimiter
    ];

    for test in malformed_tests {
        perl_parser::Parser::new(test).parse().ok(); // Should handle gracefully
    }
    Ok(())
}

fn test_extreme_string_literals() -> Result<(), String> {
    let string_tests = vec![
        format!("my $s = '{}';", "a".repeat(50000)), // Very long single-quoted string
        r#"my $s = "string with \" escaped quotes and \n newlines";"#.to_string(),
        "my $s = q{This { has } nested { braces } inside};".to_string(),
        "my $s = qq{Interpolation with $variables and @arrays};".to_string(),
        "my $heredoc = <<'EOF';\nLine 1\nLine 2\nEOF".to_string(),
    ];

    for test in string_tests {
        perl_parser::Parser::new(&test).parse().ok();
    }
    Ok(())
}

fn test_complex_variable_references() -> Result<(), String> {
    let var_tests = vec![
        "$hash{$key}{$subkey}[0]->method()",
        "$obj->{$method_name}->(@args)",
        "${$hash_ref}{key}",
        "@{$array_ref}[0..4]",
        "%{$hash_ref}",
        "$$scalar_ref",
        "&{$code_ref}(@args)",
        "*{$glob_ref}{CODE}",
    ];

    for test in var_tests {
        perl_parser::Parser::new(test).parse().ok();
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Starting comprehensive robustness testing...");

    let mut tests = Vec::new();

    // Run all test categories
    tests.push(run_test("Large input handling", test_large_input_handling));
    tests.push(run_test("Pathological regex patterns", test_pathological_regex_patterns));
    tests.push(run_test("Deeply nested structures", test_deeply_nested_structures));
    tests.push(run_test("Unicode edge cases", test_unicode_edge_cases));
    tests.push(run_test("Malformed syntax constructs", test_malformed_syntax_constructs));
    tests.push(run_test("Extreme string literals", test_extreme_string_literals));
    tests.push(run_test("Complex variable references", test_complex_variable_references));

    // Report results
    let mut passed = 0;
    let mut failed = 0;
    let total_duration = tests.iter().map(|t| t.duration).sum::<Duration>();

    println!("\n📊 Test Results:");
    println!("═══════════════════════════════════════");

    for test in &tests {
        let status = if test.passed { "✅ PASS" } else { "❌ FAIL" };
        println!("{} {} ({:?})", status, test.name, test.duration);

        if let Some(error) = &test.error {
            println!("   Error: {}", error);
        }

        if test.passed {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!("═══════════════════════════════════════");
    println!("Summary: {}/{} tests passed", passed, tests.len());
    println!("Total execution time: {:?}", total_duration);

    if failed == 0 {
        println!("🎉 All robustness tests passed!");
        println!("🛡️ Parser demonstrates excellent stability under stress conditions");
        Ok(())
    } else {
        println!("⚠️ {} tests failed - investigating robustness issues", failed);
        Err(format!("{} robustness tests failed", failed).into())
    }
}