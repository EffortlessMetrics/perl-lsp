#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_tie_variable() {
        // AC: Tie NodeKind coverage
        let source = "tie %hash, 'MyPackage', @args;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
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
                    unreachable!("Expected Tie node, got {:?}", expression.kind);
                }
            } else {
                unreachable!("Expected ExpressionStatement, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn test_tie_with_my_declaration() {
        let source = "tie my %h, 'Pkg';";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { variable, .. } = &expression.kind {
                    if let NodeKind::VariableDeclaration { declarator, .. } = &variable.kind {
                        assert_eq!(declarator, "my");
                    } else {
                        unreachable!("Expected VariableDeclaration, got {:?}", variable.kind);
                    }
                }
            }
        }
    }

    #[test]
    fn test_untie_variable() {
        let source = "untie %hash;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Untie { variable } = &expression.kind {
                    if let NodeKind::Variable { sigil, name } = &variable.kind {
                        assert_eq!(sigil, "%");
                        assert_eq!(name, "hash");
                    }
                } else {
                    unreachable!("Expected Untie node, got {:?}", expression.kind);
                }
            }
        }
    }
}
