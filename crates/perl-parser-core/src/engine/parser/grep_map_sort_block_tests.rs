#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;

    /// Helper: parse code, assert no errors, return s-expression.
    fn parse_ok(input: &str) -> String {
        let mut parser = Parser::new(input);
        let output = parser.parse_with_recovery();
        let sexp = output.ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "Expected no ERROR nodes for: {input}\nAST: {sexp}",
        );
        assert!(
            output.diagnostics.is_empty(),
            "Expected no diagnostics for: {input}\nDiagnostics: {:?}",
            output.diagnostics,
        );
        sexp
    }

    /// Helper: count top-level statements in a Program node.
    fn count_top_statements(input: &str) -> usize {
        let mut parser = Parser::new(input);
        let output = parser.parse_with_recovery();
        if let perl_ast::NodeKind::Program { statements } = &output.ast.kind {
            statements.len()
        } else {
            0
        }
    }

    #[test]
    fn grep_block_array_single_statement() {
        let sexp = parse_ok("grep { $_ > 0 } @list;");
        assert!(sexp.contains("call"), "should produce a call node: {sexp}");
        assert!(
            sexp.contains("block"),
            "should contain a block arg: {sexp}"
        );
        assert_eq!(count_top_statements("grep { $_ > 0 } @list;"), 1);
    }

    #[test]
    fn grep_block_with_defined() {
        let sexp = parse_ok("grep { defined } @items;");
        assert!(sexp.contains("call"), "should produce a call node: {sexp}");
        assert_eq!(count_top_statements("grep { defined } @items;"), 1);
    }

    #[test]
    fn map_block_array_single_statement() {
        let sexp = parse_ok("map { $_ * 2 } @numbers;");
        assert!(sexp.contains("call"), "should produce a call node: {sexp}");
        assert!(
            sexp.contains("block"),
            "should contain a block arg: {sexp}"
        );
        assert_eq!(count_top_statements("map { $_ * 2 } @numbers;"), 1);
    }

    #[test]
    fn map_block_literal_list() {
        let sexp = parse_ok("map { $_ * 2 } 1, 2, 3;");
        assert!(sexp.contains("call"), "should produce a call node: {sexp}");
        assert_eq!(count_top_statements("map { $_ * 2 } 1, 2, 3;"), 1);
    }

    #[test]
    fn sort_block_array_single_statement() {
        let sexp = parse_ok("sort { $a <=> $b } @list;");
        assert!(sexp.contains("call"), "should produce a call node: {sexp}");
        assert!(
            sexp.contains("block"),
            "should contain a block arg: {sexp}"
        );
        assert_eq!(count_top_statements("sort { $a <=> $b } @list;"), 1);
    }

    #[test]
    fn grep_block_in_assignment() {
        let sexp = parse_ok("my @r = grep { defined } @items;");
        assert!(
            sexp.contains("call"),
            "grep should produce a call node: {sexp}"
        );
        assert_eq!(
            count_top_statements("my @r = grep { defined } @items;"),
            1
        );
    }

    #[test]
    fn map_block_in_assignment() {
        let sexp = parse_ok("my @doubled = map { $_ * 2 } @numbers;");
        assert!(
            sexp.contains("call"),
            "map should produce a call node: {sexp}"
        );
        assert_eq!(
            count_top_statements("my @doubled = map { $_ * 2 } @numbers;"),
            1
        );
    }

    #[test]
    fn sort_block_in_assignment() {
        let sexp = parse_ok("my @sorted = sort { $a <=> $b } @list;");
        assert!(
            sexp.contains("call"),
            "sort should produce a call node: {sexp}"
        );
        assert_eq!(
            count_top_statements("my @sorted = sort { $a <=> $b } @list;"),
            1
        );
    }

    #[test]
    fn grep_block_in_assignment_with_array() {
        let sexp = parse_ok("my @pos = grep { $_ > 0 } @numbers;");
        assert!(sexp.contains("call"), "should produce a call node: {sexp}");
        assert_eq!(
            count_top_statements("my @pos = grep { $_ > 0 } @numbers;"),
            1
        );
    }
}
