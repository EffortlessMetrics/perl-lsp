/// Enhanced parser robustness tests for improved mutation testing
///
/// These tests focus on strengthening parser handling, error paths,
/// and edge cases to improve overall test quality and mutation detection.
use perl_parser::Parser;

#[test]
fn test_parser_robustness_edge_cases() {
    let test_cases = vec![
        // Various parsing edge cases
        "my $var",
        "my $var = ",
        "$",
        "@",
        "%",
        "sub ",
        "use ",
        "package ",
        // Various contexts
        "if ($",
        "print $",
        "$hash{",
        "$array[",
        "->",
        "::",
        // Edge cases with Unicode
        "my $café",
        "my $🦀",
        "# comment\nmy $",
        // Malformed but parseable contexts
        "my $var = sub {",
        "'string$",
        "\"string$",
        "/regex$",
        // Boundary conditions
        "",
        "   ",
        "\n\n\n",
    ];

    for code in test_cases {
        let mut parser = Parser::new(code);
        let parse_result = parser.parse();

        // Test that parser doesn't panic on edge cases
        match parse_result {
            Ok(ast) => {
                let sexp = ast.to_sexp();
                // Basic validation that we got some output
                assert!(sexp.len() >= 2, "AST should have minimal content for: {}", code);
            }
            Err(_) => {
                // Parsing failure is acceptable for edge cases - just ensure no panic
            }
        }
    }
}

#[test]
fn test_parser_boundary_conditions() {
    let long_identifier = "a".repeat(1000);
    let deep_nesting = format!("{}1{}", "(".repeat(100), ")".repeat(100));
    let many_statements = "my $x = 1; ".repeat(100);

    let boundary_cases = vec![
        // Empty and minimal cases
        "",
        " ",
        "\n",
        "\t",
        "1",
        "a",
        "$",
        // Very long identifier
        &long_identifier,
        // Deep nesting
        &deep_nesting,
        // Many statements
        &many_statements,
    ];

    for code in boundary_cases {
        let mut parser = Parser::new(code);
        let _result = parser.parse();
        // Main goal: ensure no panic on boundary conditions
    }
}

#[test]
fn test_error_recovery_robustness() {
    let error_cases = vec![
        // Syntax errors that might occur in real code
        "my $var =", // Incomplete assignment
        "if (",      // Incomplete condition
        "sub {",     // Incomplete subroutine
        "my ()",     // Empty variable list
        "$#array[",  // Incomplete array access
        "%hash{key", // Incomplete hash access
        "@array[0",  // Incomplete array index
        "package;",  // Empty package name
        "use;",      // Empty use statement
        // Mismatched delimiters
        "my $x = (;",
        "my $x = [;",
        "my $x = {;",
        "my $x = ';",
        "my $x = \";",
    ];

    for code in error_cases {
        let mut parser = Parser::new(code);
        let _result = parser.parse();
        // Error recovery should not panic, regardless of result
    }
}
