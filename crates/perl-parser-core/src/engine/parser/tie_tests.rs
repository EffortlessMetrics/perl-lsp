#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::NodeKind;

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_tie_variable() {
        // AC: Tie NodeKind coverage
        let source = "tie %hash, 'MyPackage', @args;";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { variable, package, args } = &expression.kind {
                    if let NodeKind::Variable { sigil, name } = &variable.kind {
                        assert_eq!(sigil, "%");
                        assert_eq!(name, "hash");
                    }
                    if let NodeKind::String { value, .. } = &package.kind {
                        assert!(value.contains("MyPackage"));
                    }
                    assert_eq!(args.len(), 1);
                } else {
                    panic!("Expected Tie node, got {:?}", expression.kind);
                }
            } else {
                panic!("Expected ExpressionStatement, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn test_tie_with_my_declaration() {
        let source = "tie my %h, 'Pkg';";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { variable, .. } = &expression.kind {
                    if let NodeKind::VariableDeclaration { declarator, .. } = &variable.kind {
                        assert_eq!(declarator, "my");
                    } else {
                        panic!("Expected VariableDeclaration, got {:?}", variable.kind);
                    }
                }
            }
        }
    }

    #[test]
    fn test_untie_variable() {
        let source = "untie %hash;";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Untie { variable } = &expression.kind {
                    if let NodeKind::Variable { sigil, name } = &variable.kind {
                        assert_eq!(sigil, "%");
                        assert_eq!(name, "hash");
                    }
                } else {
                    panic!("Expected Untie node, got {:?}", expression.kind);
                }
            }
        }
    }
}
