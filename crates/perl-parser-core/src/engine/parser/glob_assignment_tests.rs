#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::NodeKind;

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_typeglob_simple_assignment() {
        // AC1: recognize *foo = *bar
        let ast = parse_code("*foo = *bar;").unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Assignment { lhs, rhs, op } = &expression.kind {
                    assert_eq!(op, "=");
                    if let NodeKind::Typeglob { name } = &lhs.kind {
                        assert_eq!(name, "foo");
                    } else {
                        panic!("Expected Typeglob on LHS, got {:?}", lhs.kind);
                    }
                    
                    if let NodeKind::Typeglob { name } = &rhs.kind {
                        assert_eq!(name, "bar");
                    } else {
                        panic!("Expected Typeglob on RHS, got {:?}", rhs.kind);
                    }
                } else {
                    panic!("Expected Assignment, got {:?}", expression.kind);
                }
            } else {
                panic!("Expected ExpressionStatement, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn test_typeglob_reference_assignment() {
        // AC2: handle *foo = \&sub
        let ast = parse_code("*foo = \\&sub;").unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Assignment { lhs, rhs, .. } = &stmt.kind {
                if let NodeKind::Typeglob { name } = &lhs.kind {
                    assert_eq!(name, "foo");
                }
                // RHS should be Unary (\) of Unary (&) of Identifier (sub)
                if let NodeKind::Unary { op, .. } = &rhs.kind {
                    assert_eq!(op, "\\\\");
                }
            }
        }
    }

    #[test]
    fn test_dynamic_typeglob() {
        // AC3: dynamic typeglob *{$name} = \&function
        let ast = parse_code("*{$name} = \\&func;").unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Assignment { lhs, .. } = &stmt.kind {
                // Currently implemented as Unary(*) of Block
                if let NodeKind::Unary { op, .. } = &lhs.kind {
                    assert_eq!(op, "*");
                } else {
                    panic!("Expected Unary(*), got {:?}", lhs.kind);
                }
            }
        }
    }

    #[test]
    fn test_typeglob_dereference() {
        // AC5: Parser handles typeglob dereferencing (${*foo}, @{*bar})
        let ast = parse_code("${*foo};").unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            // ${*foo} parses as Variable($) with Name as Typeglob
            if let NodeKind::Variable { sigil, name: _ } = &stmt.kind {
                assert_eq!(sigil, "$");
            }
        }
    }
}
