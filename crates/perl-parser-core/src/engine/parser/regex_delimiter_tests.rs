#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::NodeKind;

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    #[test]
    fn test_match_bang_delimiter() {
        // AC1: recognize m!pattern!
        let source = "$text =~ m!pattern!;";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Binary { op, right, .. } = &expression.kind {
                    assert_eq!(op, "=~");
                    if let NodeKind::Regex { pattern, .. } = &right.kind {
                        assert_eq!(pattern, "!pattern!");
                    } else {
                        panic!("Expected Regex node, got {:?}", right.kind);
                    }
                }
            }
        }
    }

    #[test]
    fn test_match_brace_delimiter() {
        // AC2: recognize m{pattern} with nested braces
        let source = "$text =~ m{pat{tern}};";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Binary { op, right, .. } = &expression.kind {
                    assert_eq!(op, "=~");
                    if let NodeKind::Regex { pattern, .. } = &right.kind {
                        assert_eq!(pattern, "{pat{tern}}");
                    }
                }
            }
        }
    }

    #[test]
    fn test_substitution_pipe_delimiter() {
        // AC3: recognize s|old|new|g
        let source = "$text =~ s|old|new|g;";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Binary { op, right, .. } = &expression.kind {
                    assert_eq!(op, "=~");
                    if let NodeKind::Substitution { pattern, replacement, modifiers, .. } =
                        &right.kind
                    {
                        assert_eq!(pattern, "old");
                        assert_eq!(replacement, "new");
                        assert!(modifiers.contains('g'));
                    } else {
                        panic!("Expected Substitution node, got {:?}", right.kind);
                    }
                }
            }
        }
    }

    #[test]
    fn test_match_modifiers_bang() {
        // AC4: correct parsing of modifiers after arbitrary delimiters
        let source = "m!pattern!i;";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Regex { modifiers, .. } = &expression.kind {
                    assert_eq!(modifiers, "i");
                }
            }
        }
    }

    #[test]
    fn test_slash_compatibility() {
        // AC6: backward compatibility with slash delimiter
        let source = "/pattern/i;";
        let ast = parse_code(source).unwrap();
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Regex { pattern, modifiers, .. } = &expression.kind {
                    assert_eq!(pattern, "/pattern/");
                    assert_eq!(modifiers, "i");
                }
            }
        }
    }
}
