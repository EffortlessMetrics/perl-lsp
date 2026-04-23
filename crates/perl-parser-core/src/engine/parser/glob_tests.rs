#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_angle_bracket_glob() {
        // AC2: recognize angle bracket glob syntax <pattern>
        let source = "<*.pl>;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Glob { pattern } = &expression.kind {
                    assert_eq!(pattern, "*.pl");
                } else {
                    unreachable!("Expected Glob node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_complex_glob_patterns() {
        // AC3: glob patterns with wildcards, character classes, and brace expansion
        let patterns = ["**/*.pm", ".[a-z]*", "{a,b,c}*"];
        for p in patterns {
            let source = format!("<{}>;", p);
            let ast_opt = parse_code(&source);
            assert!(ast_opt.is_some());
            let ast = ast_opt.unwrap_or_else(|| {
                Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
            });
            if let NodeKind::Program { statements } = &ast.kind {
                let stmt = &statements[0];
                if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                    if let NodeKind::Glob { pattern } = &expression.kind {
                        assert_eq!(pattern, p);
                    }
                }
            }
        }
    }

    #[test]
    fn test_glob_function() {
        // AC1: recognize glob() function syntax
        let source = "glob('*.txt');";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::FunctionCall { name, args } = &expression.kind {
                    assert_eq!(name, "glob");
                    assert_eq!(args.len(), 1);
                }
            }
        }
    }

    #[test]
    fn test_readline_vs_glob() {
        // AC4: distinguish glob from readline
        let source = "<STDIN>;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                assert!(matches!(expression.kind, NodeKind::Readline { .. }));
            }
        }
    }
}
