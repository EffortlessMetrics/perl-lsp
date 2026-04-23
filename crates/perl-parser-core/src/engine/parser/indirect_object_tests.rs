#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_general_indirect_method_call() {
        // AC1: recognize method $object @args
        let source = "move $player 10, 20;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::IndirectCall { method, object, args } = &stmt.kind {
                assert_eq!(method, "move");
                if let NodeKind::Variable { sigil, name } = &object.kind {
                    assert_eq!(sigil, "$");
                    assert_eq!(name, "player");
                } else {
                    unreachable!("Expected Variable as object, got {:?}", object.kind);
                }
                // Arguments are parsed until statement terminator
                assert_eq!(args.len(), 2);
            } else {
                unreachable!("Expected IndirectCall node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn test_builtin_indirect_syntax() {
        // AC2: handle builtin indirect syntax (print $fh "text")
        let source = "print $fh \"Hello\";";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::IndirectCall { method, object, args } = &stmt.kind {
                assert_eq!(method, "print");
                if let NodeKind::Variable { sigil, name } = &object.kind {
                    assert_eq!(sigil, "$");
                    assert_eq!(name, "fh");
                }
                assert_eq!(args.len(), 1);
            }
        }
    }

    #[test]
    fn test_new_indirect_syntax() {
        // AC1 variant: new Class(...)
        let source = "new Player \"Steven\";";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::IndirectCall { method, object, .. } = &stmt.kind {
                assert_eq!(method, "new");
                if let NodeKind::Identifier { name } = &object.kind {
                    assert_eq!(name, "Player");
                }
            }
        }
    }
}
