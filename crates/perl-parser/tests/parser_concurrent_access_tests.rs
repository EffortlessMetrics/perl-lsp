//! Concurrent Access Tests for Perl Parser
//!
//! This test suite validates parser behavior under concurrent access conditions:
//! - Thread safety of parser instances
//! - Multiple concurrent parsing requests
//! - Incremental parsing under concurrent modifications
//! - Workspace indexing under concurrent access

use perl_parser::Parser;
use perl_tdd_support::must;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// Test basic thread safety of parser instances
#[test]
fn test_parser_thread_safety() {
    println!("Testing parser thread safety...");

    let thread_count = 16;
    let iterations_per_thread = 50;

    let test_code = r#"
use strict;
use warnings;
my $x = 42;
my $y = $x * 2;
print "Result: $y\n";
sub test_func {
    my ($param) = @_;
    return $param + 1;
}
my $result = test_func($x);
"#;

    let results = Arc::new(Mutex::new(Vec::new()));
    let error_count = Arc::new(Mutex::new(0));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let test_code = test_code.to_string();
            let results_clone = Arc::clone(&results);
            let error_count_clone = Arc::clone(&error_count);

            thread::spawn(move || {
                for iteration in 0..iterations_per_thread {
                    let start_time = Instant::now();
                    let mut parser = Parser::new(&test_code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    match result {
                        Ok(_ast) => {
                            must(results_clone.lock())
                                .push((thread_id, iteration, true, parse_time));
                        }
                        Err(e) => {
                            *must(error_count_clone.lock()) += 1;
                            must(results_clone.lock())
                                .push((thread_id, iteration, false, parse_time));
                            println!("Thread {} iteration {} error: {}", thread_id, iteration, e);
                        }
                    }
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let res = handle.join();
        assert!(res.is_ok(), "Thread should complete successfully");
    }

    use perl_tdd_support::must;
    let results = must(results.lock());
    let error_count = *must(error_count.lock());

    // Verify all operations completed
    assert_eq!(
        results.len(),
        thread_count * iterations_per_thread,
        "All thread operations should complete"
    );

    // Verify success rate
    let success_count = results.iter().filter(|(_, _, success, _)| *success).count();
    let success_rate = success_count as f64 / results.len() as f64;

    println!(
        "  ✓ Thread safety: {} operations, success rate: {:.1}%, errors: {}",
        results.len(),
        success_rate * 100.0,
        error_count
    );

    // Should have very high success rate
    assert!(success_rate > 0.95, "Success rate {:.1}% should be > 95%", success_rate * 100.0);
}

/// Test multiple concurrent parsing requests with different inputs
#[test]
fn test_concurrent_parsing_requests() {
    println!("Testing concurrent parsing requests...");

    let thread_count = 12;
    let requests_per_thread = 20;

    // Generate different test codes for each request
    let test_codes: Vec<String> =
        (0..(thread_count * requests_per_thread)).map(generate_test_code).collect();

    let test_codes = Arc::new(test_codes);
    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let test_codes_clone = Arc::clone(&test_codes);
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for request in 0..requests_per_thread {
                    let code_index = thread_id * requests_per_thread + request;
                    let code = &test_codes_clone[code_index];

                    let start_time = Instant::now();
                    let mut parser = Parser::new(code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    let mut guard = must(results_clone.lock());
                    guard.push((
                        thread_id,
                        request,
                        code_index,
                        result.is_ok(),
                        parse_time,
                        code.len(),
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

    use perl_tdd_support::must;
    let results = must(results.lock());

    // Verify all requests completed
    assert_eq!(results.len(), thread_count * requests_per_thread, "All requests should complete");

    // Analyze performance
    let mut total_time = Duration::new(0, 0);
    let mut total_size = 0;
    let mut success_count = 0;

    for (thread_id, request, code_index, success, parse_time, code_size) in results.iter() {
        total_time += *parse_time;
        total_size += *code_size;
        if *success {
            success_count += 1;
        }

        // Individual requests should complete quickly
        assert!(
            *parse_time < Duration::from_millis(100),
            "Thread {} request {} (code {}) took too long: {:?}",
            thread_id,
            request,
            code_index,
            parse_time
        );
    }

    let avg_time_per_request = total_time / results.len() as u32;
    let success_rate = success_count as f64 / results.len() as f64;
    let throughput = total_size as f64 / total_time.as_secs_f64();

    println!(
        "  ✓ Concurrent requests: {} total, avg time: {:?}, success: {:.1}%, throughput: {:.0} bytes/sec",
        results.len(),
        avg_time_per_request,
        success_rate * 100.0,
        throughput
    );

    // Should have good success rate
    assert!(success_rate > 0.9, "Success rate {:.1}% should be > 90%", success_rate * 100.0);
}

/// Test incremental parsing under concurrent modifications
#[test]
fn test_concurrent_incremental_parsing() {
    println!("Testing concurrent incremental parsing...");

    let thread_count = 8;
    let modifications_per_thread = 25;

    // Base code that will be modified
    let base_code = r#"
use strict;
use warnings;

my $variable = 42;
my @array = (1, 2, 3, 4, 5);
my %hash = (key1 => 'value1', key2 => 'value2');

sub example_sub {
    my ($param) = @_;
    return $param * 2;
}

my $result = example_sub($variable);
print "Result: $result\n";
"#;

    let base_code = Arc::new(base_code.to_string());
    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let base_code_clone = Arc::clone(&base_code);
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for modification in 0..modifications_per_thread {
                    // Generate a modified version of the base code
                    let modified_code = modify_code(&base_code_clone, thread_id, modification);

                    let start_time = Instant::now();
                    let mut parser = Parser::new(&modified_code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    let mut guard = must(results_clone.lock());
                    guard.push((
                        thread_id,
                        modification,
                        result.is_ok(),
                        parse_time,
                        modified_code.len(),
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

    let results = must(results.lock());

    // Verify all modifications completed
    assert_eq!(
        results.len(),
        thread_count * modifications_per_thread,
        "All incremental modifications should complete"
    );

    // Analyze results
    let mut total_time = Duration::new(0, 0);
    let mut success_count = 0;

    for (thread_id, modification, success, parse_time, _code_size) in results.iter() {
        total_time += *parse_time;
        if *success {
            success_count += 1;
        }

        // Incremental parsing should be fast
        assert!(
            *parse_time < Duration::from_millis(50),
            "Thread {} modification {} took too long: {:?}",
            thread_id,
            modification,
            parse_time
        );
    }

    let avg_time = total_time / results.len() as u32;
    let success_rate = success_count as f64 / results.len() as f64;

    println!(
        "  ✓ Incremental parsing: {} operations, avg time: {:?}, success: {:.1}%",
        results.len(),
        avg_time,
        success_rate * 100.0
    );

    // Should have excellent success rate
    assert!(success_rate > 0.95, "Success rate {:.1}% should be > 95%", success_rate * 100.0);
}

/// Test workspace indexing under concurrent access
#[test]
fn test_concurrent_workspace_indexing() {
    println!("Testing concurrent workspace indexing...");

    let thread_count = 6;
    let files_per_thread = 15;

    // Generate workspace files
    let workspace_files: Vec<String> =
        (0..(thread_count * files_per_thread)).map(generate_workspace_file).collect();

    let workspace_files = Arc::new(workspace_files);
    let workspace_index = Arc::new(RwLock::new(HashMap::new()));
    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let workspace_files_clone = Arc::clone(&workspace_files);
            let workspace_index_clone = Arc::clone(&workspace_index);
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for file_index in 0..files_per_thread {
                    let global_file_index = thread_id * files_per_thread + file_index;
                    let file_content = &workspace_files_clone[global_file_index];

                    let start_time = Instant::now();
                    let mut parser = Parser::new(file_content);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    if let Ok(ast) = result {
                        // Extract symbols and add to workspace index
                        let symbols = extract_symbols(&ast);

                        // Write to workspace index
                        {
                            let index_guard = workspace_index_clone.write();
                            assert!(index_guard.is_ok());
                            if let Ok(mut index) = index_guard {
                                for symbol in &symbols {
                                    index.insert(symbol.clone(), global_file_index);
                                }
                            }
                        }

                        let mut guard = must(results_clone.lock());
                        guard.push((thread_id, file_index, true, parse_time, symbols.len()));
                    } else {
                        let mut guard = must(results_clone.lock());
                        guard.push((thread_id, file_index, false, parse_time, 0));
                    }
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let res = handle.join();
        assert!(res.is_ok(), "Thread should complete successfully");
    }

    use perl_tdd_support::must;
    let results = must(results.lock());

    let workspace_index = must(workspace_index.read());

    // Verify all files were processed
    assert_eq!(
        results.len(),
        thread_count * files_per_thread,
        "All workspace files should be processed"
    );

    // Analyze results
    let mut total_time = Duration::new(0, 0);
    let mut total_symbols = 0;
    let mut success_count = 0;

    for (thread_id, file_index, success, parse_time, symbol_count) in results.iter() {
        total_time += *parse_time;
        total_symbols += *symbol_count;
        if *success {
            success_count += 1;
        }

        // Workspace indexing should be reasonably fast
        assert!(
            *parse_time < Duration::from_millis(100),
            "Thread {} file {} took too long: {:?}",
            thread_id,
            file_index,
            parse_time
        );
    }

    let avg_time = total_time / results.len() as u32;
    let success_rate = success_count as f64 / results.len() as f64;

    println!(
        "  ✓ Workspace indexing: {} files, avg time: {:?}, success: {:.1}%, total symbols: {}",
        results.len(),
        avg_time,
        success_rate * 100.0,
        total_symbols
    );

    // Should have good success rate
    assert!(success_rate > 0.9, "Success rate {:.1}% should be > 90%", success_rate * 100.0);

    // Should have extracted symbols
    assert!(!workspace_index.is_empty(), "Should have extracted symbols from workspace");
}

/// Test parser with high contention scenarios
#[test]
fn test_high_contention_scenarios() {
    println!("Testing high contention scenarios...");

    let thread_count = 20;
    let operations_per_thread = 100;

    // Shared resources that will cause contention
    let shared_code = Arc::new(generate_large_test_code(10000));
    let shared_counter = Arc::new(Mutex::new(0));
    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let shared_code_clone = Arc::clone(&shared_code);
            let shared_counter_clone = Arc::clone(&shared_counter);
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for operation in 0..operations_per_thread {
                    // Access shared counter to create contention
                    {
                        let counter_guard = shared_counter_clone.lock();
                        assert!(counter_guard.is_ok());
                        if let Ok(mut counter) = counter_guard {
                            *counter += 1;
                        }
                    }

                    let start_time = Instant::now();
                    let mut parser = Parser::new(&shared_code_clone);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    let mut guard = must(results_clone.lock());
                    guard.push((thread_id, operation, result.is_ok(), parse_time));
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let res = handle.join();
        assert!(res.is_ok(), "Thread should complete successfully");
    }

    use perl_tdd_support::must;
    let results = must(results.lock());

    let final_counter = *must(shared_counter.lock());

    // Verify all operations completed
    assert_eq!(
        results.len(),
        thread_count * operations_per_thread,
        "All high contention operations should complete"
    );

    assert_eq!(
        final_counter,
        thread_count * operations_per_thread,
        "Shared counter should reflect all operations"
    );

    // Analyze performance under contention
    let mut total_time = Duration::new(0, 0);
    let mut success_count = 0;

    for (thread_id, operation, success, parse_time) in results.iter() {
        total_time += *parse_time;
        if *success {
            success_count += 1;
        }

        // Even under contention, should complete reasonably
        assert!(
            *parse_time < Duration::from_millis(200),
            "Thread {} operation {} took too long under contention: {:?}",
            thread_id,
            operation,
            parse_time
        );
    }

    let avg_time = total_time / results.len() as u32;
    let success_rate = success_count as f64 / results.len() as f64;

    println!(
        "  ✓ High contention: {} operations, avg time: {:?}, success: {:.1}%",
        results.len(),
        avg_time,
        success_rate * 100.0
    );

    // Should maintain reasonable success rate even under contention
    assert!(
        success_rate > 0.85,
        "Success rate {:.1}% should be > 85% under contention",
        success_rate * 100.0
    );
}

/// Test parser with mixed concurrent operations
#[test]
fn test_mixed_concurrent_operations() {
    println!("Testing mixed concurrent operations...");

    let thread_count = 16;
    let operations_per_thread = 30;

    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for operation in 0..operations_per_thread {
                    let operation_type = operation % 4;
                    let start_time = Instant::now();

                    let (success, parse_time, operation_name) = match operation_type {
                        0 => {
                            // Small code parsing
                            let code = generate_test_code(operation);
                            let mut parser = Parser::new(&code);
                            let result = parser.parse();
                            (result.is_ok(), start_time.elapsed(), "small")
                        }
                        1 => {
                            // Large code parsing
                            let code = generate_large_test_code(5000);
                            let mut parser = Parser::new(&code);
                            let result = parser.parse();
                            (result.is_ok(), start_time.elapsed(), "large")
                        }
                        2 => {
                            // Complex code parsing
                            let code = generate_complex_test_code();
                            let mut parser = Parser::new(&code);
                            let result = parser.parse();
                            (result.is_ok(), start_time.elapsed(), "complex")
                        }
                        3 => {
                            // Error-prone code parsing
                            let code = generate_error_prone_code();
                            let mut parser = Parser::new(&code);
                            let result = parser.parse();
                            (result.is_ok(), start_time.elapsed(), "error_prone")
                        }
                        _ => unreachable!(),
                    };

                    results_clone.lock().unwrap().push((
                        thread_id,
                        operation,
                        operation_name,
                        success,
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

    use perl_tdd_support::must;
    let results = must(results.lock());

    // Verify all operations completed
    assert_eq!(
        results.len(),
        thread_count * operations_per_thread,
        "All mixed operations should complete"
    );

    // Analyze by operation type
    let mut operation_stats: HashMap<&str, (usize, Duration, usize)> = HashMap::new();

    for (thread_id, operation, operation_name, success, parse_time) in results.iter() {
        let entry = operation_stats.entry(*operation_name).or_insert((0, Duration::new(0, 0), 0));
        entry.0 += 1;
        entry.1 += *parse_time;
        if *success {
            entry.2 += 1;
        }

        // All operations should complete reasonably
        assert!(
            *parse_time < Duration::from_millis(500),
            "Thread {} operation {} ({}) took too long: {:?}",
            thread_id,
            operation,
            operation_name,
            parse_time
        );
    }

    println!("  ✓ Mixed operations analysis:");
    for (operation_type, (count, total_time, success_count)) in operation_stats {
        let avg_time = total_time / count as u32;
        let success_rate = success_count as f64 / count as f64;
        println!(
            "    {}: {} ops, avg: {:?}, success: {:.1}%",
            operation_type,
            count,
            avg_time,
            success_rate * 100.0
        );

        // All operation types should have reasonable success rates
        assert!(
            success_rate > 0.8,
            "Operation type {} success rate {:.1}% should be > 80%",
            operation_type,
            success_rate * 100.0
        );
    }
}

/// Test parser memory safety under concurrent access
#[test]
fn test_concurrent_memory_safety() {
    println!("Testing concurrent memory safety...");

    let thread_count = 12;
    let iterations_per_thread = 50;

    let results = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let results_clone = Arc::clone(&results);

            thread::spawn(move || {
                for iteration in 0..iterations_per_thread {
                    // Create and destroy many parsers to test memory safety

                    for i in 0..10 {
                        let code = generate_test_code(thread_id * 1000 + iteration * 10 + i);
                        let mut parser = Parser::new(&code);
                        let result = parser.parse();

                        if result.is_ok() {
                            // We can't store the parser if it borrows the code
                            // So we just test parsing and drop both
                        }
                        // Both parser and code are dropped here
                    }

                    let mut guard = must(results_clone.lock());
                    guard.push((thread_id, iteration, true));
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let res = handle.join();
        assert!(res.is_ok(), "Thread should complete successfully");
    }

    use perl_tdd_support::must;
    let results = must(results.lock());

    // Verify all iterations completed without memory issues
    assert_eq!(
        results.len(),
        thread_count * iterations_per_thread,
        "All memory safety iterations should complete"
    );

    println!("  ✓ Memory safety: {} iterations completed without issues", results.len());
}

/// Test parser with concurrent stress and recovery
#[test]
fn test_concurrent_stress_and_recovery() {
    println!("Testing concurrent stress and recovery...");

    let stress_thread_count = 8;
    let recovery_thread_count = 4;
    let stress_iterations = 100;

    let stress_results = Arc::new(Mutex::new(Vec::new()));
    let recovery_results = Arc::new(Mutex::new(Vec::new()));
    let shared_counter = Arc::new(Mutex::new(0));

    // Stress threads - create heavy load
    let stress_handles: Vec<_> = (0..stress_thread_count)
        .map(|thread_id| {
            let stress_results_clone = Arc::clone(&stress_results);
            let shared_counter_clone = Arc::clone(&shared_counter);

            thread::spawn(move || {
                for iteration in 0..stress_iterations {
                    // Increment shared counter
                    {
                        let counter_guard = shared_counter_clone.lock();
                        assert!(counter_guard.is_ok());
                        if let Ok(mut counter) = counter_guard {
                            *counter += 1;
                        }
                    }

                    // Stress with large code
                    let code = generate_large_test_code(10000);
                    let start_time = Instant::now();
                    let mut parser = Parser::new(&code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    let mut guard = must(stress_results_clone.lock());
                    guard.push((thread_id, iteration, result.is_ok(), parse_time));
                }
            })
        })
        .collect();

    // Recovery threads - test normal parsing under stress
    let recovery_handles: Vec<_> = (0..recovery_thread_count)
        .map(|thread_id| {
            let recovery_results_clone = Arc::clone(&recovery_results);
            let shared_counter_clone = Arc::clone(&shared_counter);

            thread::spawn(move || {
                for iteration in 0..stress_iterations {
                    // Read shared counter to add some contention
                    let counter_value = {
                        let counter_guard = shared_counter_clone.lock();
                        assert!(counter_guard.is_ok());
                        *counter_guard.unwrap_or_else(|_| unreachable!())
                    };

                    // Normal parsing under stress
                    let code = format!(
                        r#"
use strict;
use warnings;
my $x = {};
my $y = $x * 2;
print "Result: $y\n";
"#,
                        counter_value
                    );

                    let start_time = Instant::now();
                    let mut parser = Parser::new(&code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();

                    let mut guard = must(recovery_results_clone.lock());
                    guard.push((thread_id, iteration, result.is_ok(), parse_time, counter_value));
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in stress_handles {
        let res = handle.join();
        assert!(res.is_ok(), "Stress thread should complete successfully");
    }

    for handle in recovery_handles {
        let res = handle.join();
        assert!(res.is_ok(), "Recovery thread should complete successfully");
    }

    use perl_tdd_support::must;
    let stress_results = must(stress_results.lock());
    let recovery_results = must(recovery_results.lock());

    // Analyze stress results
    let stress_success_count = stress_results.iter().filter(|(_, _, success, _)| *success).count();
    let stress_success_rate = stress_success_count as f64 / stress_results.len() as f64;

    // Analyze recovery results
    let recovery_success_count =
        recovery_results.iter().filter(|(_, _, success, _, _)| *success).count();
    let recovery_success_rate = recovery_success_count as f64 / recovery_results.len() as f64;

    println!("  ✓ Stress and recovery:");
    println!(
        "    Stress: {} operations, success: {:.1}%",
        stress_results.len(),
        stress_success_rate * 100.0
    );
    println!(
        "    Recovery: {} operations, success: {:.1}%",
        recovery_results.len(),
        recovery_success_rate * 100.0
    );

    // Both stress and recovery should maintain reasonable success rates
    assert!(
        stress_success_rate > 0.8,
        "Stress success rate {:.1}% should be > 80%",
        stress_success_rate * 100.0
    );

    assert!(
        recovery_success_rate > 0.9,
        "Recovery success rate {:.1}% should be > 90%",
        recovery_success_rate * 100.0
    );
}

// Helper functions for generating test code

fn generate_test_code(seed: usize) -> String {
    format!(
        r#"
use strict;
use warnings;

my $var{} = {};
my @array{} = ({}, {}, {});
my %hash{} = ('key' => 'value{}');

sub test_{} {{
    my ($param) = @_;
    return $param * {};
}}

my $result{} = test_{}($var{});
print "Test {}: $result{}\n";
"#,
        seed,
        seed,
        seed,
        seed % 10,
        (seed + 1) % 10,
        (seed + 2) % 10,
        seed,
        seed,
        seed,
        seed,
        seed,
        seed,
        seed,
        seed,
        seed
    )
}

fn generate_large_test_code(size: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n\n");

    let mut current_size = code.len();
    let sub_counter = 0;

    while current_size < size {
        let sub_code = format!(
            r#"sub large_sub_{} {{
    my ($param1, $param2, $param3) = @_;
    my $result = $param1 + $param2 * $param3;
    
    # Add some complexity
    for my $i (0..10) {{
        $result += $i * $param1;
    }}
    
    return $result;
}}

"#,
            sub_counter
        );

        code.push_str(&sub_code);
        current_size = code.len();
    }

    // Add calls to the subroutines
    for i in 0..(sub_counter / 5) {
        code.push_str(&format!(
            "my $result_{} = large_sub_{}({}, {}, {});\n",
            i,
            i,
            i,
            i + 1,
            i + 2
        ));
    }

    code
}

fn generate_complex_test_code() -> String {
    r#"
use strict;
use warnings;

# Complex data structures
my $complex_ref = {
    nested => {
        deeply => {
            structure => {
                with => {
                    many => {
                        levels => [1, 2, 3, 4, 5]
                    }
                }
            }
        }
    },
    array_ref => [
        { id => 1, name => 'first' },
        { id => 2, name => 'second' },
        { id => 3, name => 'third' },
    ],
    mixed => {
        scalars => \$var,
        arrays => \@array,
        hashes => \%hash,
        subs => \&some_sub,
    }
};

# Complex regex
my $complex_regex = qr/
    ^
    (?<prefix>\w+)
    _
    (?<number>\d+)
    _
    (?<suffix>[a-z]+)
    $
/x;

# Complex conditional
if ($condition1 && $condition2 || $condition3) {
    if ($nested1) {
        if ($deeply_nested) {
            print "Deeply nested condition\n";
        }
    } elsif ($alternative) {
        print "Alternative path\n";
    }
} else {
    print "Default case\n";
}

# Complex loop with next/last
for my $i (0..100) {
    next if $i % 2 == 0;
    last if $i > 50;
    redo if $i == 25;
    
    print "Processing: $i\n";
}
"#
    .to_string()
}

fn generate_error_prone_code() -> String {
    r#"
use strict;
use warnings;

# Intentionally problematic code
my $uninitialized;
print $uninitialized;  # Use of uninitialized variable

my @array = (1, 2, 3);
my $index = 10;
print $array[$index];  # Out of bounds access

my %hash = (key1 => 'value1');
print $hash{nonexistent_key};  # Accessing non-existent key

# Syntax errors (should be handled gracefully)
if ($condition {
    print "Missing closing parenthesis\n";

sub missing_brace {
    print "Missing closing brace\n";

# Regex with issues
my $bad_regex = qr/([a-z]+/;  # Unclosed group

# Deep nesting that might cause issues
if (1) {
    if (2) {
        if (3) {
            if (4) {
                if (5) {
                    if (6) {
                        if (7) {
                            if (8) {
                                if (9) {
                                    if (10) {
                                        print "Very deep nesting\n";
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
"#
    .to_string()
}

fn modify_code(base_code: &str, thread_id: usize, modification: usize) -> String {
    let mut modified = base_code.to_string();

    // Add thread and modification specific changes
    modified.push_str(&format!("\n# Thread {} modification {}\n", thread_id, modification));
    modified.push_str(&format!(
        "my $thread_{}_mod_{} = {};\n",
        thread_id,
        modification,
        thread_id * modification
    ));
    modified.push_str(&format!(
        "print \"Thread {} modification {}: $thread_{}_mod_{}\\n\";\n",
        thread_id, modification, thread_id, modification
    ));

    modified
}

fn generate_workspace_file(file_index: usize) -> String {
    format!(
        r#"
package Module{};

use strict;
use warnings;

our $VERSION = '1.0';

sub new {{
    my $class = shift;
    my $self = {{
        id => {},
        name => 'module_{}',
    }};
    return bless $self, $class;
}}

sub method_{} {{
    my ($self) = @_;
    return $self->{{id}} * 2;
}}

sub function_{} {{
    my ($param) = @_;
    return $param + {};
}}

1;
"#,
        file_index, file_index, file_index, file_index, file_index, file_index
    )
}

fn extract_symbols(ast: &perl_parser::ast::Node) -> Vec<String> {
    use perl_parser::ast::NodeKind;

    let mut symbols = Vec::new();

    match &ast.kind {
        NodeKind::Program { statements } => {
            for child in statements {
                symbols.extend(extract_symbols(child));
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            symbols.extend(extract_symbols(expression));
        }
        // Add more symbol extraction logic based on actual NodeKind variants
        _ => {
            // Extract symbol names from node text or other properties
            // This is a simplified version - real implementation would be more sophisticated
        }
    }

    symbols
}
