//! Fuzz tests for the Perl parser using proptest

#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::PureRustPerlParser;
    use proptest::prelude::*;

    fn regex_strategy(pattern: &'static str, fallback: &'static str) -> BoxedStrategy<String> {
        match prop::string::string_regex(pattern) {
            Ok(strategy) => strategy.boxed(),
            Err(_err) => Just(fallback.to_string()).boxed(),
        }
    }

    // Strategy for generating valid Perl identifiers
    fn identifier_strategy() -> BoxedStrategy<String> {
        regex_strategy("[a-zA-Z_][a-zA-Z0-9_]{0,20}", "a")
    }

    // Strategy for generating scalar variable names
    fn scalar_var_strategy() -> impl Strategy<Value = String> {
        identifier_strategy().prop_map(|s| format!("${}", s))
    }

    // Strategy for generating array variable names
    fn array_var_strategy() -> impl Strategy<Value = String> {
        identifier_strategy().prop_map(|s| format!("@{}", s))
    }

    // Strategy for generating hash variable names
    #[allow(dead_code)]
    fn hash_var_strategy() -> impl Strategy<Value = String> {
        identifier_strategy().prop_map(|s| format!("%{}", s))
    }

    // Strategy for generating numbers
    fn number_strategy() -> BoxedStrategy<String> {
        prop_oneof![
            // Integers
            regex_strategy("[0-9]+", "0"),
            // Floats
            regex_strategy("[0-9]+\\.[0-9]+", "0.0"),
            // Scientific notation
            regex_strategy("[0-9]+\\.[0-9]+[eE][+-]?[0-9]+", "1.0e1"),
            // Hex numbers
            regex_strategy("0x[0-9a-fA-F]+", "0x0"),
            // Octal numbers
            regex_strategy("0[0-7]+", "00"),
            // Binary numbers
            regex_strategy("0b[01]+", "0b0")
        ]
        .boxed()
    }

    // Strategy for generating string literals
    fn string_strategy() -> BoxedStrategy<String> {
        prop_oneof![
            // Single quoted strings - avoid backslash at end
            regex_strategy("[^'\\\\]*", "").prop_map(|s| format!("'{}'", s)),
            // Double quoted strings (avoid control chars and quotes)
            regex_strategy("[^\"\\\\\\n\\r]*", "").prop_map(|s| format!("\"{}\"", s)),
        ]
        .boxed()
    }

    // Strategy for generating simple expressions
    fn expression_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            scalar_var_strategy(),
            number_strategy(),
            string_strategy(),
            // Binary operations
            (scalar_var_strategy(), scalar_var_strategy())
                .prop_map(|(a, b)| format!("{} + {}", a, b)),
            (scalar_var_strategy(), scalar_var_strategy())
                .prop_map(|(a, b)| format!("{} . {}", a, b)),
        ]
    }

    // Strategy for generating statements
    fn statement_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Variable declarations
            (scalar_var_strategy(), expression_strategy())
                .prop_map(|(var, expr)| format!("my {} = {};", var, expr)),
            (array_var_strategy()).prop_map(|var| format!("my {} = ();", var)),
            // Assignments
            (scalar_var_strategy(), expression_strategy())
                .prop_map(|(var, expr)| format!("{} = {};", var, expr)),
            // Print statements
            expression_strategy().prop_map(|expr| format!("print {};", expr)),
            // Conditionals
            (scalar_var_strategy(), expression_strategy())
                .prop_map(|(var, expr)| format!("if ({} > 0) {{ {} = {}; }}", var, var, expr)),
        ]
    }

    proptest! {
        #[test]
        fn test_parser_doesnt_crash(s in ".*") {
            let mut parser = PureRustPerlParser::new();
            // The parser should not panic on any input
            let _ = parser.parse(&s);
        }

        #[test]
        fn test_valid_identifiers(ident in identifier_strategy()) {
            let mut parser = PureRustPerlParser::new();
            let code = format!("my ${} = 1;", ident);
            let result = parser.parse(&code);
            prop_assert!(result.is_ok(), "Failed to parse valid identifier: {}", code);
        }

        #[test]
        fn test_valid_numbers(num in number_strategy()) {
            let mut parser = PureRustPerlParser::new();
            let code = format!("my $x = {};", num);
            let result = parser.parse(&code);
            prop_assert!(result.is_ok(), "Failed to parse valid number: {}", code);
        }

        #[test]
        fn test_valid_strings(s in string_strategy()) {
            let mut parser = PureRustPerlParser::new();
            let code = format!("my $x = {};", s);
            let result = parser.parse(&code);
            prop_assert!(result.is_ok(), "Failed to parse valid string: {}", code);
        }

        #[test]
        fn test_valid_statements(stmt in statement_strategy()) {
            let mut parser = PureRustPerlParser::new();
            let result = parser.parse(&stmt);
            prop_assert!(result.is_ok(), "Failed to parse valid statement: {}", stmt);
        }

        #[test]
        fn test_multiple_statements(stmts in prop::collection::vec(statement_strategy(), 1..10)) {
            let mut parser = PureRustPerlParser::new();
            let code = stmts.join("\n");
            let result = parser.parse(&code);
            prop_assert!(result.is_ok(), "Failed to parse multiple statements: {}", code);
        }

        #[test]
        fn test_nested_blocks(depth: u8) {
            let depth = (depth % 5) + 1; // Limit depth to 1-5
            let mut code = String::new();

            // Generate nested blocks
            for i in 0..depth {
                code.push_str(&format!("if ($x{} > 0) {{\n", i));
            }
            code.push_str("    print \"nested\";\n");
            for _ in 0..depth {
                code.push_str("}\n");
            }

            let mut parser = PureRustPerlParser::new();
            let result = parser.parse(&code);
            prop_assert!(result.is_ok(), "Failed to parse nested blocks: {}", code);
        }

        #[test]
        fn test_regex_patterns(pattern in "[a-zA-Z0-9 ]+") {
            let mut parser = PureRustPerlParser::new();
            let code = format!("$x =~ /{}/;", pattern);
            let result = parser.parse(&code);
            prop_assert!(result.is_ok(), "Failed to parse regex pattern: {}", code);
        }
    }
}
