#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::{Node, NodeKind, SourceLocation};
    use perl_tdd_support::must;

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    fn collect_typeglob_names(node: &Node, names: &mut Vec<String>) {
        if let NodeKind::Typeglob { name } = &node.kind {
            names.push(name.clone());
        }

        for child in node.children() {
            collect_typeglob_names(child, names);
        }
    }

    fn contains_error_node(node: &Node) -> bool {
        matches!(node.kind, NodeKind::Error { .. })
            || node.children().into_iter().any(contains_error_node)
    }

    #[test]
    fn test_typeglob_simple_assignment() {
        // AC1: recognize *foo = *bar
        let ast_opt = parse_code("*foo = *bar;");
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Assignment { lhs, rhs, op } = &expression.kind {
                    assert_eq!(op, "=");
                    if let NodeKind::Typeglob { name } = &lhs.kind {
                        assert_eq!(name, "foo");
                    } else {
                        unreachable!("Expected Typeglob on LHS, got {:?}", lhs.kind);
                    }

                    if let NodeKind::Typeglob { name } = &rhs.kind {
                        assert_eq!(name, "bar");
                    } else {
                        unreachable!("Expected Typeglob on RHS, got {:?}", rhs.kind);
                    }
                } else {
                    unreachable!("Expected Assignment, got {:?}", expression.kind);
                }
            } else {
                unreachable!("Expected ExpressionStatement, got {:?}", stmt.kind);
            }
        }
    }

    #[test]
    fn test_typeglob_reference_assignment() {
        // AC2: handle *foo = \&sub
        let ast_opt = parse_code("*foo = \\&sub;");
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
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
        let ast_opt = parse_code("*{$name} = \\&func;");
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::Assignment { lhs, .. } = &stmt.kind {
                // Currently implemented as Unary(*) of Block
                if let NodeKind::Unary { op, .. } = &lhs.kind {
                    assert_eq!(op, "*");
                } else {
                    unreachable!("Expected Unary(*), got {:?}", lhs.kind);
                }
            }
        }
    }

    #[test]
    fn test_typeglob_dereference() {
        // AC5: Parser handles typeglob dereferencing (${*foo}, @{*bar})
        let ast_opt = parse_code("${*foo};");
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            // ${*foo} parses as Variable($) with Name as Typeglob
            if let NodeKind::Variable { sigil, name: _ } = &stmt.kind {
                assert_eq!(sigil, "$");
            }
        }
    }

    #[test]
    fn test_english_special_punctuation_typeglobs() {
        let cases = [
            ("*LIST_SEPARATOR = *\";", "\""),
            ("*PREMATCH = *`;", "`"),
            ("*POSTMATCH = *';", "'"),
            ("*SUBSCRIPT_SEPARATOR = *;;", ";"),
            ("*FORMAT_PAGE_NUMBER = *%;", "%"),
            ("*FORMAT_NAME = *~;", "~"),
            ("*FORMAT_TOP_NAME = *^;", "^"),
            ("*MATCH = *&;", "&"),
            ("*OS_ERROR = *!;", "!"),
            ("*OLD_PERL_VERSION = *];", "]"),
            ("*EVAL_ERROR = *@ ;", "@"),
            ("*PROCESS_ID = *$ ;", "$"),
            ("*OUTPUT_RECORD_SEPARATOR = *\\;", "\\"),
        ];

        for (source, expected_rhs) in cases {
            let mut parser = Parser::new(source);
            let ast = must(parser.parse());
            let mut typeglobs = Vec::new();
            collect_typeglob_names(&ast, &mut typeglobs);

            assert!(
                !contains_error_node(&ast),
                "special punctuation typeglob should not produce Error nodes in {source:?}"
            );
            assert!(
                typeglobs.iter().any(|name| name == expected_rhs),
                "missing RHS typeglob {expected_rhs:?} in {source:?}: {typeglobs:?}"
            );
        }
    }
}
