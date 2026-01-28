#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::NodeKind;

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_named_format() {
        // AC1: recognize format NAME =
        let source = r#"format STDOUT =
Test: @<<<
$test
.
"#;
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "STDOUT");
                assert!(body.contains("Test: @<<<"));
                assert!(body.contains("$test"));
            } else {
                panic!("Expected Format node, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn test_anonymous_format() {
        // AC6: handle anonymous formats
        let source = r#"format =
Anon: @>>>
$val
.
"#;
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "");
                assert!(body.contains("Anon: @>>>"));
            }
        }
    }

    #[test]
    fn test_format_terminator() {
        // AC4: recognize standalone . terminator
        let source = r#"format FOO =
.
"#;
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Format { name, body } = &stmt.kind {
                assert_eq!(name, "FOO");
                // The lexer captures the newline before the dot
                assert_eq!(body, "\n");
            }
        }
    }
}
