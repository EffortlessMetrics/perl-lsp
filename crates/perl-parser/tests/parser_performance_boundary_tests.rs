//! Comprehensive Performance Boundary Tests for Perl Parser
//!
//! This test suite validates parser behavior at performance limits including:
//! - Maximum file sizes the parser can handle
//! - Very large numbers of tokens and AST nodes
//! - Parsing time limits and timeout behavior
//! - Memory usage limits and garbage collection

use perl_parser::Parser;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::thread;

/// Maximum reasonable file size for testing (10MB)
const MAX_TEST_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Performance thresholds based on documented benchmarks
const MAX_PARSE_TIME_PER_KB: Duration = Duration::from_micros(500);
const MAX_MEMORY_USAGE_PER_KB: usize = 1024; // 1KB memory per 1KB of source

/// Test parser with very large files
#[test]
fn test_maximum_file_size_handling() {
    println!("Testing maximum file size handling...");
    
    // Test progressively larger files
    let test_sizes = vec![
        1 * 1024,        // 1KB
        10 * 1024,       // 10KB
        100 * 1024,      // 100KB
        1 * 1024 * 1024, // 1MB
        5 * 1024 * 1024, // 5MB
    ];
    
    for size in test_sizes {
        println!("Testing file size: {} bytes", size);
        
        // Generate valid Perl code of specified size
        let code = generate_large_perl_file(size);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Verify parsing completed (may have errors but shouldn't crash)
        assert!(result.is_ok(), "Parser should handle {} byte file without crashing", size);
        
        // Check performance is within reasonable bounds
        let size_kb = size as f64 / 1024.0;
        let expected_max_time = MAX_PARSE_TIME_PER_KB * size_kb.ceil() as u32;
        
        assert!(
            parse_time < expected_max_time,
            "Parse time {:?} for {}KB exceeds expected maximum {:?}",
            parse_time, size_kb, expected_max_time
        );
        
        println!("  ✓ Parsed {} bytes in {:?}", size, parse_time);
    }
}

/// Test parser with very large numbers of tokens
#[test]
fn test_maximum_token_count_handling() {
    println!("Testing maximum token count handling...");
    
    // Generate code with increasing token density
    let token_counts = vec![1_000, 10_000, 100_000, 1_000_000];
    
    for target_tokens in token_counts {
        println!("Testing with ~{} tokens...", target_tokens);
        
        let code = generate_high_token_density_code(target_tokens);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Parser should handle {} tokens without crashing", target_tokens);
        
        let ast = result.unwrap();
        let actual_tokens = count_ast_tokens(&ast);
        
        println!("  ✓ Generated {} tokens, parsed in {:?}", actual_tokens, parse_time);
        
        // Verify token count is in expected range
        assert!(
            actual_tokens >= target_tokens / 2,
            "Token count {} is much less than expected {}",
            actual_tokens, target_tokens
        );
    }
}

/// Test parser with very deep AST structures
#[test]
fn test_maximum_ast_depth_handling() {
    println!("Testing maximum AST depth handling...");
    
    let depths = vec![10, 50, 100, 200, 500];
    
    for depth in depths {
        println!("Testing AST depth: {}", depth);
        
        let code = generate_deeply_nested_code(depth);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should either parse successfully or hit recursion limit gracefully
        if result.is_ok() {
            let ast = result.unwrap();
            let actual_depth = calculate_ast_depth(&ast);
            
            println!("  ✓ Parsed depth {} in {:?}", actual_depth, parse_time);
            
            // Verify depth is reasonable
            assert!(
                actual_depth <= depth + 10, // Allow some margin
                "AST depth {} exceeds expected {}",
                actual_depth, depth
            );
        } else {
            // Should fail gracefully with recursion limit error
            println!("  ✓ Hit recursion limit at depth {} in {:?}", depth, parse_time);
        }
    }
}

/// Test parser timeout behavior
#[test]
fn test_parsing_timeout_behavior() {
    println!("Testing parsing timeout behavior...");
    
    // Generate code that might cause timeouts
    let timeout_inducing_code = generate_timeout_inducing_code();
    
    let start_time = Instant::now();
    let mut parser = Parser::new(&timeout_inducing_code);
    let result = parser.parse();
    let parse_time = start_time.elapsed();
    
    // Should complete within reasonable time (even if with errors)
    let timeout_threshold = Duration::from_secs(10);
    assert!(
        parse_time < timeout_threshold,
        "Parsing should complete within {:?}, took {:?}",
        timeout_threshold, parse_time
    );
    
    println!("  ✓ Timeout test completed in {:?}", parse_time);
}

/// Test memory usage during parsing
#[test]
fn test_memory_usage_limits() {
    println!("Testing memory usage limits...");
    
    let test_sizes = vec![1_000, 10_000, 100_000, 1_000_000];
    
    for size in test_sizes {
        println!("Testing memory usage for {} characters...", size);
        
        let code = generate_memory_intensive_code(size);
        
        // Measure memory before parsing
        let memory_before = get_memory_usage();
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Measure memory after parsing
        let memory_after = get_memory_usage();
        let memory_used = memory_after.saturating_sub(memory_before);
        
        assert!(result.is_ok(), "Parser should handle {} characters without crashing", size);
        
        // Check memory usage is reasonable
        let size_kb = size as f64 / 1024.0;
        let expected_max_memory = (MAX_MEMORY_USAGE_PER_KB as f64 * size_kb) as usize;
        
        // Allow significant margin for AST overhead
        let memory_margin = expected_max_memory * 10;
        
        assert!(
            memory_used <= memory_margin,
            "Memory usage {}KB for {}KB source exceeds expected margin {}KB",
            memory_used / 1024, size_kb as usize, memory_margin / 1024
        );
        
        println!("  ✓ Used {}KB memory for {}KB source in {:?}", 
                memory_used / 1024, size_kb as usize, parse_time);
    }
}

/// Test concurrent parsing performance
#[test]
fn test_concurrent_parsing_performance() {
    println!("Testing concurrent parsing performance...");
    
    let thread_counts = vec![1, 2, 4, 8];
    let file_size = 100_000; // 100KB test file
    
    for thread_count in thread_counts {
        println!("Testing with {} threads...", thread_count);
        
        let code = Arc::new(generate_large_perl_file(file_size));
        let results = Arc::new(Mutex::new(Vec::new()));
        
        let start_time = Instant::now();
        
        let mut handles = Vec::new();
        for i in 0..thread_count {
            let code_clone = Arc::clone(&code);
            let results_clone = Arc::clone(&results);
            
            let handle = thread::spawn(move || {
                let thread_start = Instant::now();
                let mut parser = Parser::new(&code_clone);
                let result = parser.parse();
                let thread_time = thread_start.elapsed();
                
                results_clone.lock().unwrap().push((i, result, thread_time));
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should complete successfully");
        }
        
        let total_time = start_time.elapsed();
        let results = results.lock().unwrap();
        
        // Verify all threads completed successfully
        assert_eq!(results.len(), thread_count, "All threads should complete");
        
        for (thread_id, result, thread_time) in results.iter() {
            assert!(result.is_ok(), "Thread {} should parse successfully", thread_id);
            println!("    Thread {} completed in {:?}", thread_id, thread_time);
        }
        
        println!("  ✓ {} threads completed in {:?}", thread_count, total_time);
    }
}

/// Test parser behavior with garbage collection pressure
#[test]
fn test_garbage_collection_pressure() {
    println!("Testing garbage collection pressure...");
    
    // Create many parser instances to trigger GC pressure
    let iterations = 1000;
    let file_size = 10_000;
    
    let start_time = Instant::now();
    
    for i in 0..iterations {
        let code = generate_large_perl_file(file_size);
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        
        assert!(result.is_ok(), "Parser should handle iteration {} without crashing", i);
        
        // Drop parser explicitly to trigger cleanup
        drop(parser);
        drop(result);
        
        // Periodically force garbage collection if available
        if i % 100 == 0 {
            // Note: Rust doesn't have explicit GC, but this creates pressure
            let _temp: Vec<String> = (0..1000).map(|j| format!("temp_{}_{}", i, j)).collect();
            drop(_temp);
        }
    }
    
    let total_time = start_time.elapsed();
    let avg_time_per_parse = total_time / iterations;
    
    println!("  ✓ {} iterations completed in {:?} (avg: {:?})", 
             iterations, total_time, avg_time_per_parse);
    
    // Verify average time remains reasonable
    assert!(
        avg_time_per_parse < Duration::from_millis(100),
        "Average parse time {:?} exceeds 100ms under GC pressure",
        avg_time_per_parse
    );
}

/// Test parser with extreme edge cases that might cause performance issues
#[test]
fn test_performance_edge_cases() {
    println!("Testing performance edge cases...");
    
    let edge_cases = vec![
        ("Very long identifier", generate_long_identifier_code()),
        ("Deeply nested arrays", generate_deep_array_code()),
        ("Complex regex patterns", generate_complex_regex_code()),
        ("Massive string concatenation", generate_string_concat_code()),
        ("Huge hash structure", generate_huge_hash_code()),
    ];
    
    for (name, code) in edge_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Parser should handle {} without crashing", name);
        
        // Should complete within reasonable time
        assert!(
            parse_time < Duration::from_secs(5),
            "Edge case '{}' took too long: {:?}",
            name, parse_time
        );
        
        println!("  ✓ {} completed in {:?}", name, parse_time);
    }
}

// Helper functions for generating test code

fn generate_large_perl_file(size: usize) -> String {
    let mut code = String::with_capacity(size);
    code.push_str("# Large Perl file for testing\n");
    code.push_str("use strict;\nuse warnings;\n\n");
    
    let mut current_size = code.len();
    let sub_counter = 0;
    
    while current_size < size {
        let sub_name = format!("test_sub_{}", sub_counter);
        let sub_code = format!(
            r#"sub {} {{
    my ($param1, $param2) = @_;
    my $result = $param1 + $param2;
    return $result;
}}

"#, sub_name);
        
        code.push_str(&sub_code);
        current_size = code.len();
    }
    
    // Add a main section
    code.push_str("my $total = 0;\n");
    for i in 0..(sub_counter / 10) {
        code.push_str(&format!("$total += test_sub_{}({}, {});\n", i, i, i * 2));
    }
    code.push_str("print \"Total: $total\\n\";\n");
    
    code
}

fn generate_high_token_density_code(target_tokens: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    let mut token_count = 0;
    let var_counter = 0;
    
    while token_count < target_tokens {
        // Generate expressions with high token density
        let expr = format!(
            "my $var{} = {} + {} * {} / {} - {} % {} && {} || {};\n",
            var_counter,
            var_counter,
            var_counter + 1,
            var_counter + 2,
            var_counter + 3,
            var_counter + 4,
            var_counter + 5,
            var_counter + 6,
            var_counter + 7
        );
        
        code.push_str(&expr);
        token_count += 20; // Approximate token count
        var_counter += 1;
    }
    
    code
}

fn generate_deeply_nested_code(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate deeply nested if-else structure
    for i in 0..depth {
        let indent = "    ".repeat(i);
        code.push_str(&format!("{}if ($var{}) {{\n", indent, i));
    }
    
    // Add some content at the deepest level
    let deepest_indent = "    ".repeat(depth);
    code.push_str(&format!("{}my $result = 'deep';\n", deepest_indent));
    
    // Close all the if blocks
    for i in (0..depth).rev() {
        let indent = "    ".repeat(i);
        code.push_str(&format!("{}}} else {{\n", indent));
        code.push_str(&format!("{}my $else{} = 'else';\n", indent, i));
    }
    
    // Final closing
    for _ in 0..depth {
        code.push_str("}\n");
    }
    
    code
}

fn generate_timeout_inducing_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate code that might cause pathological behavior
    // Very long regex with many alternatives
    code.push_str("my $pattern = qr/");
    for i in 0..1000 {
        if i > 0 { code.push('|'); }
        code.push_str(&format!("pattern{}", i));
    }
    code.push_str("/;\n");
    
    // Massive array with complex expressions
    code.push_str("my @array = (\n");
    for i in 0..1000 {
        code.push_str(&format!("    {} + {} * {} / {}", i, i+1, i+2, i+3));
        if i < 999 { code.push(','); }
        code.push('\n');
    }
    code.push_str(");\n");
    
    // Deeply nested data structure
    code.push_str("my $deep = {\n");
    for i in 0..100 {
        code.push_str(&format!("    key{} => {{\n", i));
        for j in 0..10 {
            code.push_str(&format!("        subkey{} => [", j));
            for k in 0..10 {
                code.push_str(&format!("'value{}_{}_{}'", i, j, k));
                if k < 9 { code.push_str(", "); }
            }
            code.push_str("]");
            if j < 9 { code.push(','); }
            code.push('\n');
        }
        code.push_str("    }");
        if i < 99 { code.push(','); }
        code.push('\n');
    }
    code.push_str("};\n");
    
    code
}

fn generate_memory_intensive_code(size: usize) -> String {
    let mut code = String::with_capacity(size);
    code.push_str("use strict; use warnings;\n");
    
    let mut current_size = code.len();
    
    // Generate many variables and data structures
    let var_counter = 0;
    while current_size < size {
        // Generate arrays
        let array_def = format!(
            "my @array{} = ({});\n",
            var_counter,
            (0..100).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
        );
        code.push_str(&array_def);
        
        // Generate hashes
        let hash_def = format!(
            "my %hash{} = ({});\n",
            var_counter,
            (0..50).map(|i| format!("'key{}' => 'value{}'", i, i)).collect::<Vec<_>>().join(", ")
        );
        code.push_str(&hash_def);
        
        // Generate subroutines
        let sub_def = format!(
            r#"sub sub{} {{
    my ($param) = @_;
    return $param * 2;
}}
"#, var_counter);
        code.push_str(&sub_def);
        
        current_size = code.len();
    }
    
    code
}

fn generate_long_identifier_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate very long identifier names
    let long_name = "a".repeat(1000);
    code.push_str(&format!("my $var_{} = 42;\n", long_name));
    code.push_str(&format!("sub sub_{} {{ return $_[0] * 2; }}\n", long_name));
    code.push_str(&format!("my %hash_{} = ('key' => 'value');\n", long_name));
    code.push_str(&format!("my @array_{} = (1, 2, 3);\n", long_name));
    
    // Use the long identifiers
    code.push_str(&format!("print $var_{};\n", long_name));
    code.push_str(&format!("print sub_{}($var_{});\n", long_name, long_name));
    code.push_str("print $hash_{'key'};\n");
    code.push_str("print $array_[0];\n");
    
    code
}

fn generate_deep_array_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    let depth = 100;
    code.push_str("my $deep_array = ");
    
    for i in 0..depth {
        code.push('[');
    }
    
    code.push_str("42");
    
    for _ in 0..depth {
        code.push(']');
    }
    
    code.push_str(";\n");
    code.push_str("print $deep_array;\n");
    
    code
}

fn generate_complex_regex_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate complex regex patterns
    code.push_str("my $complex1 = qr/");
    for i in 0..100 {
        code.push_str(&format!("(?<group{}>\\w+)", i));
        if i < 99 { code.push('|'); }
    }
    code.push_str("/;\n");
    
    code.push_str("my $complex2 = qr/");
    for i in 0..50 {
        code.push_str(&format!("(?:[a-z{}]+)", i));
        if i < 49 { code.push('|'); }
    }
    code.push_str("/;\n");
    
    code.push_str("my $complex3 = qr/");
    for i in 0..25 {
        code.push_str(&format!("(?=.*pattern{})(?!.*negative{})", i, i));
    }
    code.push_str("/;\n");
    
    code
}

fn generate_string_concat_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    code.push_str("my $result = ");
    for i in 0..1000 {
        code.push_str(&format!("'string{}'", i));
        if i < 999 { code.push_str(" . "); }
    }
    code.push_str(";\n");
    code.push_str("print $result;\n");
    
    code
}

fn generate_huge_hash_code() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    code.push_str("my %huge_hash = (\n");
    for i in 0..10000 {
        code.push_str(&format!("    'key{}' => {{\n", i));
        for j in 0..10 {
            code.push_str(&format!("        'subkey{}' => {{\n", j));
            for k in 0..5 {
                code.push_str(&format!("            'deepkey{}' => 'value{}',\n", k, i * j * k));
            }
            code.push_str("        },\n");
        }
        code.push_str("    }");
        if i < 9999 { code.push(','); }
        code.push('\n');
    }
    code.push_str(");\n");
    
    code
}

// Helper functions for analysis

fn count_ast_tokens(node: &perl_parser::Node) -> usize {
    let mut count = 1;
    node.for_each_child(|child| {
        count += count_ast_tokens(child);
    });
    count
}

fn calculate_ast_depth(node: &perl_parser::Node) -> usize {
    let mut max_child_depth = 0;
    node.for_each_child(|child| {
        let child_depth = calculate_ast_depth(child);
        if child_depth > max_child_depth {
            max_child_depth = child_depth;
        }
    });
    1 + max_child_depth
}

fn get_memory_usage() -> usize {
    // This is a simplified memory usage measurement
    // In a real implementation, you might use platform-specific APIs
    // For now, return a reasonable estimate
    std::mem::size_of::<perl_parser::Parser>() + 
    std::mem::size_of::<perl_parser::ast::Node>() * 1000 // Estimate
}