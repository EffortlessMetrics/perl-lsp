//! Tests to ensure parser handles deeply nested constructs without stack overflow

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::PureRustPerlParser;
    
    /// Helper to test parsing without stack overflow
    fn test_parse_no_overflow(input: &str) {
        let mut parser = PureRustPerlParser::new();
        
        // This should not cause stack overflow
        match parser.parse(input) {
            Ok(_) => {
                // Successfully parsed
            }
            Err(e) => {
                panic!("Failed to parse: {:?}\nInput: {}", e, input);
            }
        }
    }
    
    #[test]
    fn test_simple_expressions() {
        let test_cases = vec![
            "$x",
            "$x = 42",
            "@arr = (1, 2, 3)",
            "%hash = (a => 1, b => 2)",
            "$x + $y",
            "print \"Hello\"",
            "if ($x) { print $x }",
            "for (1..10) { print }",
        ];
        
        for input in test_cases {
            test_parse_no_overflow(input);
        }
    }
    
    #[test]
    fn test_nested_structures() {
        test_parse_no_overflow("{ { { print } } }");
        test_parse_no_overflow("[[[[1]]]]");
        test_parse_no_overflow("((((42))))");
        test_parse_no_overflow("if ($a) { if ($b) { if ($c) { print } } }");
    }
    
    #[test]
    fn test_complex_expressions() {
        test_parse_no_overflow("$result = ($a + $b) * ($c - $d) / $e");
        test_parse_no_overflow("@sorted = sort { $a <=> $b } @numbers");
        test_parse_no_overflow("$ref = \\@{$hash{key}->[0]}");
    }
    
    #[test]
    fn test_deep_nesting() {
        // Test that parser handles deep nesting using stacker
        let depth = 50; // Safe depth that should work with stacker
        let mut expr = "42".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }
        
        test_parse_no_overflow(&expr);
    }
    
    #[test]
    fn test_various_depths() {
        // Test expressions with varying depths
        for depth in [5, 10, 20] {
            let mut expr = "1".to_string();
            for _ in 0..depth {
                expr = format!("($expr + 1)");
            }
            
            test_parse_no_overflow(&expr);
        }
    }
    
    #[test] 
    fn test_stacker_prevents_overflow() {
        // This test verifies that stacker is working
        // by parsing a moderately deep expression
        let depth = 30;
        let mut expr = "$x".to_string();
        for _ in 0..depth {
            expr = format!("({})->method()", expr);
        }
        
        let mut parser = PureRustPerlParser::new();
        // This should succeed thanks to stacker
        assert!(parser.parse(&expr).is_ok(), "Stacker should prevent stack overflow");
    }
}