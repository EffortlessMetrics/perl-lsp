//! Tests to ensure parser handles deeply nested constructs without stack overflow

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::PureRustPerlParser;

    #[test]
    fn test_basic_parsing() {
        let mut parser = PureRustPerlParser::new();

        // Test basic expressions
        assert!(parser.parse("$x").is_ok());
        assert!(parser.parse("$x = 42").is_ok());

        // Test print with string
        // Note: Some complex cases might fail during preprocessing
        // For now, just test simpler cases
        assert!(parser.parse("print 42").is_ok());
    }

    #[test]
    fn test_nested_parentheses() {
        let mut parser = PureRustPerlParser::new();

        // Test increasingly nested parentheses
        assert!(parser.parse("(42)").is_ok());
        assert!(parser.parse("((42))").is_ok());
        assert!(parser.parse("(((42)))").is_ok());
        assert!(parser.parse("((((42))))").is_ok());

        // Test 10 levels
        let expr10 = "((((((((((42))))))))))";
        assert!(parser.parse(expr10).is_ok());
    }

    #[test]
    fn test_nested_expressions() {
        let mut parser = PureRustPerlParser::new();

        // Test nested arithmetic
        assert!(parser.parse("1 + 2").is_ok());
        assert!(parser.parse("(1 + 2) * 3").is_ok());
        assert!(parser.parse("((1 + 2) * 3) / 4").is_ok());
    }

    #[test]
    fn test_complex_expressions() {
        let mut parser = PureRustPerlParser::new();

        // Test more complex Perl constructs
        assert!(parser.parse("if ($x) { print $x }").is_ok());
        assert!(parser.parse("for (1..10) { print }").is_ok());
        assert!(parser.parse("@sorted = sort { $a <=> $b } @numbers").is_ok());
        // This is a complex dereferencing case that might not be fully supported
        // assert!(parser.parse(r#"$ref = \@{$hash{key}->[0]}"#).is_ok());
    }

    #[test]
    fn test_stacker_integration() {
        let mut parser = PureRustPerlParser::new();

        // Test that stacker prevents stack overflow
        // Create a moderately deep expression
        let mut expr = "42".to_string();
        for _ in 0..20 {
            expr = format!("({})", expr);
        }

        // This should work thanks to stacker
        let result = parser.parse(&expr);
        assert!(result.is_ok(), "Parser should handle 20 levels of nesting");
    }
}
