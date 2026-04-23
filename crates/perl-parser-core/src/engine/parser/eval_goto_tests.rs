#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;

    /// Helper: parse code and assert no errors were produced.
    fn assert_parses_without_errors(input: &str) {
        let mut parser = Parser::new(input);
        let output = parser.parse_with_recovery();
        let sexp = output.ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "Expected no ERROR nodes for input: {}\nAST: {}",
            input,
            sexp,
        );
        assert!(
            output.diagnostics.is_empty(),
            "Expected no diagnostics for input: {}\nDiagnostics: {:?}",
            input,
            output.diagnostics,
        );
    }

    /// Helper: parse code and return the s-expression.
    fn parse_sexp(input: &str) -> String {
        let mut parser = Parser::new(input);
        let output = parser.parse_with_recovery();
        output.ast.to_sexp()
    }

    // ===== eval block edge cases =====

    #[test]
    fn test_eval_block_or_pattern() {
        // eval { die "oops" } or warn "caught: $@";
        assert_parses_without_errors(r#"eval { die "oops" } or warn "caught: $@";"#);
    }

    #[test]
    fn test_eval_block_in_assignment() {
        // my $result = eval { complex_operation() };
        assert_parses_without_errors("my $result = eval { complex_operation() };");
    }

    #[test]
    fn test_eval_block_defined_or() {
        // eval { $obj->method() } // do { handle_error() };
        assert_parses_without_errors("eval { 1 } // do { handle_error() };");
    }

    #[test]
    fn test_eval_block_logical_or() {
        // eval { ... } || die "failed";
        assert_parses_without_errors(r#"eval { 1 } || die "failed";"#);
    }

    #[test]
    fn test_eval_block_logical_and() {
        // eval { ... } && print "ok";
        assert_parses_without_errors(r#"eval { 1 } && print "ok";"#);
    }

    #[test]
    fn test_eval_string_or_pattern() {
        // eval $code or die "eval failed: $@";
        assert_parses_without_errors(r#"eval $code or die "eval failed: $@";"#);
    }

    #[test]
    fn test_eval_block_produces_eval_node() {
        let sexp = parse_sexp("eval { 1 };");
        assert!(sexp.contains("eval"), "Expected eval node in AST: {}", sexp,);
    }

    // ===== goto statement tests =====

    #[test]
    fn test_goto_label() {
        // goto LABEL;
        assert_parses_without_errors("goto LABEL;");
    }

    #[test]
    fn test_goto_sub_reference() {
        // goto &other_sub;
        assert_parses_without_errors("goto &other_sub;");
    }

    #[test]
    fn test_goto_produces_goto_node() {
        let sexp = parse_sexp("goto LABEL;");
        assert!(sexp.contains("goto"), "Expected goto node in AST: {}", sexp,);
    }

    #[test]
    fn test_goto_sub_reference_produces_goto_node() {
        let sexp = parse_sexp("goto &other_sub;");
        assert!(sexp.contains("goto"), "Expected goto node in AST: {}", sexp,);
    }

    #[test]
    fn test_goto_computed_label() {
        // goto $label;
        assert_parses_without_errors("goto $label;");
    }
}
