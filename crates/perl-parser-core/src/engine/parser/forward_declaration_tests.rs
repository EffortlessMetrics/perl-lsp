#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::NodeKind;

    /// Helper: parse code and assert no errors were produced, returning the AST.
    fn parse_ok(input: &str) -> perl_ast::ast::Node {
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
        output.ast
    }

    #[test]
    fn test_forward_declaration_bare() {
        // sub foo; — forward declaration with no prototype, no block
        let ast = parse_ok("sub foo;");
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            if let NodeKind::Subroutine { name, prototype, signature, body, .. } =
                &statements[0].kind
            {
                assert_eq!(name.as_deref(), Some("foo"));
                assert!(prototype.is_none());
                assert!(signature.is_none());
                // Body should be an empty block (forward declaration)
                if let NodeKind::Block { statements } = &body.kind {
                    assert!(statements.is_empty(), "Forward declaration should have empty body");
                } else {
                    unreachable!("Expected Block body, got {:?}", body.kind);
                }
            } else {
                unreachable!("Expected Subroutine, got {:?}", statements[0].kind);
            }
        }
    }

    #[test]
    fn test_forward_declaration_with_prototype() {
        // sub foo(@); — forward declaration with prototype
        let ast = parse_ok("sub foo(@);");
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            if let NodeKind::Subroutine { name, prototype, body, .. } = &statements[0].kind {
                assert_eq!(name.as_deref(), Some("foo"));
                // Should have a prototype
                assert!(prototype.is_some(), "Expected prototype for sub foo(@)");
                // Body should be an empty block (forward declaration)
                if let NodeKind::Block { statements } = &body.kind {
                    assert!(statements.is_empty(), "Forward declaration should have empty body");
                } else {
                    unreachable!("Expected Block body, got {:?}", body.kind);
                }
            } else {
                unreachable!("Expected Subroutine, got {:?}", statements[0].kind);
            }
        }
    }

    #[test]
    fn test_forward_declaration_with_attribute() {
        // sub foo :method; — forward declaration with attribute
        let ast = parse_ok("sub foo :method;");
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            if let NodeKind::Subroutine { name, attributes, body, .. } = &statements[0].kind {
                assert_eq!(name.as_deref(), Some("foo"));
                assert!(!attributes.is_empty(), "Expected attributes for sub foo :method");
                assert_eq!(attributes[0], "method");
                // Body should be an empty block (forward declaration)
                if let NodeKind::Block { statements } = &body.kind {
                    assert!(statements.is_empty(), "Forward declaration should have empty body");
                } else {
                    unreachable!("Expected Block body, got {:?}", body.kind);
                }
            } else {
                unreachable!("Expected Subroutine, got {:?}", statements[0].kind);
            }
        }
    }

    #[test]
    fn test_normal_subroutine_still_works() {
        // sub foo { 1 } — normal subroutine with block body
        let ast = parse_ok("sub foo { 1 }");
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            if let NodeKind::Subroutine { name, body, .. } = &statements[0].kind {
                assert_eq!(name.as_deref(), Some("foo"));
                // Body should be a non-empty block
                if let NodeKind::Block { statements } = &body.kind {
                    assert!(!statements.is_empty(), "Normal subroutine should have non-empty body");
                } else {
                    unreachable!("Expected Block body, got {:?}", body.kind);
                }
            } else {
                unreachable!("Expected Subroutine, got {:?}", statements[0].kind);
            }
        }
    }

    #[test]
    fn test_forward_declaration_with_dollar_prototype() {
        // sub foo($); — forward declaration with scalar prototype
        let ast = parse_ok("sub foo($);");
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            if let NodeKind::Subroutine { name, body, .. } = &statements[0].kind {
                assert_eq!(name.as_deref(), Some("foo"));
                if let NodeKind::Block { statements } = &body.kind {
                    assert!(statements.is_empty(), "Forward declaration should have empty body");
                } else {
                    unreachable!("Expected Block body, got {:?}", body.kind);
                }
            } else {
                unreachable!("Expected Subroutine, got {:?}", statements[0].kind);
            }
        }
    }

    #[test]
    fn test_multiple_forward_declarations() {
        // Multiple forward declarations in sequence
        let ast = parse_ok("sub foo; sub bar; sub baz;");
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 3, "Should parse three forward declarations");
            for (i, name) in ["foo", "bar", "baz"].iter().enumerate() {
                if let NodeKind::Subroutine { name: sub_name, body, .. } = &statements[i].kind {
                    assert_eq!(sub_name.as_deref(), Some(*name));
                    if let NodeKind::Block { statements } = &body.kind {
                        assert!(statements.is_empty());
                    }
                }
            }
        }
    }
}
