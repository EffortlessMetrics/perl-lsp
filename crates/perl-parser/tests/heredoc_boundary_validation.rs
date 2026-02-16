use perl_parser::Parser;

#[test]
fn test_heredoc_boundary_fix_validation() {
    println!("=== Validating Heredoc Boundary Fix (PR #153) ===");

    // Test cases that would previously crash due to off-by-one error in parse_heredoc_delimiter
    let critical_test_cases = [
        // Empty quoted delimiters - these triggered the original crash
        ("<<\"\"", "Empty double-quoted delimiter"),
        ("<<''", "Empty single-quoted delimiter"),
        // Single character delimiters - potential boundary issues
        ("<<\"a\"", "Single char double-quoted"),
        ("<<'a'", "Single char single-quoted"),
        // Malformed delimiters that triggered the boundary error
        ("<<\"", "Unterminated double quote"),
        ("<<'", "Unterminated single quote"),
        ("<<\"a", "Missing closing double quote"),
        ("<<'a", "Missing closing single quote"),
        // Edge cases that stressed the boundary logic
        ("<<\"\\\"\"", "Escaped quote in delimiter"),
        ("<<'\\''", "Escaped quote in single delimiter"),
    ];

    let mut total_tests = 0;
    let mut passed_tests = 0;

    for (test_input, description) in &critical_test_cases {
        total_tests += 1;
        println!("Testing: {} with input: {}", description, test_input);

        // This should not crash or panic after the boundary fix
        let result = std::panic::catch_unwind(|| {
            let mut parser = Parser::new(test_input);
            parser.parse()
        });

        assert!(result.is_ok(), "Critical boundary issue detected in: {}", description);
        if result.is_ok() {
            println!("✅ SAFE (no crash)");
            passed_tests += 1;
        } else {
            println!("❌ CRASHED");
        }
    }

    println!("\n=== Boundary Fix Validation Results ===");
    println!("Total tests: {}", total_tests);
    println!("Passed (no crashes): {}", passed_tests);

    assert_eq!(passed_tests, total_tests, "Some heredoc boundary tests failed");
    println!("✅ ALL TESTS PASSED - Boundary fix is working correctly!");
}

#[test]
fn test_heredoc_integration_after_fix() {
    // Test complete heredoc construct to ensure fix doesn't break normal functionality
    let integration_test = r#"
my $text = <<"EOF";
This is a test
EOF
print $text;
"#;

    let result = std::panic::catch_unwind(|| {
        let mut parser = Parser::new(integration_test);
        parser.parse()
    });

    assert!(result.is_ok(), "Integration test should not crash after boundary fix");
}

#[test]
fn test_specific_boundary_conditions() {
    // Test the exact conditions fixed in parse_heredoc_delimiter (lines 5267 & 5270)
    let boundary_conditions = [
        "<<\"\"",  // rest.len() == 2, exactly at boundary
        "<<''",    // rest.len() == 2, exactly at boundary
        "<<\"a\"", // rest.len() == 3, valid case
        "<<'a'",   // rest.len() == 3, valid case
        "<<\"",    // rest.len() == 2, but missing end quote
        "<<'",     // rest.len() == 2, but missing end quote
    ];

    for test_case in &boundary_conditions {
        println!("Testing boundary condition: {}", test_case);

        // Should not panic due to slice index out of bounds
        let result = std::panic::catch_unwind(|| {
            let mut parser = Parser::new(test_case);
            let _result = parser.parse();
        });

        assert!(result.is_ok(), "Boundary condition test failed for: {}", test_case);
    }
}
