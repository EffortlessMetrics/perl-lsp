#[cfg(test)]
mod property_scanner {
    use proptest::prelude::*;
    use crate::test_harness::{parse_perl_code, validate_tree_no_errors};

    // Strategy for generating random Perl identifiers
    fn perl_identifier() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "var", "x", "y", "z", "foo", "bar", "baz", "qux", "quux", "corge",
            "grault", "garply", "waldo", "fred", "plugh", "xyzzy", "thud",
            "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta",
            "theta", "iota", "kappa", "lambda", "mu", "nu", "xi", "omicron",
            "pi", "rho", "sigma", "tau", "upsilon", "phi", "chi", "psi", "omega"
        ])
    }

    // Strategy for generating random Perl variable names
    fn perl_variable() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "$var", "$x", "$y", "$z", "$foo", "$bar", "$baz", "$qux",
            "$alpha", "$beta", "$gamma", "$delta", "$epsilon", "$zeta",
            "@array", "@list", "@items", "@values", "@data",
            "%hash", "%dict", "%map", "%table", "%config"
        ])
    }

    // Strategy for generating random numbers
    fn perl_number() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "0", "1", "2", "3", "42", "100", "1000", "10000",
            "0.0", "1.0", "2.5", "3.14", "42.0", "100.5",
            "0x0", "0x1", "0xFF", "0x100", "0xFFFF",
            "0b0", "0b1", "0b1010", "0b11111111",
            "0o0", "0o1", "0o77", "0o100", "0o777"
        ])
    }

    // Strategy for generating random string literals
    fn perl_string() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            r#""Hello, World!""#,
            r#"'Hello, World!'"#,
            r#""Simple string""#,
            r#"'Simple string'"#,
            r#""Empty""#,
            r#"''"#,
            r#""With\nnewline""#,
            r#""With\ttab""#,
            r#""With\r\ncrlf""#,
            r#""With $interpolation""#,
            r#"'No $interpolation'",
        ])
    }

    // Strategy for generating random operators
    fn perl_operator() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "+", "-", "*", "/", "%", "**",
            "==", "!=", "eq", "ne",
            "<", ">", "<=", ">=", "lt", "gt", "le", "ge",
            "&&", "||", "and", "or", "xor",
            "&", "|", "^", "~",
            "<<", ">>",
            "=", "+=", "-=", "*=", "/=", "%=",
            "++", "--",
            "?", ":", "..", "...",
        ])
    }

    // Strategy for generating simple Perl expressions
    fn perl_expression() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "42",
            "$var",
            "1 + 2",
            "$x * $y",
            "($a + $b) * $c",
            "print 'Hello'",
            "my $var = 42",
            "return 1",
            "undef",
            "true",
            "false",
        ])
    }

    // Strategy for generating simple Perl statements
    fn perl_statement() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "my $var = 42;",
            "print 'Hello';",
            "return 1;",
            "undef;",
            "1 + 2;",
            "$x = $y;",
            "if ($x) { print $x; }",
            "while ($x) { $x--; }",
            "for my $i (1..10) { print $i; }",
            "sub foo { return 1; }",
        ])
    }

    // Property test: All valid Perl expressions should parse without panicking
    #[test]
    fn test_parse_no_panic() {
        proptest!(|(expr in perl_expression())| {
            let result = parse_perl_code(&expr);
            // Should not panic, even if it fails to parse
            assert!(result.is_ok() || result.is_err());
        });
    }

    // Property test: All valid Perl statements should parse without panicking
    #[test]
    fn test_statement_no_panic() {
        proptest!(|(stmt in perl_statement())| {
            let result = parse_perl_code(&stmt);
            // Should not panic, even if it fails to parse
            assert!(result.is_ok() || result.is_err());
        });
    }

    // Property test: Variable declarations should parse successfully
    #[test]
    fn test_variable_declaration_parses() {
        proptest!(|(var in perl_variable(), num in perl_number())| {
            let code = format!("my {} = {};", var, num);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse: {}", code);
        });
    }

    // Property test: String assignments should parse successfully
    #[test]
    fn test_string_assignment_parses() {
        proptest!(|(var in perl_variable(), string in perl_string())| {
            let code = format!("my {} = {};", var, string);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse: {}", code);
        });
    }

    // Property test: Simple expressions should parse successfully
    #[test]
    fn test_simple_expression_parses() {
        proptest!(|(var in perl_variable(), num1 in perl_number(), num2 in perl_number())| {
            let code = format!("my {} = {} + {};", var, num1, num2);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse: {}", code);
        });
    }

    // Property test: Function calls should parse successfully
    #[test]
    fn test_function_call_parses() {
        proptest!(|(func in perl_identifier(), arg in perl_string())| {
            let code = format!("{}({});", func, arg);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse: {}", code);
        });
    }

    // Property test: Generated code should not contain error nodes if it parses successfully
    #[test]
    fn test_no_error_nodes_on_success() {
        proptest!(|(stmt in perl_statement())| {
            if let Ok(tree) = parse_perl_code(&stmt) {
                let validation = validate_tree_no_errors(&tree);
                assert!(validation.is_ok(), "Tree has error nodes: {}", stmt);
            }
        });
    }

    // Property test: Round-trip parsing should be consistent
    #[test]
    fn test_round_trip_consistency() {
        proptest!(|(stmt in perl_statement())| {
            let result1 = parse_perl_code(&stmt);
            let result2 = parse_perl_code(&stmt);
            
            // Both parses should have the same outcome
            match (result1, result2) {
                (Ok(tree1), Ok(tree2)) => {
                    // Both trees should be valid
                    assert!(validate_tree_no_errors(&tree1).is_ok());
                    assert!(validate_tree_no_errors(&tree2).is_ok());
                }
                (Err(e1), Err(e2)) => {
                    // Both should fail with similar errors
                    println!("Both parses failed as expected: {} vs {}", e1, e2);
                }
                (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
                    panic!("Inconsistent parsing results for: {}", stmt);
                }
            }
        });
    }

    // Property test: Whitespace variations should not affect parsing
    #[test]
    fn test_whitespace_invariance() {
        proptest!(|(stmt in perl_statement())| {
            let variants = vec![
                stmt.clone(),
                stmt.replace(" ", "  "),  // Double spaces
                stmt.replace(" ", "\t"),  // Tabs instead of spaces
                stmt.replace(" ", "\n"),  // Newlines instead of spaces
                format!(" {}", stmt),     // Leading space
                format!("{} ", stmt),     // Trailing space
                format!(" {} ", stmt),    // Both leading and trailing
            ];
            
            let mut results = Vec::new();
            for variant in variants {
                results.push(parse_perl_code(&variant));
            }
            
            // All variants should have the same success/failure outcome
            let first_success = results[0].is_ok();
            for (i, result) in results.iter().enumerate() {
                assert_eq!(result.is_ok(), first_success, 
                          "Variant {} has different outcome: {:?}", i, result);
            }
        });
    }

    // Property test: Comments should not affect parsing
    #[test]
    fn test_comment_invariance() {
        proptest!(|(stmt in perl_statement())| {
            let with_comment = format!("{} # This is a comment", stmt);
            let result1 = parse_perl_code(&stmt);
            let result2 = parse_perl_code(&with_comment);
            
            // Both should have the same success/failure outcome
            assert_eq!(result1.is_ok(), result2.is_ok(), 
                      "Comment affects parsing: {} vs {}", 
                      result1.is_ok(), result2.is_ok());
        });
    }

    // Property test: Multiple statements should parse
    #[test]
    fn test_multiple_statements() {
        proptest!(|(stmt1 in perl_statement(), stmt2 in perl_statement())| {
            let code = format!("{}\n{}", stmt1, stmt2);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse multiple statements: {}", code);
        });
    }

    // Property test: Nested expressions should parse
    #[test]
    fn test_nested_expressions() {
        proptest!(|(var in perl_variable(), num1 in perl_number(), num2 in perl_number(), num3 in perl_number())| {
            let code = format!("my {} = ({} + {}) * {};", var, num1, num2, num3);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse nested expression: {}", code);
        });
    }

    // Property test: String concatenation should parse
    #[test]
    fn test_string_concatenation() {
        proptest!(|(var in perl_variable(), str1 in perl_string(), str2 in perl_string())| {
            let code = format!("my {} = {}.{};", var, str1, str2);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse string concatenation: {}", code);
        });
    }

    // Property test: Array and hash access should parse
    #[test]
    fn test_array_hash_access() {
        proptest!(|(var in perl_variable(), index in perl_number())| {
            let array_access = format!("my {} = $array[{}];", var, index);
            let hash_access = format!("my {} = $hash{{'key'}};", var);
            
            let result1 = parse_perl_code(&array_access);
            let result2 = parse_perl_code(&hash_access);
            
            assert!(result1.is_ok(), "Failed to parse array access: {}", array_access);
            assert!(result2.is_ok(), "Failed to parse hash access: {}", hash_access);
        });
    }

    // Property test: Control flow should parse
    #[test]
    fn test_control_flow() {
        proptest!(|(var in perl_variable(), num in perl_number())| {
            let if_stmt = format!("if (my {} = {}) {{ print $var; }}", var, num);
            let while_stmt = format!("while (my {} = {}) {{ print $var; }}", var, num);
            let for_stmt = format!("for my {} (1..{}) {{ print $var; }}", var, num);
            
            let result1 = parse_perl_code(&if_stmt);
            let result2 = parse_perl_code(&while_stmt);
            let result3 = parse_perl_code(&for_stmt);
            
            assert!(result1.is_ok(), "Failed to parse if statement: {}", if_stmt);
            assert!(result2.is_ok(), "Failed to parse while statement: {}", while_stmt);
            assert!(result3.is_ok(), "Failed to parse for statement: {}", for_stmt);
        });
    }

    // Property test: Function definitions should parse
    #[test]
    fn test_function_definition() {
        proptest!(|(func in perl_identifier(), var in perl_variable(), num in perl_number())| {
            let code = format!("sub {} {{ my {} = {}; return $var; }}", func, var, num);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse function definition: {}", code);
        });
    }

    // Property test: Package declarations should parse
    #[test]
    fn test_package_declaration() {
        proptest!(|(pkg in perl_identifier())| {
            let code = format!("package {};", pkg);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse package declaration: {}", code);
        });
    }

    // Property test: Use statements should parse
    #[test]
    fn test_use_statement() {
        proptest!(|(module in perl_identifier())| {
            let code = format!("use {};", module);
            let result = parse_perl_code(&code);
            assert!(result.is_ok(), "Failed to parse use statement: {}", code);
        });
    }
} 