//! Extreme Input Edge Cases for Perl Parser
//!
//! This test suite validates parser behavior with extreme inputs that push
//! the parser to its absolute limits. These tests are designed to ensure
//! the parser remains stable and performs reasonably even with pathological
//! inputs that might occur in real-world scenarios or adversarial inputs.

use perl_parser::Parser;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Maximum reasonable parsing time for extreme inputs (in seconds)
const MAX_PARSE_TIME: Duration = Duration::from_secs(30);

/// Maximum reasonable memory usage for extreme inputs (in MB)
const _MAX_MEMORY_USAGE_MB: usize = 500;

/// Test extremely large identifiers and symbols
#[test]
fn test_extremely_large_identifiers() {
    println!("Testing extremely large identifiers...");

    // Generate identifiers of various extreme sizes
    let test_cases = vec![
        ("1KB identifier", "x".repeat(1024)),
        ("10KB identifier", "y".repeat(10 * 1024)),
        ("100KB identifier", "z".repeat(100 * 1024)),
        ("1MB identifier", "a".repeat(1024 * 1024)),
    ];

    for (name, identifier) in test_cases {
        println!("Testing: {}", name);

        let code = format!("my ${} = 42;\nprint \"${{{}}}\\n\"", identifier, identifier);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        // Should either parse successfully or fail gracefully
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the identifier is present in the AST
                let sexp = ast.to_sexp();
                assert!(sexp.contains("variable"), "Variable not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extremely large identifiers, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test extremely deep nesting that exceeds reasonable limits
#[test]
fn test_extreme_nesting_depth() {
    println!("Testing extreme nesting depth...");

    // Generate deeply nested structures beyond reasonable limits
    let test_cases = vec![
        ("500 deep parentheses", generate_deep_parentheses(500)),
        ("1000 deep brackets", generate_deep_brackets(1000)),
        ("2000 deep braces", generate_deep_braces(2000)),
        ("3000 deep conditionals", generate_deep_conditionals(3000)),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        // Should either parse successfully or fail gracefully with recursion limit error
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the AST depth is reasonable
                let depth = calculate_ast_depth(&ast);
                assert!(depth < 1000, "AST depth {} seems unreasonable for {}", depth, name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extreme nesting, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
                assert!(
                    e.to_string().contains("recursion")
                        || e.to_string().contains("depth")
                        || e.to_string().contains("stack")
                        || e.to_string().contains("limit"),
                    "Error should mention depth/recursion limit for {}: {}",
                    name,
                    e
                );
            }
        }
    }
}

/// Test extremely long strings and string operations
#[test]
fn test_extremely_large_strings() {
    println!("Testing extremely large strings...");

    let test_cases = vec![
        ("10MB string literal", "x".repeat(10 * 1024 * 1024)),
        ("50MB string literal", "y".repeat(50 * 1024 * 1024)),
        ("100MB string literal", "z".repeat(100 * 1024 * 1024)),
    ];

    for (name, string_content) in test_cases {
        println!("Testing: {}", name);

        let code = format!("my $large_string = '{}';", string_content);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the string is present in the AST
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("string") || sexp.contains("literal"),
                    "String not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extremely large strings, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test extremely large arrays and hashes
#[test]
fn test_extremely_large_data_structures() {
    println!("Testing extremely large data structures...");

    // Generate code with extremely large arrays and hashes
    let test_cases = vec![
        ("1M element array", generate_large_array(1_000_000)),
        ("100K element hash", generate_large_hash(100_000)),
        ("10K nested structure", generate_nested_structure(10_000)),
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
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the structure is present in the AST
                let sexp = ast.to_sexp();
                if name.contains("array") {
                    assert!(
                        sexp.contains("array") || sexp.contains("list"),
                        "Array not found in AST for {}",
                        name
                    );
                } else if name.contains("hash") {
                    assert!(
                        sexp.contains("hash") || sexp.contains("pair"),
                        "Hash not found in AST for {}",
                        name
                    );
                }
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extremely large structures, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test extremely complex regular expressions
#[test]
fn test_extremely_complex_regex() {
    println!("Testing extremely complex regular expressions...");

    let test_cases = vec![
        ("Catastrophic backtracking".to_string(), r"/^(a+)+b$/".to_string()),
        ("Nested quantifiers".to_string(), r"/^(a*)*b$/".to_string()),
        ("Excessive alternation".to_string(), generate_excessive_alternation(1000)),
        ("Deeply nested groups".to_string(), generate_nested_regex_groups(100)),
        (
            "Unicode character classes".to_string(),
            r"/[\p{L}\p{N}\p{P}\p{S}\p{Z}\p{C}\p{M}]+/".to_string(),
        ),
        ("Complex lookarounds".to_string(), r"/(?=(?<!a)b)(?<!c)(?!d)/".to_string()),
        ("Recursive patterns".to_string(), r"/(?R)/".to_string()),
        ("Backreference hell".to_string(), generate_backreference_hell(50)),
    ];

    for (name, pattern) in &test_cases {
        println!("Testing: {}", name);

        let code = format!("my $result = 'test' =~ {};", pattern);

        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the regex is present in the AST
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("regex") || sexp.contains("pattern") || sexp.contains("match"),
                    "Regex not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For complex regex, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test extremely large source files
#[test]
fn test_extremely_large_files() {
    println!("Testing extremely large source files...");

    let test_cases = vec![
        ("100K line file", generate_large_file(100_000)),
        ("1M line file", generate_large_file(1_000_000)),
        ("10M character file", generate_large_character_file(10_000_000)),
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
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the AST is reasonable for the input size
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For extremely large files, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test extremely complex expressions
#[test]
fn test_extremely_complex_expressions() {
    println!("Testing extremely complex expressions...");

    let test_cases = vec![
        ("Deeply nested ternary", generate_nested_ternary(50)),
        ("Massive method chain", generate_massive_method_chain(100)),
        ("Complex dereference chain", generate_complex_dereference(50)),
        ("Huge operator precedence", generate_operator_precedence_mess(100)),
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
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for {}", name);

                // Verify the expression is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // For complex expressions, parsing might fail, but should fail gracefully
                assert!(parse_time < MAX_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test concurrent parsing with extreme inputs
#[test]
fn test_concurrent_extreme_inputs() {
    println!("Testing concurrent parsing with extreme inputs...");

    let thread_count = 8;
    let iterations_per_thread = 5;

    let test_cases = vec![
        generate_deep_parentheses(1000),
        generate_large_array(100_000),
        generate_excessive_alternation(500),
        generate_nested_ternary(25),
        generate_large_file(50_000),
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
            *parse_time < MAX_PARSE_TIME,
            "Thread {} iteration {} case {} took too long: {:?}",
            thread_id,
            iteration,
            case_index,
            parse_time
        );
    }

    // At least some parses should succeed even with extreme inputs
    let success_count = results.iter().filter(|(_, _, _, _, success)| *success).count();
    assert!(success_count > 0, "At least some parses should succeed");
}

/// Test memory pressure with extreme inputs
#[test]
fn test_memory_pressure_with_extreme_inputs() {
    println!("Testing memory pressure with extreme inputs...");

    // Simulate memory pressure by parsing multiple extreme inputs sequentially
    let test_cases = vec![
        generate_large_array(50_000),
        generate_large_hash(25_000),
        generate_nested_structure(5_000),
        generate_large_file(25_000),
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
                assert!(parse_time < MAX_PARSE_TIME, "Parse time exceeded limit for case {}", i);

                // Verify the AST is reasonable
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for case {}", i);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Under memory pressure, parsing might fail, but should fail gracefully
                assert!(
                    parse_time < MAX_PARSE_TIME,
                    "Error detection took too long for case {}",
                    i
                );
            }
        }
    }
}

// Helper functions to generate test cases

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

fn generate_large_array(size: usize) -> String {
    let mut result = String::from("my @array = (");
    for i in 0..size {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&i.to_string());
    }
    result.push_str(");");
    result
}

fn generate_large_hash(size: usize) -> String {
    let mut result = String::from("my %hash = (");
    for i in 0..size {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&format!("'key{}' => {}", i, i));
    }
    result.push_str(");");
    result
}

fn generate_nested_structure(depth: usize) -> String {
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

fn generate_excessive_alternation(count: usize) -> String {
    let mut result = String::from("/(?:");
    for i in 0..count {
        if i > 0 {
            result.push_str("|");
        }
        result.push_str(&format!("pattern{}", i));
    }
    result.push_str(")/");
    result
}

fn generate_nested_regex_groups(depth: usize) -> String {
    let mut result = String::from("/");
    for _ in 0..depth {
        result.push_str("(?:");
    }
    result.push_str("test");
    for _ in 0..depth {
        result.push_str(")");
    }
    result.push_str("/");
    result
}

fn generate_backreference_hell(count: usize) -> String {
    let mut result = String::from("/^(a");
    for i in 1..=count {
        result.push_str(&format!(")(a{})", i));
    }
    result.push_str("\\1");
    for i in 2..=count {
        result.push_str(&format!("\\{}", i));
    }
    result.push_str(")$/");
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
        if i > 0 {
            result.push_str(" + ");
        }
        result.push_str(&format!("$var{} * $val{} / $div{} % $mod{}", i, i, i, i));
    }
    result.push_str(";");
    result
}

fn calculate_ast_depth(node: &perl_parser::Node) -> usize {
    // Simple depth calculation - count the maximum nesting level
    let sexp = node.to_sexp();
    let mut max_depth = 0;
    let mut current_depth = 0;

    for ch in sexp.chars() {
        if ch == '(' {
            current_depth += 1;
            max_depth = max_depth.max(current_depth);
        } else if ch == ')' {
            current_depth -= 1;
        }
    }

    max_depth
}
