#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_next_last_redo_simple() {
        // AC1: recognize next, last, redo keywords
        let keywords = ["next", "last", "redo"];
        for kw in keywords {
            let source = format!("{};", kw);
            let ast = parse_code(&source).unwrap();
            if let NodeKind::Program { statements } = &ast.kind {
                let stmt = &statements[0];
                if let NodeKind::LoopControl { op, label } = &stmt.kind {
                    assert_eq!(op, kw);
                    assert!(label.is_none());
                } else {
                    unreachable!("Expected LoopControl, got {:?}", stmt.kind);
                }
            }
        }
    }

    #[test]
    fn test_loop_control_with_label() {
        // AC3: labels supported for continue/redo
        let source = "next OUTER;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::LoopControl { op, label } = &stmt.kind {
                assert_eq!(op, "next");
                assert_eq!(label.as_deref(), Some("OUTER"));
            }
        }
    }

    #[test]
    fn test_loop_control_in_while() {
        // AC2: Correct parsing in while loop
        let source = "while (1) { last; }";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let while_stmt = &statements[0];
            if let NodeKind::While { body, .. } = &while_stmt.kind {
                if let NodeKind::Block { statements } = &body.kind {
                    let last_stmt = &statements[0];
                    assert!(matches!(last_stmt.kind, NodeKind::LoopControl { .. }));
                }
            }
        }
    }
}
