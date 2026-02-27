#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_named_format() -> Result<(), String> {
        // AC1: recognize format NAME =
        let source = r#"format STDOUT =
Test: @<<<
$test
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "STDOUT");
                assert!(body.contains("Test: @<<<"));
                assert!(body.contains("$test"));
            } else {
                return Err(format!("Expected Format node, got {:?}", stmt.kind));
            }
        }
        Ok(())
    }

    #[test]
    fn test_anonymous_format() -> Result<(), String> {
        // AC6: handle anonymous formats
        let source = r#"format =
Anon: @>>>
$val
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "");
                assert!(body.contains("Anon: @>>>"));
            }
        }
        Ok(())
    }

    #[test]
    fn test_format_terminator() -> Result<(), String> {
        // AC4: recognize standalone . terminator
        let source = r#"format FOO =
.
"#;
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "FOO");
                // The lexer captures the newline before the dot
                assert_eq!(body, "\n");
            }
        }
        Ok(())
    }
}
