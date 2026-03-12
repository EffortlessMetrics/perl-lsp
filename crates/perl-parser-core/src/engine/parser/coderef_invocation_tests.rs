#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_coderef_invocation_no_args() {
        let code = "$code->();";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("method_call"),
            "Coderef invocation $code->() should parse as method_call: {sexp}",
        );
    }

    #[test]
    fn test_coderef_invocation_with_args() {
        let code = r#"$code->("arg1", "arg2");"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("method_call"),
            "Coderef invocation $code->(args) should parse as method_call: {sexp}",
        );
    }

    #[test]
    fn test_coderef_invocation_from_hash_value() {
        let code = "$hash{callback}->();";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("method_call"),
            "Coderef from hash $hash{{callback}}->() should parse as method_call: {sexp}",
        );
    }

    #[test]
    fn test_coderef_invocation_from_array_element() {
        let code = "$array[0]->();";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("method_call"),
            "Coderef from array $array[0]->() should parse as method_call: {sexp}",
        );
    }

    #[test]
    fn test_coderef_invocation_chained_after_method() {
        let code = "$obj->method()->();";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        // Should contain two method_call nodes: one for ->method() and one for ->()
        let count = sexp.matches("method_call").count();
        assert!(
            count >= 2,
            "Chained $obj->method()->() should produce at least 2 method_call nodes, got {count}: {sexp}",
        );
    }

    #[test]
    fn test_coderef_invocation_with_sub_assignment() {
        let code = r#"
my $code = sub { print "hello" };
$code->();
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("method_call"), "Coderef with sub assignment should parse: {sexp}",);
    }

    #[test]
    fn test_coderef_invocation_no_errors() {
        let code = "$code->();";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR") && !sexp.contains("Missing"),
            "Coderef invocation should produce no error nodes: {sexp}",
        );
    }
}
