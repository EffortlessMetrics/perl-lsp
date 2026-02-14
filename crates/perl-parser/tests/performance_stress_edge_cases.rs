//! Performance Stress Edge Cases for Perl Parser
//!
//! This test suite validates parser performance under extreme stress conditions,
//! including massive data structures, pathological patterns, resource exhaustion,
//! and concurrent parsing scenarios.

use perl_parser::Parser;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Maximum parsing time for performance tests (in seconds)
const MAX_PERFORMANCE_PARSE_TIME: Duration = Duration::from_secs(60);

/// Maximum memory usage threshold (in MB)
const MAX_MEMORY_THRESHOLD_MB: usize = 1000;

/// Test massive data structures
#[test]
fn test_massive_data_structures() {
    println!("Testing massive data structures...");
    
    let test_cases = vec![
        ("1M element array", generate_massive_array(1_000_000)),
        ("5M element array", generate_massive_array(5_000_000)),
        ("10M element array", generate_massive_array(10_000_000)),
        ("1M element hash", generate_massive_hash(1_000_000)),
        ("5M element hash", generate_massive_hash(5_000_000)),
        ("10K deep nested structure", generate_deep_nested_structure(10_000)),
        ("50K deep nested structure", generate_deep_nested_structure(50_000)),
        ("100K deep nested structure", generate_deep_nested_structure(100_000)),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {} (size: {} bytes)", name, code.len());
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the structure is present in the AST
                let sexp = ast.to_sexp();
                if name.contains("array") {
                    assert!(sexp.contains("array") || sexp.contains("list"), 
                           "Array not found in AST for {}", name);
                } else if name.contains("hash") {
                    assert!(sexp.contains("hash") || sexp.contains("pair"), 
                           "Hash not found in AST for {}", name);
                } else if name.contains("nested") {
                    assert!(!sexp.is_empty(), "AST should not be empty for nested structure {}", name);
                }
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For massive structures, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test pathological regex patterns
#[test]
fn test_pathological_regex_patterns() {
    println!("Testing pathological regex patterns...");
    
    let test_cases = vec![
        // Catastrophic backtracking patterns
        ("Catastrophic backtracking 1".to_string(), 
         r#"my $text = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; 
           my $pattern = /^(a+)+b$/; 
           if ($text =~ $pattern) { print "Match\n"; }"#.to_string()),
        
        ("Catastrophic backtracking 2".to_string(), 
         r#"my $text = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; 
           my $pattern = /^(a+)*a$/; 
           if ($text =~ $pattern) { print "Match\n"; }"#.to_string()),
        
        ("Nested quantifiers".to_string(), 
         r#"my $text = "abcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabc"; 
           my $pattern = /^(a.*)+b$/; 
           if ($text =~ $pattern) { print "Match\n"; }"#.to_string()),
        
        // Excessive alternation
        ("1000 alternatives".to_string(), generate_regex_alternations(1000)),
        ("5000 alternatives".to_string(), generate_regex_alternations(5000)),
        ("10000 alternatives".to_string(), generate_regex_alternations(10000)),
        
        // Complex character classes
        ("Huge character class".to_string(), generate_huge_character_class()),
        ("Nested character classes".to_string(), generate_nested_character_classes()),
        
        // Complex lookarounds
        ("Complex lookaheads".to_string(), generate_complex_lookaheads()),
        ("Complex lookbehinds".to_string(), generate_complex_lookbehinds()),
        ("Nested lookarounds".to_string(), generate_nested_lookarounds()),
        
        // Recursive patterns
        ("Deep recursion".to_string(), generate_recursive_regex(100)),
        ("Mutual recursion".to_string(), generate_mutual_recursive_regex()),
        
        // Unicode complexity
        ("Massive Unicode class".to_string(), generate_massive_unicode_class()),
        ("Complex Unicode properties".to_string(), generate_complex_unicode_properties()),
        
        // Backreference hell
        ("Many backreferences".to_string(), generate_many_backreferences(100)),
        ("Nested backreferences".to_string(), generate_nested_backreferences()),
    ];
    
    for (name, code) in &test_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the regex is present in the AST
                let sexp = ast.to_sexp();
                assert!(sexp.contains("regex") || sexp.contains("pattern") || sexp.contains("match"), 
                       "Regex not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Pathological regex might cause issues, but should fail gracefully
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test extremely large source files
#[test]
fn test_extremely_large_files() {
    println!("Testing extremely large source files...");
    
    let test_cases = vec![
        ("100K lines", generate_large_file(100_000)),
        ("500K lines", generate_large_file(500_000)),
        ("1M lines", generate_large_file(1_000_000)),
        ("10MB file", generate_large_character_file(10_000_000)),
        ("50MB file", generate_large_character_file(50_000_000)),
        ("100MB file", generate_large_character_file(100_000_000)),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {} (size: {} bytes)", name, code.len());
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the AST is reasonable for the input size
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extremely large files, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test deeply nested constructs
#[test]
fn test_deeply_nested_constructs() {
    println!("Testing deeply nested constructs...");
    
    let test_cases = vec![
        ("1000 deep parentheses", generate_deep_parentheses(1000)),
        ("5000 deep parentheses", generate_deep_parentheses(5000)),
        ("10000 deep parentheses", generate_deep_parentheses(10000)),
        ("1000 deep brackets", generate_deep_brackets(1000)),
        ("5000 deep brackets", generate_deep_brackets(5000)),
        ("10000 deep brackets", generate_deep_brackets(10000)),
        ("1000 deep braces", generate_deep_braces(1000)),
        ("5000 deep braces", generate_deep_braces(5000)),
        ("10000 deep braces", generate_deep_braces(10000)),
        ("1000 deep conditionals", generate_deep_conditionals(1000)),
        ("5000 deep conditionals", generate_deep_conditionals(5000)),
        ("10000 deep conditionals", generate_deep_conditionals(10000)),
        ("1000 deep loops", generate_deep_loops(1000)),
        ("5000 deep loops", generate_deep_loops(5000)),
        ("10000 deep loops", generate_deep_loops(10000)),
        ("1000 deep subroutines", generate_deep_subroutines(1000)),
        ("5000 deep subroutines", generate_deep_subroutines(5000)),
        ("10000 deep subroutines", generate_deep_subroutines(10000)),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the structure is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extremely deep nesting, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test complex expressions
#[test]
fn test_complex_expressions() {
    println!("Testing complex expressions...");
    
    let test_cases = vec![
        ("1000 nested ternary", generate_nested_ternary(1000)),
        ("5000 nested ternary", generate_nested_ternary(5000)),
        ("10000 nested ternary", generate_nested_ternary(10000)),
        ("1000 method chain", generate_massive_method_chain(1000)),
        ("5000 method chain", generate_massive_method_chain(5000)),
        ("10000 method chain", generate_massive_method_chain(10000)),
        ("1000 dereference chain", generate_complex_dereference(1000)),
        ("5000 dereference chain", generate_complex_dereference(5000)),
        ("10000 dereference chain", generate_complex_dereference(10000)),
        ("1000 operator precedence", generate_operator_precedence_mess(1000)),
        ("5000 operator precedence", generate_operator_precedence_mess(5000)),
        ("10000 operator precedence", generate_operator_precedence_mess(10000)),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the expression is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For complex expressions, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test concurrent parsing with stress
#[test]
fn test_concurrent_parsing_stress() {
    println!("Testing concurrent parsing stress...");
    
    let thread_count = 16;
    let iterations_per_thread = 10;
    
    let test_cases = vec![
        generate_massive_array(100_000),
        generate_deep_conditionals(1000),
        generate_nested_ternary(500),
        generate_massive_method_chain(500),
        generate_large_file(10_000),
        generate_regex_alternations(1000),
        generate_deep_parentheses(1000),
        generate_complex_dereference(500),
    ];
    
    let results = Arc::new(Mutex::new(Vec::new()));
    let error_count = Arc::new(Mutex::new(0));
    
    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let test_cases = test_cases.clone();
            let results_clone = Arc::clone(&results);
            let error_count_clone = Arc::clone(&error_count);
            
            thread::spawn(move || {
                for iteration in 0..iterations_per_thread {
                    let case_index = (thread_id + iteration) % test_cases.len();
                    let code = &test_cases[case_index];
                    
                    let start_time = Instant::now();
                    let mut parser = Parser::new(code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();
                    
                    let mut results = results_clone.lock().unwrap();
                    results.push((thread_id, iteration, case_index, parse_time, result.is_ok()));
                    
                    if result.is_err() {
                        *error_count_clone.lock().unwrap() += 1;
                    }
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let results = results.lock().unwrap();
    let error_count = *error_count.lock().unwrap();
    
    println!("Completed {} concurrent parses with {} errors", results.len(), error_count);
    
    // Verify no parse took too long
    for (thread_id, iteration, case_index, parse_time, _success) in results.iter() {
        assert!(
            *parse_time < MAX_PERFORMANCE_PARSE_TIME,
            "Thread {} iteration {} case {} took too long: {:?}",
            thread_id, iteration, case_index, parse_time
        );
    }
    
    // At least some parses should succeed even under stress
    let success_count = results.iter().filter(|(_, _, _, _, success)| *success).count();
    assert!(success_count > 0, "At least some parses should succeed");
}

/// Test memory pressure scenarios
#[test]
fn test_memory_pressure_scenarios() {
    println!("Testing memory pressure scenarios...");
    
    // Simulate memory pressure by parsing multiple large inputs sequentially
    let test_cases = vec![
        generate_massive_array(500_000),
        generate_massive_hash(250_000),
        generate_deep_nested_structure(25_000),
        generate_large_file(50_000),
        generate_nested_ternary(1000),
        generate_massive_method_chain(1000),
        generate_regex_alternations(5000),
        generate_deep_parentheses(5000),
    ];
    
    for (i, code) in test_cases.iter().enumerate() {
        println!("Testing memory pressure case {} (size: {} bytes)", i, code.len());
        
        let start_time = Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for case {}", i);
                
                // Verify the AST is reasonable
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for case {}", i);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Under memory pressure, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for case {}", i);
            }
        }
    }
}

/// Test resource exhaustion scenarios
#[test]
fn test_resource_exhaustion_scenarios() {
    println!("Testing resource exhaustion scenarios...");
    
    let test_cases = vec![
        // File handle exhaustion simulation
        ("Many file handles", generate_many_file_handles(1000)),
        
        // Symbol table exhaustion
        ("Huge symbol table", generate_huge_symbol_table(100_000)),
        
        // String memory exhaustion
        ("Huge strings", generate_huge_strings(1000)),
        
        // Regex compilation exhaustion
        ("Many regex", generate_many_regex(1000)),
        
        // Subroutine call depth exhaustion
        ("Deep subroutine calls", generate_deep_subroutine_calls(1000)),
        
        // Array/Hash exhaustion
        ("Huge arrays", generate_huge_arrays(1000)),
        ("Huge hashes", generate_huge_hashes(1000)),
        
        // Package exhaustion
        ("Many packages", generate_many_packages(1000)),
        
        // Module exhaustion
        ("Many modules", generate_many_modules(1000)),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the structure is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Resource exhaustion should cause graceful failure
                assert!(parse_time < MAX_PERFORMANCE_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

// Helper functions to generate test cases

fn generate_massive_array(size: usize) -> String {
    let mut result = String::from("my @array = (");
    for i in 0..size {
        if i > 0 { result.push_str(", "); }
        result.push_str(&i.to_string());
    }
    result.push_str(");");
    result
}

fn generate_massive_hash(size: usize) -> String {
    let mut result = String::from("my %hash = (");
    for i in 0..size {
        if i > 0 { result.push_str(", "); }
        result.push_str(&format!("'key{}' => {}", i, i));
    }
    result.push_str(");");
    result
}

fn generate_deep_nested_structure(depth: usize) -> String {
    let mut result = String::from("my $struct = {");
    for i in 0..depth {
        result.push_str(&format!("level{} => {{ ", i));
    }
    result.push_str("value => 42");
    for _ in 0..depth {
        result.push_str(" }");
    }
    result.push_str("};");
    result
}

fn generate_large_file(lines: usize) -> String {
    let mut result = String::new();
    for i in 0..lines {
        result.push_str(&format!("my $var{} = {};\n", i, i));
        result.push_str(&format!("print \"Line {}: $var{}\\n\";\n", i, i));
        if i % 2 == 0 {
            result.push_str(&format!("if ($var{} % 2 == 0) {{\n", i));
            result.push_str("    print \"Even number\\n\";\n");
            result.push_str("}\n");
        }
    }
    result
}

fn generate_large_character_file(chars: usize) -> String {
    "x".repeat(chars) + "\n"
}

fn generate_deep_parentheses(depth: usize) -> String {
    "(".repeat(depth) + "42" + &")".repeat(depth)
}

fn generate_deep_brackets(depth: usize) -> String {
    "[".repeat(depth) + "42" + &"]".repeat(depth)
}

fn generate_deep_braces(depth: usize) -> String {
    "{".repeat(depth) + "42" + &"}".repeat(depth)
}

fn generate_deep_conditionals(depth: usize) -> String {
    let mut result = String::new();
    for _ in 0..depth {
        result.push_str("if (1) { ");
    }
    result.push_str("42");
    for _ in 0..depth {
        result.push_str(" }");
    }
    result
}

fn generate_deep_loops(depth: usize) -> String {
    let mut result = String::new();
    for i in 0..depth {
        result.push_str(&format!("for my $i{} (1..10) {{ ", i));
    }
    result.push_str("print \"Deep loop\\n\"");
    for _ in 0..depth {
        result.push_str(" }");
    }
    result
}

fn generate_deep_subroutines(depth: usize) -> String {
    let mut result = String::new();
    for i in 0..depth {
        result.push_str(&format!("sub sub{} {{ return {}; }} ", i, i));
    }
    result.push_str("print \"Deep subroutines\\n\";");
    result
}

fn generate_nested_ternary(depth: usize) -> String {
    let mut result = String::from("my $result = ");
    for i in 0..depth {
        result.push_str(&format!("($var{} ? $val{} : ", i, i));
    }
    result.push_str("42");
    for _ in 0..depth {
        result.push_str(")");
    }
    result.push_str(";");
    result
}

fn generate_massive_method_chain(length: usize) -> String {
    let mut result = String::from("my $result = $obj");
    for i in 0..length {
        result.push_str(&format!("->method{}", i));
    }
    result.push_str(";");
    result
}

fn generate_complex_dereference(depth: usize) -> String {
    let mut result = String::from("my $result = $ref");
    for i in 0..depth {
        result.push_str(&format!("->{{nested}}{{key{}}}{{subkey{}}}[{}]", i, i, i));
    }
    result.push_str(";");
    result
}

fn generate_operator_precedence_mess(count: usize) -> String {
    let mut result = String::from("my $result = ");
    for i in 0..count {
        if i > 0 { result.push_str(" + "); }
        result.push_str(&format!("$var{} * $val{} / $div{} % $mod{}", i, i, i, i));
    }
    result.push_str(";");
    result
}

fn generate_regex_alternations(count: usize) -> String {
    let mut result = String::from("my $pattern = /(?:");
    for i in 0..count {
        if i > 0 { result.push_str("|"); }
        result.push_str(&format!("pattern{}", i));
    }
    result.push_str(")/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_huge_character_class() -> String {
    let mut result = String::from("my $pattern = /[");
    for i in 0..1000 {
        result.push_str(&(i as u8 as char).to_string());
    }
    result.push_str("]/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_nested_character_classes() -> String {
    let mut result = String::from("my $pattern = /[");
    for _ in 0..100 {
        result.push_str("a-zA-Z0-9");
    }
    result.push_str("]/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_complex_lookaheads() -> String {
    let mut result = String::from("my $pattern = /");
    for _ in 0..100 {
        result.push_str("(?=test)");
    }
    result.push_str("test/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_complex_lookbehinds() -> String {
    let mut result = String::from("my $pattern = /");
    for _ in 0..50 {
        result.push_str("(?<=test)");
    }
    result.push_str("test/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_nested_lookarounds() -> String {
    let mut result = String::from("my $pattern = /");
    for _ in 0..25 {
        result.push_str("(?=(?<=test))");
    }
    result.push_str("test/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_recursive_regex(depth: usize) -> String {
    let mut result = String::from("my $pattern = /");
    for _ in 0..depth {
        result.push_str("(?:");
    }
    result.push_str("test");
    for _ in 0..depth {
        result.push_str(")*");
    }
    result.push_str("/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_mutual_recursive_regex() -> String {
    String::from("my $pattern = /(?:(?P<name>test)|(?P=name))*/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }")
}

fn generate_massive_unicode_class() -> String {
    let mut result = String::from("my $pattern = /[\\p{L}\\p{N}\\p{P}\\p{S}\\p{Z}\\p{C}\\p{M}");
    for i in 0..100 {
        result.push_str(&format!("\\p{{Block{:03}}}", i));
    }
    result.push_str("]+/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_complex_unicode_properties() -> String {
    let mut result = String::from("my $pattern = /[");
    for _ in 0..100 {
        result.push_str("\\p{L}&\\p{Uppercase}|\\p{L}&\\p{Lowercase}|");
    }
    result.push_str("]/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_many_backreferences(count: usize) -> String {
    let mut result = String::from("my $pattern = /^(");
    for i in 1..=count {
        result.push_str(&format!("(capture{})", i));
    }
    result.push_str(")");
    for i in 1..=count {
        result.push_str(&format!("\\{}", i));
    }
    result.push_str("$/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_nested_backreferences() -> String {
    let mut result = String::from("my $pattern = /^(");
    for _ in 0..50 {
        result.push_str("(test)");
    }
    result.push_str(")");
    for i in 1..=50 {
        result.push_str(&format!("\\{}", i));
    }
    result.push_str("$/; my $text = 'test'; if ($text =~ $pattern) { print 'Match\\n'; }");
    result
}

fn generate_many_file_handles(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("open my $fh{}, '<', 'file{}.txt' or die $!; ", i, i));
    }
    result.push_str("print 'Many file handles\\n';");
    result
}

fn generate_huge_symbol_table(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("my $variable{} = {};\n", i, i));
    }
    result.push_str("print 'Huge symbol table\\n';");
    result
}

fn generate_huge_strings(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("my $string{} = '{}';\n", i, "x".repeat(10000)));
    }
    result.push_str("print 'Huge strings\\n';");
    result
}

fn generate_many_regex(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("my $regex{} = /pattern{}/;\n", i, i));
    }
    result.push_str("print 'Many regex\\n';");
    result
}

fn generate_deep_subroutine_calls(depth: usize) -> String {
    let mut result = String::new();
    for i in 0..depth {
        result.push_str(&format!("sub func{} {{ return func{}(); }}\n", i, i + 1));
    }
    result.push_str(&format!("sub func{} {{ return 42; }}\n", depth));
    result.push_str("my $result = func0();");
    result
}

fn generate_huge_arrays(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("my @array{} = (1..1000);\n", i));
    }
    result.push_str("print 'Huge arrays\\n';");
    result
}

fn generate_huge_hashes(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("my %hash{} = map {{ $_ => $_ * 2 }} (1..1000);\n", i));
    }
    result.push_str("print 'Huge hashes\\n';");
    result
}

fn generate_many_packages(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("package Package{};\n", i));
        result.push_str(&format!("sub test{} {{ return {}; }}\n", i, i));
    }
    result.push_str("print 'Many packages\\n';");
    result
}

fn generate_many_modules(count: usize) -> String {
    let mut result = String::new();
    for i in 0..count {
        result.push_str(&format!("use Module{};\n", i));
    }
    result.push_str("print 'Many modules\\n';");
    result
}