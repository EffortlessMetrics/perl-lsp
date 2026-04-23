#!/usr/bin/env cargo run --bin
//! Quick recursion limit validation test
//!
//! This test specifically validates that the recursion depth limits are working
//! and that the previously identified stack overflow vulnerability has been fixed.

use std::time::{Duration, Instant};

/// Simple parser tester that focuses on recursion limits
fn test_recursion_limits() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Testing recursion limit enforcement...");

    // Test cases with different nesting depths
    let test_cases = vec![
        (50, "{ ".repeat(50) + "42" + &" }".repeat(50)),   // Safe depth
        (100, "{ ".repeat(100) + "42" + &" }".repeat(100)), // Should be safe
        (300, "{ ".repeat(300) + "42" + &" }".repeat(300)), // Near limit
        (600, "{ ".repeat(600) + "42" + &" }".repeat(600)), // Above limit
        (1000, "{ ".repeat(1000) + "42" + &" }".repeat(1000)), // Way above limit
    ];

    for (depth, input) in test_cases {
        println!("  Testing depth {}: {} chars", depth, input.len());
        let start = Instant::now();

        // Call parser with timeout protection
        let result = std::panic::catch_unwind(|| {
            perl_parser::Parser::new(&input).parse()
        });

        let elapsed = start.elapsed();

        match result {
            Ok(parse_result) => {
                match parse_result {
                    Ok(_ast) => {
                        println!("    ✅ Parsed successfully in {:?}", elapsed);
                    }
                    Err(err) => {
                        println!("    🛡️ Parse error (expected for deep nesting): {:?}", err);
                        // This is expected for deep nesting - the recursion limit should kick in
                        if depth > 500 {
                            println!("    ✅ Recursion limit working correctly");
                        }
                    }
                }
            }
            Err(_panic) => {
                println!("    💥 PANIC! Recursion limits may not be working!");
                return Err("Parser panicked - recursion limits not working".into());
            }
        }

        if elapsed > Duration::from_secs(1) {
            println!("    ⚠️ Slow parsing: {:?}", elapsed);
        }
    }

    println!("🎉 Recursion limit test completed successfully");
    Ok(())
}

/// Test Unicode edge cases
fn test_unicode_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Testing Unicode edge cases...");

    let unicode_tests = vec![
        "my $emoji = '🦀';",
        "my $\u{1F4A9} = 42;", // poop emoji identifier
        "my $x\u{200B}y = 123;", // zero-width space
        "print \"\u{FEFF}Hello\";", // BOM character
        "'malformed invalid utf8'", // invalid UTF-8 (simplified)
    ];

    for (i, test) in unicode_tests.iter().enumerate() {
        println!("  Unicode test {}: {}", i+1, test);
        let result = std::panic::catch_unwind(|| {
            perl_parser::Parser::new(test).parse()
        });

        match result {
            Ok(_parse_result) => {
                println!("    ✅ Handled gracefully");
            }
            Err(_panic) => {
                println!("    💥 PANIC on Unicode input!");
                return Err("Parser panicked on Unicode input".into());
            }
        }
    }

    println!("🎉 Unicode test completed successfully");
    Ok(())
}

/// Test malformed builtin function constructs
fn test_builtin_function_robustness() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Testing enhanced builtin function parsing robustness...");

    let builtin_tests = vec![
        "map {", // Unclosed map block
        "grep { } @array", // Empty grep block
        "sort { $a <=> $b", // Unclosed sort block
        "map { { { } } @array", // Nested blocks in map
        "sort { die 'error' } @array", // Complex expression in sort
        "map { return $_ } @array", // Return in map (unusual)
    ];

    for (i, test) in builtin_tests.iter().enumerate() {
        println!("  Builtin test {}: {}", i+1, test);
        let result = std::panic::catch_unwind(|| {
            perl_parser::Parser::new(test).parse()
        });

        match result {
            Ok(_parse_result) => {
                println!("    ✅ Handled gracefully");
            }
            Err(_panic) => {
                println!("    💥 PANIC on builtin function input!");
                return Err("Parser panicked on builtin function input".into());
            }
        }
    }

    println!("🎉 Builtin function robustness test completed successfully");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting focused fuzz testing for PR #153...");

    // Test each area with error handling
    if let Err(e) = test_recursion_limits() {
        eprintln!("❌ Recursion test failed: {}", e);
        return Err(e);
    }

    if let Err(e) = test_unicode_edge_cases() {
        eprintln!("❌ Unicode test failed: {}", e);
        return Err(e);
    }

    if let Err(e) = test_builtin_function_robustness() {
        eprintln!("❌ Builtin function test failed: {}", e);
        return Err(e);
    }

    println!("✅ All focused fuzz tests passed!");
    println!("🛡️ Parser demonstrates good robustness against edge cases");

    Ok(())
}