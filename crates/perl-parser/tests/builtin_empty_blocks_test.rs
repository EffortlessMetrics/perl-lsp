#[cfg(test)]
mod builtin_empty_blocks_tests {
    use perl_parser::Parser;

    fn parse_and_check(input: &str, expected_contains: &str) {
        use perl_tdd_support::must;
        let mut parser = Parser::new(input);
        let result = must(parser.parse());
        let sexp = result.to_sexp();
        assert!(
            sexp.contains(expected_contains),
            "Expected '{}' to contain '{}', but got: {}",
            sexp,
            expected_contains,
            sexp
        );
    }

    #[test]
    fn test_sort_empty_block() {
        parse_and_check("sort {} @array", "(call sort ((block ) (variable @ array)))");
    }

    #[test]
    fn test_map_empty_block() {
        parse_and_check("map {} @array", "(call map ((block ) (variable @ array)))");
    }

    #[test]
    fn test_grep_empty_block() {
        parse_and_check("grep {} @array", "(call grep ((block ) (variable @ array)))");
    }

    #[test]
    fn test_sort_with_expression() {
        parse_and_check("sort { $a cmp $b } @array", "(call sort ((block ");
    }

    #[test]
    fn test_map_with_expression() {
        parse_and_check("map { $_ * 2 } @array", "(call map ((block ");
    }

    #[test]
    fn test_grep_with_expression() {
        parse_and_check("grep { $_ > 5 } @array", "(call grep ((block ");
    }

    #[test]
    fn test_ref_empty_hash() {
        parse_and_check("ref {}", "(call ref ((hash ))");
    }

    #[test]
    fn test_defined_empty_hash() {
        parse_and_check("defined {}", "(call defined ((hash ))");
    }

    #[test]
    fn test_scalar_empty_hash() {
        parse_and_check("scalar {}", "(call scalar ((hash ))");
    }

    #[test]
    fn test_keys_empty_hash() {
        parse_and_check("keys {}", "(call keys ((hash ))");
    }

    #[test]
    fn test_values_empty_hash() {
        parse_and_check("values {}", "(call values ((hash ))");
    }

    #[test]
    fn test_each_empty_hash() {
        parse_and_check("each {}", "(call each ((hash ))");
    }

    #[test]
    fn test_return_sort_empty_block() {
        parse_and_check("return sort {} @array", "(return (call sort ((block ))");
    }

    #[test]
    fn test_return_map_empty_block() {
        parse_and_check("return map {} @array", "(return (call map ((block ))");
    }

    #[test]
    fn test_return_grep_empty_block() {
        parse_and_check("return grep {} @array", "(return (call grep ((block ))");
    }
}
