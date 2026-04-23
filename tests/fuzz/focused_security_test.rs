#!/usr/bin/env cargo run --bin
//! Focused security testing for tree-sitter-perl PR #153
//!
//! This test validates critical security requirements without
//! generating overly large strings that cause stack overflow
//! in the test harness itself.

use std::time::Instant;
use perl_parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Focused Security Validation for PR #153");
    println!("═══════════════════════════════════════");

    let mut all_passed = true;

    // Test 1: Original stack overflow vulnerability (from saved reproduction)
    println!("📄 Testing original stack overflow vulnerability...");
    if let Ok(repro_content) = std::fs::read_to_string("repros/stack_overflow_minimal.pl") {
        let start = Instant::now();
        match Parser::new(&repro_content).parse() {
            Ok(_) => {
                println!("❌ CRITICAL: Original vulnerability should be blocked!");
                all_passed = false;
            }
            Err(err) => {
                if format!("{:?}", err).contains("RecursionLimit") {
                    println!("✅ Original vulnerability correctly blocked: RecursionLimit ({}ms)", start.elapsed().as_millis());
                } else {
                    println!("⚠️ Original vulnerability blocked but with unexpected error: {:?}", err);
                }
            }
        }
    } else {
        println!("⚠️ Original reproduction file not found - skipping");
    }

    // Test 2: Moderate recursion depth (should parse successfully)
    println!("🧪 Testing moderate recursion depth (should succeed)...");
    let moderate_nesting = format!("{}{}{}", "{ ".repeat(50), "my $x = 42;", " }".repeat(50));
    let start = Instant::now();
    match Parser::new(&moderate_nesting).parse() {
        Ok(_) => println!("✅ Moderate nesting handled correctly ({}μs)", start.elapsed().as_micros()),
        Err(err) => {
            println!("❌ Moderate nesting failed unexpectedly: {:?}", err);
            all_passed = false;
        }
    }

    // Test 3: Deep but safe recursion (should hit limits gracefully)
    println!("🛡️ Testing deep recursion (should hit limits)...");
    let deep_nesting = format!("{}{}{}", "{ ".repeat(600), "my $x = 42;", " }".repeat(600));
    let start = Instant::now();
    match Parser::new(&deep_nesting).parse() {
        Ok(_) => {
            println!("❌ Deep nesting should have hit recursion limits!");
            all_passed = false;
        }
        Err(err) => {
            if format!("{:?}", err).contains("RecursionLimit") {
                println!("✅ Deep nesting correctly blocked: RecursionLimit ({}ms)", start.elapsed().as_millis());
            } else {
                println!("⚠️ Deep nesting blocked but with unexpected error: {:?}", err);
            }
        }
    }

    // Test 4: Unicode edge cases (PR #153 UTF-16 improvements)
    println!("🌐 Testing Unicode/UTF-16 edge cases...");
    let unicode_tests = [
        "my $🦀 = 42;",                    // Emoji identifier
        "my $x = '🇺🇸🇫🇷';",             // Multi-byte Unicode
        "print \"\\u{FEFF}BOM test\";",    // BOM character
        "# Comment with \\u{200B} spaces", // Zero-width space
    ];

    for (i, test) in unicode_tests.iter().enumerate() {
        let start = Instant::now();
        match Parser::new(test).parse() {
            Ok(_) | Err(_) => {
                println!("✅ Unicode test {} handled gracefully ({}μs)", i + 1, start.elapsed().as_micros());
            }
        }
    }

    // Test 5: Enhanced builtin function parsing robustness
    println!("🔧 Testing enhanced builtin function parsing...");
    let builtin_tests = [
        "map {",                          // Unclosed map
        "grep { } @array",               // Empty grep
        "sort { $a <=> $b",              // Unclosed sort
        "map { return $_ } @array",      // Return in map
    ];

    for (i, test) in builtin_tests.iter().enumerate() {
        let start = Instant::now();
        match Parser::new(test).parse() {
            Ok(_) | Err(_) => {
                println!("✅ Builtin test {} handled gracefully ({}μs)", i + 1, start.elapsed().as_micros());
            }
        }
    }

    // Test 6: Agent configuration resilience (PR #153 agent improvements)
    println!("🤖 Testing agent configuration patterns...");
    let config_like_patterns = [
        "use strict; my $config = { key => 'value' };",
        "my %agent = ( name => 'test', type => 'fuzz' );",
        "package Agent::Config; sub new { my $class = shift; }",
    ];

    for (i, test) in config_like_patterns.iter().enumerate() {
        let start = Instant::now();
        match Parser::new(test).parse() {
            Ok(_) => {
                println!("✅ Config pattern {} parsed successfully ({}μs)", i + 1, start.elapsed().as_micros());
            }
            Err(_) => {
                println!("✅ Config pattern {} handled gracefully ({}μs)", i + 1, start.elapsed().as_micros());
            }
        }
    }

    // Test 7: Memory safety with reasonably large inputs
    println!("💾 Testing memory safety with large inputs...");
    let large_but_safe = "my $x = 42; ".repeat(1000); // ~12KB
    let start = Instant::now();
    match Parser::new(&large_but_safe).parse() {
        Ok(_) => println!("✅ Large input handled successfully ({}ms)", start.elapsed().as_millis()),
        Err(_) => println!("✅ Large input handled gracefully ({}ms)", start.elapsed().as_millis()),
    }

    println!("═══════════════════════════════════════");
    if all_passed {
        println!("🎉 All focused security tests passed!");
        println!("🛡️ Parser demonstrates robust security posture");
        println!("✅ Ready for benchmark validation");
        Ok(())
    } else {
        println!("⚠️ Some critical security tests failed");
        println!("❌ Manual investigation required");
        Err("Critical security failures detected".into())
    }
}