//! Tests to ensure iterative parser produces same results as recursive parser

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::{
        pure_rust_parser::{PureRustPerlParser, PerlParser, Rule},
        parser_benchmark::{ParserBenchmark, ParserImpl},
    };
    use pest::Parser;
    
    /// Helper to compare recursive and iterative results
    fn compare_parsers(input: &str) {
        let mut parser_recursive = PureRustPerlParser::new();
        let mut parser_iterative = PureRustPerlParser::new();
        
        // Parse with Pest
        let pairs_rec = PerlParser::parse(Rule::program, input).unwrap();
        let pairs_iter = PerlParser::parse(Rule::program, input).unwrap();
        
        let pair_rec = pairs_rec.into_iter().next().unwrap();
        let pair_iter = pairs_iter.into_iter().next().unwrap();
        
        // Build AST with both methods
        let recursive_result = parser_recursive.build_node(pair_rec).unwrap();
        let iterative_result = parser_iterative.build_node_iterative(pair_iter).unwrap();
        
        // Convert to S-expressions for comparison
        let rec_sexp = recursive_result.map(|n| parser_recursive.to_sexp(&n)).unwrap_or_default();
        let iter_sexp = iterative_result.map(|n| parser_iterative.to_sexp(&n)).unwrap_or_default();
        
        assert_eq!(rec_sexp, iter_sexp, 
            "Mismatch for input: {}\nRecursive: {}\nIterative: {}", 
            input, rec_sexp, iter_sexp);
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
            compare_parsers(input);
        }
    }
    
    #[test]
    fn test_nested_structures() {
        compare_parsers("{ { { print } } }");
        compare_parsers("[[[[1]]]]");
        compare_parsers("((((42))))");
        compare_parsers("if ($a) { if ($b) { if ($c) { print } } }");
    }
    
    #[test]
    fn test_complex_expressions() {
        compare_parsers("$result = ($a + $b) * ($c - $d) / $e");
        compare_parsers("@sorted = sort { $a <=> $b } @numbers");
        compare_parsers("$ref = \\@{$hash{key}->[0]}");
    }
    
    #[test]
    #[cfg(debug_assertions)]
    fn test_deep_nesting_debug() {
        // In debug mode, test that iterative handles deep nesting
        let depth = 500;
        let mut expr = "42".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }
        
        let mut parser = PureRustPerlParser::new();
        let pairs = PerlParser::parse(Rule::expression, &expr).unwrap();
        let pair = pairs.into_iter().next().unwrap();
        
        // This should work without stack overflow
        let result = parser.build_node_iterative(pair);
        assert!(result.is_ok(), "Iterative parser should handle deep nesting");
    }
    
    #[test]
    fn test_performance_characteristics() {
        use std::time::Instant;
        
        // Create expressions with varying depths
        for depth in [10, 50, 100, 200] {
            let mut expr = "1".to_string();
            for _ in 0..depth {
                expr = format!("($expr + 1)");
            }
            
            let mut bench = ParserBenchmark::new();
            
            // Time iterative approach
            let start = Instant::now();
            let iter_result = bench.bench_parser(ParserImpl::Iterative, &expr);
            let iter_time = start.elapsed();
            
            // Time recursive with stacker
            let start = Instant::now();
            let stack_result = bench.bench_parser(ParserImpl::RecursiveWithStacker, &expr);
            let stack_time = start.elapsed();
            
            println!("Depth {}: Iterative {:?}, Stacker {:?}", 
                depth, iter_time, stack_time);
            
            // Both should succeed
            assert!(iter_result.is_ok());
            assert!(stack_result.is_ok());
        }
    }
}