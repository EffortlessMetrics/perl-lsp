#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};
    use perl_tdd_support::must_some;

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
            let ast_opt = parse_code(&source);
            assert!(ast_opt.is_some());
            let ast = ast_opt.unwrap_or_else(|| {
                Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
            });
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

    #[test]
    fn test_last_and_redo_with_labels() {
        // Labels should be accepted for all loop-control ops.
        let cases = [("last OUTER;", "last"), ("redo INNER;", "redo")];

        for (source, expected_op) in cases {
            let ast = must_some(parse_code(source));
            assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "expected Program, got {:?}",
                ast.kind
            );
            let NodeKind::Program { statements } = &ast.kind else {
                return;
            };
            let stmt = must_some(statements.first());
            assert!(
                matches!(stmt.kind, NodeKind::LoopControl { .. }),
                "expected LoopControl, got {:?}",
                stmt.kind
            );
            let NodeKind::LoopControl { op, label } = &stmt.kind else {
                return;
            };
            assert_eq!(op, expected_op);
            assert!(label.is_some(), "expected label for source: {source}");
        }
    }

    #[test]
    fn test_loop_control_with_statement_modifier() {
        // Parsing should keep loop-control statements intact when followed by modifiers.
        let source = "next if $should_skip;";
        let ast = must_some(parse_code(source));
        assert!(
            matches!(ast.kind, NodeKind::Program { .. }),
            "expected Program, got {:?}",
            ast.kind
        );
        let NodeKind::Program { statements } = &ast.kind else {
            return;
        };
        let stmt = must_some(statements.first());
        let sexp = stmt.to_sexp();
        assert!(
            sexp.contains("statement_modifier_if"),
            "Expected statement modifier wrapper, got: {sexp}"
        );
        assert!(sexp.contains("next"), "Expected loop control keyword, got: {sexp}");
    }

    #[test]
    fn test_loop_control_in_continue_block() {
        // Continue blocks are a common place for redo/next control flow.
        let source = "while ($x) { $x--; } continue { redo; }";
        let ast = must_some(parse_code(source));

        let sexp = ast.to_sexp();
        assert!(sexp.contains("redo"), "Expected redo in continue block, got: {sexp}");
        assert!(!sexp.contains("ERROR"), "Parse should not emit ERROR nodes: {sexp}");
    }
}
