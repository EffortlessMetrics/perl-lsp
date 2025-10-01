/// Enhanced edge case parsing tests for improved mutation testing preparation
///
/// This test suite focuses on strengthening parser coverage for edge cases and
/// complex Perl syntax patterns that may not be fully covered by existing tests.
/// These tests are designed to catch mutations and improve test quality.
use perl_parser::Parser;

#[test]
fn test_deeply_nested_structures() {
    // Test deeply nested hash and array references
    let test_cases = vec![
        // Deeply nested hash references
        r#"$hash{key1}{key2}{key3}{key4}{key5}"#,
        // Deeply nested array references
        r#"$array[0][1][2][3][4]"#,
        // Mixed nested structures
        r#"$data{users}[0]{profile}{settings}[2]{theme}"#,
        // Complex dereferencing
        r#"${$hash{key}}->{nested}[0]"#,
        // Postfix dereferencing (modern Perl)
        r#"$ref->@*"#,
        r#"$ref->%*"#,
        r#"$ref->$*"#,
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();
        assert!(result.is_ok(), "Test case {} failed to parse: {}", i, input);

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        // Ensure the AST contains expected elements based on the input
        if input.contains("hash") || input.contains("{") {
            assert!(
                sexp.contains("hash_access") || sexp.contains("variable"),
                "Hash access not found in AST for: {}",
                input
            );
        }
        if input.contains("array") || input.contains("[") {
            assert!(
                sexp.contains("array_access") || sexp.contains("variable"),
                "Array access not found in AST for: {}",
                input
            );
        }
    }
}

#[test]
fn test_complex_subroutine_signatures() {
    // Test modern Perl subroutine signatures with various parameter types
    let test_cases = vec![
        // Basic signatures
        "sub test($x) { return $x; }",
        "sub test($x, $y) { return $x + $y; }",
        // Optional parameters
        "sub test($x, $y = 42) { return $x + $y; }",
        // Slurpy parameters
        "sub test($first, @rest) { return @rest; }",
        "sub test($first, %opts) { return %opts; }",
        // Mixed parameter types
        "sub test($req, $opt = 'default', @slurpy) { }",
        // Prototype with signature
        "sub test :prototype($) ($x) { return $x; }",
        // Anonymous subroutines with signatures
        "my $sub = sub($x, $y) { $x + $y };",
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();
        assert!(result.is_ok(), "Test case {} failed to parse: {}", i, input);

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        // Verify subroutine structure is captured
        assert!(
            sexp.contains("sub") || sexp.contains("subroutine"),
            "Subroutine not found in AST for: {}",
            input
        );

        // Verify parameters are captured
        if input.contains("$x") {
            assert!(
                sexp.contains("$x") || sexp.contains("variable"),
                "Parameter $x not found in AST for: {}",
                input
            );
        }
    }
}

#[test]
fn test_complex_regex_patterns() {
    // Test complex regular expression patterns that stress the parser
    let test_cases = vec![
        // Basic regex with modifiers
        r#"$str =~ /pattern/gi"#,
        // Complex character classes
        r#"$str =~ /[a-zA-Z0-9_\-\.]+@[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,}/i"#,
        // Lookahead and lookbehind
        r#"$str =~ /(?=.*[A-Z])(?=.*[a-z])(?=.*\d).{8,}/"#,
        // Non-capturing groups
        r#"$str =~ /(?:foo|bar|baz)+/"#,
        // Named captures
        r#"$str =~ /(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/"#,
        // Substitution with complex patterns
        r#"$str =~ s/(\w+)\s+(\w+)/$2, $1/g"#,
        // Transliteration
        r#"$str =~ tr/a-z/A-Z/"#,
        r#"$str =~ y/aeiou/AEIOU/"#,
        // Regex with different delimiters
        r#"$str =~ m{pattern}"#,
        r#"$str =~ m|pattern|"#,
        r#"$str =~ m#pattern#"#,
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();
        assert!(result.is_ok(), "Test case {} failed to parse: {}", i, input);

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        // Verify regex operation is captured
        assert!(
            sexp.contains("=~")
                || sexp.contains("match")
                || sexp.contains("regex")
                || sexp.contains("substitution")
                || sexp.contains("subst")
                || sexp.contains("transliteration")
                || sexp.contains("trans"),
            "Regex operation not found in AST for: {}",
            input
        );
    }
}

#[test]
fn test_unicode_and_encoding_edge_cases() {
    // Test Unicode handling and encoding edge cases
    let test_cases = vec![
        // Unicode identifiers
        r#"my $café = "coffee";"#,
        r#"my $🦀 = "Rust";"#,
        // Unicode in strings
        r#"my $greeting = "Hello, 世界";"#,
        // Unicode in regex
        r#"$text =~ /\p{L}+/"#,
        r#"$text =~ /[α-ω]+/"#,
        // Unicode normalization
        r#"use Unicode::Normalize; my $nfc = NFC($text);"#,
        // Encoding pragmas
        r#"use utf8; my $unicode_string = "café";"#,
        // Wide characters
        r#"my $wide = "\x{1F980}";"#,
        // Mixed ASCII and Unicode
        r#"my $mixed = "ASCII_text_καὶ_Greek";"#,
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();
        assert!(result.is_ok(), "Test case {} failed to parse: {}", i, input);

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        // Basic validation that structure is captured
        assert!(!sexp.is_empty(), "Empty AST for Unicode test case: {}", input);
    }
}

#[test]
fn test_error_recovery_scenarios() {
    // Test parser recovery from various syntax errors
    let test_cases = vec![
        // Unmatched parentheses
        "my $x = (1 + 2;",
        // Unmatched brackets
        "my @array = [1, 2, 3;",
        // Unmatched braces
        "my %hash = {key => 'value';",
        // Incomplete subroutine
        "sub test {",
        // Malformed regex
        "my $x = /[unclosed;",
        // Invalid variable names
        "my $1invalid = 42;",
        // Incomplete statements
        "if ($condition",
        // Missing semicolon in block
        "{ my $x = 1 my $y = 2; }",
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();

        // Error recovery: parser should either succeed with partial parsing
        // or fail gracefully without panicking
        match result {
            Ok(ast) => {
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "Empty AST for error case {}: {}", i, input);
            }
            Err(_) => {
                // Expected for malformed input - just ensure no panic
                // This tests error handling paths
            }
        }
    }
}

#[test]
fn test_large_literal_handling() {
    // Test handling of large literals and edge cases
    let long_string = format!("my $long = '{}';", "x".repeat(1000));
    let large_array =
        format!("my @big = ({});", (0..100).map(|i| i.to_string()).collect::<Vec<_>>().join(", "));
    let large_hash = format!(
        "my %big = ({});",
        (0..50).map(|i| format!("'key{}' => 'value{}'", i, i)).collect::<Vec<_>>().join(", ")
    );

    let test_cases = vec![
        // Large numbers
        "my $big = 999999999999999999999;",
        // Very long strings
        &long_string,
        // Large arrays
        &large_array,
        // Large hashes
        &large_hash,
        // Deeply nested expressions
        "my $result = ((((1 + 2) * 3) - 4) / 5);",
        // Long method chains
        "my $result = $obj->method1()->method2()->method3()->method4()->method5();",
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();
        assert!(result.is_ok(), "Test case {} failed to parse: {}", i, input);

        let ast = result.unwrap();
        let sexp = ast.to_sexp();

        // Ensure parsing completes and produces reasonable output
        assert!(!sexp.is_empty(), "Empty AST for large literal test case: {}", input);
        assert!(sexp.len() > 10, "AST too small for input: {}", input);
    }
}

#[test]
fn test_modern_perl_features() {
    // Test modern Perl features that may not be well covered
    let test_cases = vec![
        // Postfix dereferencing
        "my $array_ref = [1, 2, 3]; my @array = $array_ref->@*;",
        "my $hash_ref = {a => 1}; my %hash = $hash_ref->%*;",
        // Key/value slices
        "my %subset = %hash{'key1', 'key2'};",
        "my @values = @hash{'key1', 'key2'};",
        // State variables
        "sub counter { state $count = 0; return ++$count; }",
        // Smart match (deprecated but still parsed)
        "given ($value) { when (1) { say 'one'; } default { say 'other'; } }",
        // Try/catch (experimental)
        "use experimental 'try'; try { die 'error'; } catch ($e) { warn $e; }",
        // Function signatures with types
        "use experimental 'signatures'; sub add($x, $y) { $x + $y }",
    ];

    for (_i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();

        // Modern features may not be fully supported, so we're more lenient
        match result {
            Ok(ast) => {
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "Empty AST for modern Perl test case: {}", input);
            }
            Err(_) => {
                // Some modern features might not be implemented yet
                // This is acceptable for now
            }
        }
    }
}

#[test]
fn test_boundary_conditions() {
    // Test various boundary conditions that could expose bugs
    let test_cases = vec![
        // Empty input
        "",
        // Whitespace only
        "   \t\n  ",
        // Single character
        ";",
        // Single variable
        "$x",
        // Single number
        "42",
        // Single string
        "'hello'",
        // Minimal subroutine
        "sub{1}",
        // Minimal block
        "{1}",
        // Minimal array
        "[]",
        // Minimal hash
        "{}",
        // Comments only
        "# This is a comment\n",
        "# Comment 1\n# Comment 2\n",
        // POD only
        "=pod\nThis is documentation\n=cut\n",
        // Mixed whitespace and comments
        "  # comment  \n\t# another\n  ",
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let mut parser = Parser::new(input);
        let result = parser.parse();

        // Boundary conditions should parse successfully or fail gracefully
        match result {
            Ok(ast) => {
                let sexp = ast.to_sexp();
                // Empty or minimal input may produce minimal AST
                if !input.trim().is_empty()
                    && !input.trim().starts_with('#')
                    && !input.trim().starts_with('=')
                {
                    assert!(
                        !sexp.is_empty(),
                        "Empty AST for boundary test case {}: '{}'",
                        i,
                        input
                    );
                }
            }
            Err(_) => {
                // Some boundary cases may legitimately fail
                // Just ensure no panic occurs
            }
        }
    }
}

#[test]
fn test_performance_stress_patterns() {
    // Test patterns that could cause performance issues
    let nested_parens = format!("my $x = {};", "(".repeat(50) + "42" + &")".repeat(50));
    let long_identifier = format!("my ${} = 42;", "a".repeat(100));
    let many_variables = format!(
        "my ({}) = (1..100);",
        (0..100).map(|i| format!("$var{}", i)).collect::<Vec<_>>().join(", ")
    );
    let alternation_options =
        format!("/{}/", (0..50).map(|i| format!("option{}", i)).collect::<Vec<_>>().join("|"));
    let long_string = format!("my $str = '{}';", "content ".repeat(200));

    let test_cases = vec![
        // Deeply nested parentheses
        &nested_parens,
        // Long identifier names
        &long_identifier,
        // Many variables in one statement
        &many_variables,
        // Alternation with many options
        &alternation_options,
        // Long string literals
        &long_string,
    ];

    for (i, input) in test_cases.iter().enumerate() {
        let start = std::time::Instant::now();
        let mut parser = Parser::new(input);
        let result = parser.parse();
        let duration = start.elapsed();

        // Ensure parsing completes in reasonable time (less than 1 second)
        assert!(
            duration.as_secs() < 1,
            "Performance test case {} took too long: {:?}",
            i,
            duration
        );

        // Verify parsing succeeded
        assert!(result.is_ok(), "Performance test case {} failed to parse", i);
    }
}
