#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_indirect_method_call() {
        let code = r#"
my $method = "print";
my $object = "hello";
$method $object;
"#;
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok(), "Failed to parse indirect method call");

        let ast = must(result);
        let _sexp = ast.to_sexp();
        // Should parse as variable followed by variable if not detected as indirect call,
        // OR as indirect call if our heuristics match.
        // However, `$method $object` is NOT valid indirect syntax because method name must be bareword.
        // It's actually a syntax error in strict Perl unless $method is a coderef.
        // But our parser might see it as two expression statements or something else.

        // Let's test valid indirect call: method name is bareword
        let code_valid = "print $object 'arg';";
        let mut parser_valid = Parser::new(code_valid);
        let result_valid = parser_valid.parse();
        assert!(result_valid.is_ok());
        let ast_valid = result_valid.unwrap();
        let sexp_valid = ast_valid.to_sexp();
        // Should be parsed as indirect call
        assert!(
            sexp_valid.contains("indirect_call"),
            "Should detect indirect call in: {}",
            sexp_valid
        );
    }

    #[test]
    fn test_indirect_constructor() {
        let code = "new MyClass($arg);";
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = must(result);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("indirect_call"), "Should detect indirect constructor in: {}", sexp);
    }

    #[test]
    fn test_arrow_dereference() {
        let code = "$ref->{key};";
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = must(result);
        let sexp = ast.to_sexp();
        // Should be binary operation, NOT indirect call
        assert!(
            !sexp.contains("indirect_call"),
            "Arrow deref should not be indirect call in: {}",
            sexp
        );
    }

    #[test]
    fn test_ambiguous_print() {
        // print $x, $y -> print($x, $y) (not indirect)
        let code_direct = "print $x, $y;";
        let mut parser = Parser::new(code_direct);
        let ast = parser.parse().unwrap();
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("indirect_call"),
            "Comma should prevent indirect call detection: {}",
            sexp
        );

        // print $fh $x -> print($fh, $x) (indirect)
        let code_indirect = "print $fh $x;";
        let mut parser2 = Parser::new(code_indirect);
        let ast2 = parser2.parse().unwrap();
        let sexp2 = ast2.to_sexp();
        assert!(
            sexp2.contains("indirect_call"),
            "Should detect indirect print to filehandle: {}",
            sexp2
        );
    }
}
