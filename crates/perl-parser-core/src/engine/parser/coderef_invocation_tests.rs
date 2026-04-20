#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use crate::parser::Parser;
    use perl_ast::ast::{Node, NodeKind};

    /// Helper: parse code and return the full AST.
    fn parse_program(code: &str) -> Node {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => ast,
            Err(e) => panic!("Parse failed for `{code}`: {e:?}"),
        }
    }

    /// Helper: parse code and return the first statement node.
    fn parse_first_stmt(code: &str) -> Node {
        let ast = parse_program(code);
        let ast_sexp = ast.to_sexp();
        let NodeKind::Program { mut statements } = ast.kind else {
            panic!("Expected Program with statements, got: {}", ast_sexp)
        };
        if statements.is_empty() {
            panic!("Expected Program with statements, got: {}", ast_sexp);
        }
        statements.swap_remove(0)
    }

    /// Helper: check that the AST sexp contains no ERROR nodes.
    fn assert_no_errors(code: &str) {
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes: {}", code, sexp,);
    }

    /// Helper: extract expression from an ExpressionStatement.
    fn unwrap_expr_stmt(stmt: Node) -> Node {
        let NodeKind::ExpressionStatement { expression } = stmt.kind else {
            panic!(
                "Expected ExpressionStatement, got {} (sexp: {})",
                stmt.kind.kind_name(),
                stmt.to_sexp()
            )
        };
        *expression
    }

    // ---------------------------------------------------------------
    // Basic coderef invocation: $code->()
    // ---------------------------------------------------------------

    #[test]
    fn coderef_invocation_no_args() {
        let code = "$code->();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $code->(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()", "Expected ->() coderef call, got name={}", name);
        // First arg is the callee ($code)
        assert!(!args.is_empty(), "Expected at least 1 arg (the callee expression)",);
        assert_eq!(
            args[0].kind.kind_name(),
            "Variable",
            "First arg should be the Variable node for $code",
        );
        // No additional args beyond the callee
        assert_eq!(args.len(), 1, "Expected exactly 1 arg (callee only) for empty arglist");
    }

    // ---------------------------------------------------------------
    // Coderef invocation with arguments: $code->($x, $y)
    // ---------------------------------------------------------------

    #[test]
    fn coderef_invocation_with_args() {
        let code = "$code->($x, $y);";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $code->($x, $y), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        // First arg is callee, then $x and $y
        assert!(args.len() >= 3, "Expected callee + 2 args, got {} args", args.len());
    }

    // ---------------------------------------------------------------
    // Hash element coderef: $hash{callback}->()
    // ---------------------------------------------------------------

    #[test]
    fn hash_element_coderef_invocation() {
        let code = "$hash{callback}->();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $hash{{callback}}->(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        // First arg should be the hash access expression $hash{callback}
        assert!(!args.is_empty(), "Expected callee arg");
        assert_eq!(
            args[0].kind.kind_name(),
            "Binary",
            "First arg should be Binary (hash access), got {}",
            args[0].kind.kind_name(),
        );
    }

    // ---------------------------------------------------------------
    // Array element coderef: $arr[0]->()
    // ---------------------------------------------------------------

    #[test]
    fn array_element_coderef_invocation() {
        let code = "$arr[0]->();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $arr[0]->(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        assert!(!args.is_empty(), "Expected callee arg");
        assert_eq!(
            args[0].kind.kind_name(),
            "Binary",
            "First arg should be Binary (array access), got {}",
            args[0].kind.kind_name(),
        );
    }

    // ---------------------------------------------------------------
    // Chained method then coderef: $obj->method->()
    // ---------------------------------------------------------------

    #[test]
    fn chained_method_then_coderef() {
        let code = "$obj->method->();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $obj->method->(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        // First arg should be the MethodCall node for $obj->method
        assert!(!args.is_empty(), "Expected callee arg");
        assert_eq!(
            args[0].kind.kind_name(),
            "MethodCall",
            "First arg should be MethodCall for $obj->method, got {}",
            args[0].kind.kind_name(),
        );
    }

    // ---------------------------------------------------------------
    // $self->can('method')->() chained
    // ---------------------------------------------------------------

    #[test]
    fn can_method_coderef_invocation() {
        let code = "$self->can('method')->();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $self->can('method')->(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        assert!(!args.is_empty(), "Expected callee arg");
        // The callee should be a MethodCall ($self->can('method'))
        assert_eq!(
            args[0].kind.kind_name(),
            "MethodCall",
            "Expected MethodCall as callee, got {}",
            args[0].kind.kind_name(),
        );
    }

    // ---------------------------------------------------------------
    // Ampersand coderef call: &$code()
    // ---------------------------------------------------------------

    #[test]
    fn ampersand_coderef_call() {
        // &$code() - the & sigil with a scalar variable and parens
        // This is handled in parse_variable_from_sigil, not in postfix.
        // Just verify it parses without errors.
        let code = "&$coderef();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let sexp = expr.to_sexp();
        assert!(
            sexp.contains("call") || sexp.contains("Call"),
            "Expected some call form for &$coderef(), got: {}",
            sexp,
        );
    }

    // ---------------------------------------------------------------
    // Coderef invocation with complex expression args
    // ---------------------------------------------------------------

    #[test]
    fn coderef_invocation_with_string_arg() {
        let code = r#"$handler->("hello");"#;
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $handler->(\"hello\"), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        // callee + one string arg
        assert_eq!(args.len(), 2, "Expected callee + 1 arg, got {} args", args.len());
    }

    // ---------------------------------------------------------------
    // Double coderef chain: $a->()->()
    // ---------------------------------------------------------------

    #[test]
    fn double_coderef_chain() {
        let code = "$factory->()->();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // The outer call should be a ->() wrapping another ->()
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            panic!(
                "Expected FunctionCall for $factory->()->(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(name, "->()");
        assert!(!args.is_empty(), "Expected callee arg");
        // The inner callee should also be a ->() FunctionCall
        let NodeKind::FunctionCall { name: inner_name, .. } = &args[0].kind else {
            panic!(
                "Expected inner FunctionCall for $factory->()->(), got {} (sexp: {})",
                args[0].kind.kind_name(),
                args[0].to_sexp()
            )
        };
        assert_eq!(inner_name, "->()");
    }

    // ---------------------------------------------------------------
    // Coderef in assignment context
    // ---------------------------------------------------------------

    #[test]
    fn coderef_invocation_in_assignment() {
        let code = "my $result = $callback->(42);";
        assert_no_errors(code);
        // Drill into the AST to find the FunctionCall with name "->()"
        let ast = parse_program(code);
        let mut found = false;
        if let NodeKind::Program { ref statements } = ast.kind {
            if let Some(stmt) = statements.first() {
                if let NodeKind::VariableDeclaration { initializer: Some(ref init), .. } = stmt.kind
                {
                    if let NodeKind::FunctionCall { ref name, ref args } = init.kind {
                        assert_eq!(name, "->()");
                        // callee ($callback) + arg (42)
                        assert_eq!(
                            args.len(),
                            2,
                            "Expected callee + 1 arg, got {} args",
                            args.len()
                        );
                        found = true;
                    }
                }
            }
        }
        assert!(
            found,
            "Did not find ->() FunctionCall in assignment RHS (sexp: {})",
            ast.to_sexp()
        );
    }
}
