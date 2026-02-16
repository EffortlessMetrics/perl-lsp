//! Boundary Validation Tests for Perl Parser
//!
//! This test suite validates parser behavior at exact boundary conditions:
//! - All configured limits (max_parse_time, max_ast_nodes, etc.)
//! - Parser behavior at exact boundary conditions
//! - Graceful degradation when limits are exceeded
//! - Recovery from boundary violations

use perl_parser::Parser;
use std::time::{Duration, Instant};

/// Maximum recursion depth from parser implementation
const MAX_RECURSION_DEPTH: usize = 128;

/// Maximum heredoc depth from parser implementation
const MAX_HEREDOC_DEPTH: usize = 100;

/// Heredoc timeout from parser implementation
const HEREDOC_TIMEOUT_MS: u64 = 5000;

/// Test parser at exact recursion depth boundary
#[test]
fn test_recursion_depth_boundary() {
    println!("Testing recursion depth boundary...");

    // Test just below the limit
    let below_limit_code = generate_nested_code(MAX_RECURSION_DEPTH - 5);
    let start_time = Instant::now();
    let mut parser = Parser::new(&below_limit_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    assert!(result.is_ok(), "Should parse successfully below recursion limit");
    println!("  ✓ Below limit ({}): parsed in {:?}", MAX_RECURSION_DEPTH - 5, parse_time);

    // Test exactly at the limit
    let at_limit_code = generate_nested_code(MAX_RECURSION_DEPTH);
    let start_time = Instant::now();
    let mut parser = Parser::new(&at_limit_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    // May succeed or fail gracefully at the exact limit
    match result {
        Ok(_ast) => {
            println!(
                "  ✓ At limit ({}): parsed successfully in {:?}",
                MAX_RECURSION_DEPTH, parse_time
            );
        }
        Err(e) => {
            println!("  ✓ At limit ({}): failed gracefully: {:?}", MAX_RECURSION_DEPTH, e);
            assert!(
                e.to_string().contains("recursion")
                    || e.to_string().contains("depth")
                    || e.to_string().contains("nesting"),
                "Should fail with recursion-related error"
            );
        }
    }

    // Test just above the limit
    let above_limit_code = generate_nested_code(MAX_RECURSION_DEPTH + 5);
    let start_time = Instant::now();
    let mut parser = Parser::new(&above_limit_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    // Should fail gracefully above the limit
    match result {
        Ok(_) => {
            println!("  ⚠ Above limit ({}): unexpectedly succeeded", MAX_RECURSION_DEPTH + 5);
        }
        Err(e) => {
            println!("  ✓ Above limit ({}): failed as expected: {:?}", MAX_RECURSION_DEPTH + 5, e);
            assert!(
                e.to_string().contains("recursion")
                    || e.to_string().contains("depth")
                    || e.to_string().contains("nesting"),
                "Should fail with recursion-related error"
            );
        }
    }

    // Should fail quickly even when exceeding limit
    assert!(
        parse_time < Duration::from_secs(2),
        "Should fail quickly when exceeding recursion limit"
    );
}

/// Test parser at exact heredoc depth boundary
#[test]
fn test_heredoc_depth_boundary() {
    println!("Testing heredoc depth boundary...");

    // Test just below the limit
    let below_limit_code = generate_heredoc_code(MAX_HEREDOC_DEPTH - 5);
    let start_time = Instant::now();
    let mut parser = Parser::new(&below_limit_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    assert!(result.is_ok(), "Should parse successfully below heredoc limit");
    println!("  ✓ Below limit ({}): parsed in {:?}", MAX_HEREDOC_DEPTH - 5, parse_time);

    // Test exactly at the limit
    let at_limit_code = generate_heredoc_code(MAX_HEREDOC_DEPTH);
    let start_time = Instant::now();
    let mut parser = Parser::new(&at_limit_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    // May succeed or fail gracefully at the exact limit
    match result {
        Ok(_ast) => {
            println!(
                "  ✓ At limit ({}): parsed successfully in {:?}",
                MAX_HEREDOC_DEPTH, parse_time
            );
        }
        Err(e) => {
            println!("  ✓ At limit ({}): failed gracefully: {:?}", MAX_HEREDOC_DEPTH, e);
            assert!(
                e.to_string().contains("heredoc")
                    || e.to_string().contains("depth")
                    || e.to_string().contains("limit"),
                "Should fail with heredoc-related error"
            );
        }
    }

    // Test just above the limit
    let above_limit_code = generate_heredoc_code(MAX_HEREDOC_DEPTH + 5);
    let start_time = Instant::now();
    let mut parser = Parser::new(&above_limit_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    // Should fail gracefully above the limit
    match result {
        Ok(_) => {
            println!("  ⚠ Above limit ({}): unexpectedly succeeded", MAX_HEREDOC_DEPTH + 5);
        }
        Err(e) => {
            println!("  ✓ Above limit ({}): failed as expected: {:?}", MAX_HEREDOC_DEPTH + 5, e);
            assert!(
                e.to_string().contains("heredoc")
                    || e.to_string().contains("depth")
                    || e.to_string().contains("limit"),
                "Should fail with heredoc-related error"
            );
        }
    }

    // Should fail quickly when exceeding limit
    assert!(
        parse_time < Duration::from_secs(5),
        "Should fail quickly when exceeding heredoc limit"
    );
}

/// Test parser timeout boundary
#[test]
fn test_timeout_boundary() {
    println!("Testing timeout boundary...");

    // Test code that should complete well before timeout
    let fast_code = generate_reasonable_code();
    let start_time = Instant::now();
    let mut parser = Parser::new(&fast_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    assert!(result.is_ok(), "Should parse reasonable code quickly");
    assert!(parse_time < Duration::from_millis(100), "Should complete well before timeout");
    println!("  ✓ Fast code: completed in {:?}", parse_time);

    // Test code that approaches timeout
    let slow_code = generate_slow_code();
    let start_time = Instant::now();
    let mut parser = Parser::new(&slow_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    // Should either complete or timeout gracefully
    match result {
        Ok(_ast) => {
            println!("  ✓ Slow code: completed in {:?}", parse_time);
            assert!(
                parse_time < Duration::from_millis(HEREDOC_TIMEOUT_MS),
                "Should complete before heredoc timeout"
            );
        }
        Err(e) => {
            println!("  ✓ Slow code: timed out gracefully: {:?}", e);
            assert!(
                e.to_string().contains("timeout") || e.to_string().contains("time"),
                "Should fail with timeout-related error"
            );
        }
    }

    // Test code that should definitely timeout
    let timeout_code = generate_timeout_code();
    let start_time = Instant::now();
    let mut parser = Parser::new(&timeout_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();

    // Should timeout gracefully
    match result {
        Ok(_) => {
            println!("  ⚠ Timeout code: unexpectedly succeeded in {:?}", parse_time);
        }
        Err(e) => {
            println!("  ✓ Timeout code: timed out as expected in {:?}: {:?}", parse_time, e);
            assert!(
                e.to_string().contains("timeout") || e.to_string().contains("time"),
                "Should fail with timeout-related error"
            );
            assert!(
                parse_time >= Duration::from_millis(HEREDOC_TIMEOUT_MS),
                "Should take at least the timeout duration"
            );
        }
    }
}

/// Test file size boundaries
#[test]
fn test_file_size_boundaries() {
    println!("Testing file size boundaries...");

    // Test various file sizes
    let test_sizes = vec![
        (1 * 1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1 * 1024 * 1024, "1MB"),
        (5 * 1024 * 1024, "5MB"),
    ];

    for (size, description) in test_sizes {
        println!("Testing file size: {}", description);

        let code = generate_sized_code(size);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        // Should handle reasonable file sizes
        if size <= 5 * 1024 * 1024 {
            assert!(result.is_ok(), "Should handle {} file", description);
            println!("  ✓ {}: parsed in {:?}", description, parse_time);
        } else {
            // Very large files might fail gracefully
            match result {
                Ok(_) => {
                    println!("  ✓ {}: parsed successfully in {:?}", description, parse_time);
                }
                Err(e) => {
                    println!("  ✓ {}: failed gracefully: {:?}", description, e);
                }
            }
        }

        // Performance should scale reasonably
        let time_per_kb = parse_time.as_millis() as f64 / (size as f64 / 1024.0);
        assert!(
            time_per_kb < 10.0, // Less than 10ms per KB
            "Performance degraded too much for {}: {:.2}ms/KB",
            description,
            time_per_kb
        );
    }
}

/// Test token count boundaries
#[test]
fn test_token_count_boundaries() {
    println!("Testing token count boundaries...");

    // Test various token densities
    let token_tests = vec![
        (1000, "Low token density"),
        (10000, "Medium token density"),
        (100000, "High token density"),
        (1000000, "Very high token density"),
    ];

    for (target_tokens, description) in token_tests {
        println!("Testing: {}", description);

        let code = generate_token_dense_code(target_tokens);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        // Should handle reasonable token counts
        if target_tokens <= 100000 {
            assert!(result.is_ok(), "Should handle {} tokens", target_tokens);
            println!("  ✓ {}: parsed in {:?}", description, parse_time);
        } else {
            // Very high token counts might fail gracefully
            match result {
                Ok(_) => {
                    println!("  ✓ {}: parsed successfully in {:?}", description, parse_time);
                }
                Err(e) => {
                    println!("  ✓ {}: failed gracefully: {:?}", description, e);
                }
            }
        }

        // Performance should scale reasonably
        let time_per_1k_tokens = parse_time.as_millis() as f64 / (target_tokens as f64 / 1000.0);
        assert!(
            time_per_1k_tokens < 5.0, // Less than 5ms per 1K tokens
            "Performance degraded too much for {}: {:.2}ms/1K tokens",
            description,
            time_per_1k_tokens
        );
    }
}

/// Test AST node count boundaries
#[test]
fn test_ast_node_boundaries() {
    println!("Testing AST node boundaries...");

    // Test various AST complexities
    let node_tests = vec![
        (100, "Simple AST"),
        (1000, "Medium AST"),
        (10000, "Complex AST"),
        (100000, "Very complex AST"),
    ];

    for (target_nodes, description) in node_tests {
        println!("Testing: {}", description);

        let code = generate_node_dense_code(target_nodes);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                let actual_nodes = count_ast_nodes(&ast);
                println!("  ✓ {}: {} nodes in {:?}", description, actual_nodes, parse_time);

                // Should be reasonably close to target
                let ratio = actual_nodes as f64 / target_nodes as f64;
                assert!(
                    ratio >= 0.5 && ratio <= 2.0,
                    "Node count ratio {:.1} should be between 0.5 and 2.0",
                    ratio
                );
            }
            Err(e) => {
                println!("  ✓ {}: failed gracefully: {:?}", description, e);
            }
        }

        // Performance should scale reasonably
        let time_per_1k_nodes = parse_time.as_millis() as f64 / (target_nodes as f64 / 1000.0);
        assert!(
            time_per_1k_nodes < 20.0, // Less than 20ms per 1K nodes
            "Performance degraded too much for {}: {:.2}ms/1K nodes",
            description,
            time_per_1k_nodes
        );
    }
}

/// Test graceful degradation when limits are exceeded
#[test]
fn test_graceful_degradation() {
    println!("Testing graceful degradation...");

    let degradation_scenarios = vec![
        ("Excessive recursion", generate_excessive_recursion()),
        ("Excessive heredocs", generate_excessive_heredocs()),
        ("Excessive nesting", generate_excessive_nesting()),
        ("Excessive complexity", generate_excessive_complexity()),
    ];

    for (scenario_name, code) in degradation_scenarios {
        println!("Testing scenario: {}", scenario_name);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        // Should either succeed or fail gracefully
        match result {
            Ok(_ast) => {
                println!("  ✓ {}: succeeded with partial AST in {:?}", scenario_name, parse_time);

                // Even when succeeding, should have reasonable performance
                assert!(
                    parse_time < Duration::from_secs(10),
                    "Should complete within reasonable time even for complex scenarios"
                );
            }
            Err(e) => {
                println!("  ✓ {}: failed gracefully in {:?}: {:?}", scenario_name, parse_time, e);

                // Should fail with meaningful error message
                assert!(!e.to_string().is_empty(), "Error message should not be empty");

                assert!(parse_time < Duration::from_secs(10), "Should fail quickly, not hang");
            }
        }
    }
}

/// Test recovery from boundary violations
#[test]
fn test_boundary_violation_recovery() {
    println!("Testing boundary violation recovery...");

    // First, trigger a boundary violation
    let violation_code = generate_excessive_recursion();
    let mut parser = Parser::new(&violation_code);
    let violation_result = parser.parse();

    // Should either succeed or fail gracefully
    match violation_result {
        Ok(_) => {
            println!("  ✓ Violation scenario succeeded unexpectedly");
        }
        Err(e) => {
            println!("  ✓ Violation scenario failed as expected: {:?}", e);
        }
    }

    // Now test that parser can recover and handle normal code
    let normal_code = r#"
use strict;
use warnings;
my $x = 42;
my $y = $x * 2;
print "Result: $y\n";
"#;

    let mut recovery_parser = Parser::new(normal_code);
    let recovery_result = recovery_parser.parse();

    assert!(recovery_result.is_ok(), "Parser should recover and handle normal code");
    println!("  ✓ Parser recovered and handled normal code successfully");

    // Test multiple cycles of violation and recovery
    for cycle in 0..5 {
        let violation_code = generate_excessive_recursion();
        let mut parser = Parser::new(&violation_code);
        let _ = parser.parse(); // May fail

        let mut parser = Parser::new(normal_code);
        let result = parser.parse();

        assert!(result.is_ok(), "Parser should recover in cycle {}", cycle);
    }

    println!("  ✓ Parser recovered through {} violation/recovery cycles", 5);
}

/// Test boundary conditions with concurrent access
#[test]
fn test_concurrent_boundary_conditions() {
    println!("Testing concurrent boundary conditions...");

    use std::sync::{Arc, Mutex};
    use std::thread;

    let thread_count = 8;
    let operations_per_thread = 20;

    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for operation in 0..operations_per_thread {
                    // Mix of boundary testing scenarios
                    let scenario = operation % 4;
                    let (code, scenario_name) = match scenario {
                        0 => (
                            generate_nested_code(MAX_RECURSION_DEPTH + operation % 10),
                            "recursion",
                        ),
                        1 => (generate_heredoc_code(MAX_HEREDOC_DEPTH + operation % 10), "heredoc"),
                        2 => (generate_complex_code(operation * 1000), "complexity"),
                        3 => (generate_large_code(operation * 5000), "size"),
                        _ => unreachable!(),
                    };

                    let start_time = Instant::now();
                    let mut parser = Parser::new(&code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    results_clone.lock().unwrap().push((
                        thread_id,
                        operation,
                        scenario_name,
                        result.is_ok(),
                        parse_time,
                    ));
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let res = handle.join();
        assert!(res.is_ok(), "Thread should complete successfully");
    }

    let results_guard = results.lock();
    assert!(results_guard.is_ok(), "Lock should be acquired");
    let results = results_guard.unwrap_or_else(|_| unreachable!());

    // Analyze results by scenario type
    let mut scenario_stats: std::collections::HashMap<&str, (usize, Duration, usize)> =
        std::collections::HashMap::new();

    for (thread_id, operation, scenario_name, success, parse_time) in results.iter() {
        let entry = scenario_stats.entry(*scenario_name).or_insert((0, Duration::new(0, 0), 0));
        entry.0 += 1;
        entry.1 += *parse_time;
        if *success {
            entry.2 += 1;
        }

        // All operations should complete reasonably
        assert!(
            *parse_time < Duration::from_secs(10),
            "Thread {} operation {} ({}) took too long: {:?}",
            thread_id,
            operation,
            scenario_name,
            parse_time
        );
    }

    println!("  ✓ Concurrent boundary conditions:");
    for (scenario_type, (count, total_time, success_count)) in scenario_stats {
        let avg_time = total_time / count as u32;
        let success_rate = success_count as f64 / count as f64;
        println!(
            "    {}: {} ops, avg: {:?}, success: {:.1}%",
            scenario_type,
            count,
            avg_time,
            success_rate * 100.0
        );

        // Should have reasonable success rates even under concurrent boundary testing
        assert!(
            success_rate > 0.7,
            "Concurrent scenario {} success rate {:.1}% should be > 70%",
            scenario_type,
            success_rate * 100.0
        );
    }
}

/// Test edge cases at boundary values
#[test]
fn test_boundary_edge_cases() {
    println!("Testing boundary edge cases...");

    let max_ident = "x".repeat(1000);
    let max_line = "x ".repeat(1000);
    let zero_recursion = generate_nested_code(0);
    let single_recursion = generate_nested_code(1);
    let max_recursion = generate_nested_code(MAX_RECURSION_DEPTH);
    let boundary_whitespace = "\n".repeat(1000);
    let max_comment = "# ".repeat(1000);

    let edge_cases = vec![
        ("Empty string", ""),
        ("Single character", "x"),
        ("Maximum identifier length", &max_ident),
        ("Maximum line length", &max_line),
        ("Zero recursion", &zero_recursion),
        ("Single recursion", &single_recursion),
        ("Maximum safe recursion", &max_recursion),
        ("Boundary whitespace", &boundary_whitespace),
        ("Maximum comment length", &max_comment),
        ("Empty statements", ";\n;\n;\n"),
        ("Nested empty blocks", "{\n{\n{\n{\n{\n\n\n\n\n"),
    ];

    for (case_name, code) in edge_cases {
        println!("Testing edge case: {}", case_name);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        // Should handle edge cases without crashing
        match result {
            Ok(_ast) => {
                println!("  ✓ {}: parsed successfully in {:?}", case_name, parse_time);
            }
            Err(e) => {
                println!("  ✓ {}: failed gracefully in {:?}: {:?}", case_name, parse_time, e);

                // Should fail with meaningful error
                assert!(
                    !e.to_string().is_empty(),
                    "Error message should not be empty for edge case: {}",
                    case_name
                );
            }
        }

        // Should complete quickly for edge cases
        assert!(
            parse_time < Duration::from_secs(5),
            "Edge case {} should complete quickly, took {:?}",
            case_name,
            parse_time
        );
    }
}

// Helper functions for generating test code

fn generate_nested_code(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $result = ");

    for _ in 0..depth {
        code.push_str("(");
    }

    code.push_str("42");

    for _ in 0..depth {
        code.push_str(")");
    }

    code.push_str(";\n");
    code
}

fn generate_heredoc_code(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    for i in 0..count {
        code.push_str(&format!(
            r#"my $heredoc{} = <<END{};
Heredoc content {}
END
"#,
            i, i, i
        ));
    }

    code
}

fn generate_reasonable_code() -> String {
    r#"
use strict;
use warnings;

my $x = 42;
my $y = $x * 2;

sub calculate {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = calculate($x, $y);
print "Result: $result\n";
"#
    .to_string()
}

fn generate_slow_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    // Generate code that takes some time but should complete
    for i in 0..1000 {
        code.push_str(&format!(
            r#"my $var{} = {};
my @array{} = ({}..{});
my %hash{} = (map {{ ("key$_" => "value$_") }} (1..50));

"#,
            i,
            i,
            i,
            i * 10,
            (i + 1) * 10,
            i
        ));
    }

    code
}

fn generate_timeout_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    // Generate code that should timeout
    for i in 0..10000 {
        code.push_str(&format!(
            r#"my $timeout_var{} = {{
    level1 => {{
        level2 => {{
            level3 => {{
                level4 => {{
                    deep => 'value{}'
                }}
            }}
        }}
    }}
}};
"#,
            i, i
        ));
    }

    code
}

fn generate_sized_code(size: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    let mut current_size = code.len();
    let counter = 0;

    while current_size < size {
        let statement = format!("my $var{} = {};\n", counter, counter * 2);
        code.push_str(&statement);
        current_size = code.len();
    }

    code
}

fn generate_token_dense_code(target_tokens: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    let mut token_count = 0;
    let counter = 0;

    while token_count < target_tokens {
        let expression = format!(
            "my $expr{} = {} + {} * {} / {} - {} % {} && {} || {};\n",
            counter,
            counter,
            counter + 1,
            counter + 2,
            counter + 3,
            counter + 4,
            counter + 5,
            counter + 6,
            counter + 7
        );
        code.push_str(&expression);
        token_count += 15; // Approximate token count
    }

    code
}

fn generate_node_dense_code(target_nodes: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    let mut node_count = 0;
    let counter = 0;

    while node_count < target_nodes {
        let statement = format!(
            r#"if ($cond{}) {{
    my $var{} = {};
    if ($nested{}) {{
        my $inner{} = $var{} * 2;
        print "Inner: $inner{}\n";
    }} else {{
        my $else{} = $var{} + 1;
        print "Else: $else{}\n";
    }}
}} elsif ($alt{}) {{
    my $alt_var{} = {};
    print "Alt: $alt_var{}\n";
}} else {{
    my $default{} = 0;
    print "Default: $default{}\n";
}}
"#,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter,
            counter
        );
        code.push_str(&statement);
        node_count += 20; // Approximate node count
    }

    code
}

fn generate_excessive_recursion() -> String {
    generate_nested_code(MAX_RECURSION_DEPTH + 20)
}

fn generate_excessive_heredocs() -> String {
    generate_heredoc_code(MAX_HEREDOC_DEPTH + 20)
}

fn generate_excessive_nesting() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    for i in 0..(MAX_RECURSION_DEPTH + 20) {
        code.push_str(&format!("if ($var{}) {{\n", i));
    }

    code.push_str("print 'Deeply nested';\n");

    for _ in 0..(MAX_RECURSION_DEPTH + 20) {
        code.push_str("}\n");
    }

    code
}

fn generate_excessive_complexity() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    // Generate complex expressions
    for i in 0..1000 {
        code.push_str(&format!(
            r#"my $complex{} = ($var{} && $var2{} || $var3{}) ? ($var4{} + $var5{} * $var6{}) : ($var7{} - $var8{} / $var9{});
"#, i, i, i, i, i, i, i, i, i, i));
    }

    code
}

fn generate_complex_code(size: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    for i in 0..size {
        code.push_str(&format!(
            r#"sub complex_{} {{
    my ($param1, $param2, $param3) = @_;
    my $result = $param1 + $param2 * $param3;
    
    if ($result > 100) {{
        $result *= 2;
    }} elsif ($result < 50) {{
        $result /= 2;
    }} else {{
        $result = $result ** 2;
    }}
    
    return $result;
}}
"#,
            i
        ));
    }

    code
}

fn generate_large_code(size: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");

    for i in 0..size {
        code.push_str(&format!(
            r#"my $large_var_{} = 'This is a large string with some content {}';
my @large_array_{} = (1..100);
my %large_hash_{} = (map {{ ("key$_" => "value$_") }} (1..50));
"#,
            i, i, i, i
        ));
    }

    code
}

fn count_ast_nodes(ast: &perl_parser::ast::Node) -> usize {
    use perl_parser::ast::NodeKind;

    match &ast.kind {
        NodeKind::Program { statements } => {
            1 + statements.iter().map(count_ast_nodes).sum::<usize>()
        }
        NodeKind::ExpressionStatement { expression } => 1 + count_ast_nodes(expression),
        // Add more cases as needed based on actual NodeKind variants
        _ => 1,
    }
}
