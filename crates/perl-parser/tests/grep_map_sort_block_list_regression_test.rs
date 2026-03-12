#[cfg(test)]
mod grep_map_sort_block_list_regression_tests {
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn assert_parses_with_block_list_and_trailing_arg(code: &str, builtin_name: &str) {
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();

        assert!(!sexp.contains("(ERROR"), "unexpected parse error in: {sexp}");
        assert!(
            sexp.contains(&format!("(call {builtin_name}")),
            "expected builtin call in: {sexp}"
        );
        assert!(sexp.contains("(block"), "expected block arg for {builtin_name} in: {sexp}");
        assert!(
            sexp.contains("(variable @ items)"),
            "expected list arg for {builtin_name} in: {sexp}"
        );
        assert!(sexp.contains("(number 42)"), "expected trailing argument after list in: {sexp}");
    }

    #[test]
    fn map_block_list_inside_parenthesized_call() {
        assert_parses_with_block_list_and_trailing_arg("foo(map { $_ } @items, 42);", "map");
    }

    #[test]
    fn grep_block_list_inside_parenthesized_call() {
        assert_parses_with_block_list_and_trailing_arg("foo(grep { $_ > 1 } @items, 42);", "grep");
    }

    #[test]
    fn sort_block_list_inside_parenthesized_call() {
        assert_parses_with_block_list_and_trailing_arg(
            "foo(sort { $a <=> $b } @items, 42);",
            "sort",
        );
    }
}
