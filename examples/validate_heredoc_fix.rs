use perl_parser::Parser;

fn main() {
    println!("=== Validating Heredoc Boundary Fix (PR #153) ===");

    // Test cases that would previously crash due to off-by-one error
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
        print!("Testing: {} ... ", description);

        // This should not crash or panic after the boundary fix
        let result = std::panic::catch_unwind(|| {
            let mut parser = Parser::new(test_input);
            parser.parse()
        });

        match result {
            Ok(_) => {
                println!("✅ SAFE (no crash)");
                passed_tests += 1;
            }
            Err(_) => {
                println!("❌ CRASHED");
            }
        }
    }

    println!("\n=== Boundary Fix Validation Results ===");
    println!("Total tests: {}", total_tests);
    println!("Passed (no crashes): {}", passed_tests);
    println!("Failed (crashed): {}", total_tests - passed_tests);

    if passed_tests == total_tests {
        println!("✅ ALL TESTS PASSED - Boundary fix is working correctly!");
        println!("The off-by-one error in heredoc delimiter parsing has been successfully resolved.");
    } else {
        println!("❌ SOME TESTS FAILED - Boundary issues still exist!");
    }

    // Additional test: Integration with actual heredoc content
    println!("\n=== Integration Test with Heredoc Content ===");
    let integration_test = r#"
my $text = <<"EOF";
This is a test
EOF
print $text;
"#;

    print!("Testing complete heredoc construct ... ");
    let result = std::panic::catch_unwind(|| {
        let mut parser = Parser::new(integration_test);
        parser.parse()
    });

    match result {
        Ok(_) => println!("✅ SAFE"),
        Err(_) => println!("❌ CRASHED"),
    }
}