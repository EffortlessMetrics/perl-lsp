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

    #[test]
    fn test_tie_scalar() {
        let source = "tie my $scalar, 'MyScalar::Tie';";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { variable, package, args } = &expression.kind {
                    if let NodeKind::VariableDeclaration { declarator, .. } = &variable.kind {
                        assert_eq!(declarator, "my");
                    } else {
                        unreachable!("Expected VariableDeclaration, got {:?}", variable.kind);
                    }
                    if let NodeKind::String { value, .. } = &package.kind {
                        assert!(value.contains("MyScalar::Tie"));
                    }
                    assert!(args.is_empty());
                } else {
                    unreachable!("Expected Tie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_tie_array() {
        let source = "tie my @array, 'MyArray::Tie', @args;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { variable, package, args } = &expression.kind {
                    if let NodeKind::VariableDeclaration { declarator, .. } = &variable.kind {
                        assert_eq!(declarator, "my");
                    }
                    if let NodeKind::String { value, .. } = &package.kind {
                        assert!(value.contains("MyArray::Tie"));
                    }
                    assert_eq!(args.len(), 1);
                } else {
                    unreachable!("Expected Tie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_tie_glob() {
        let source = "tie *FH, 'Tie::Handle';";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { package, .. } = &expression.kind {
                    if let NodeKind::String { value, .. } = &package.kind {
                        assert!(value.contains("Tie::Handle"));
                    }
                } else {
                    unreachable!("Expected Tie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_tie_with_multiple_args() {
        let source = "tie %hash, 'DB_File', $filename, 0, 0644;";
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
                        assert!(value.contains("DB_File"));
                    }
                    assert_eq!(args.len(), 3);
                } else {
                    unreachable!("Expected Tie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_untie_scalar() {
        let source = "untie $scalar;";
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
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "scalar");
                    }
                } else {
                    unreachable!("Expected Untie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_untie_array() {
        let source = "untie @array;";
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
                        assert_eq!(sigil, "@");
                        assert_eq!(name, "array");
                    }
                } else {
                    unreachable!("Expected Untie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_untie_glob() {
        let source = "untie *FH;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Untie { .. } = &expression.kind {
                    // Untie with glob parses successfully
                } else {
                    unreachable!("Expected Untie node, got {:?}", expression.kind);
                }
            }
        }
    }

    #[test]
    fn test_tied_function() {
        // `tied` is a builtin function, not a special AST node
        let source = "my $obj = tied %hash;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            assert!(!statements.is_empty(), "tied statement should parse");
        }
    }

    #[test]
    fn test_tied_in_conditional() {
        let source = "if (tied @array) { print 1; }";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            assert!(!statements.is_empty(), "tied in conditional should parse");
        }
    }

    #[test]
    fn test_tie_with_our_declaration() {
        let source = "tie our %config, 'Config::Tie';";
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
                        assert_eq!(declarator, "our");
                    } else {
                        unreachable!("Expected VariableDeclaration, got {:?}", variable.kind);
                    }
                }
            }
        }
    }

    #[test]
    fn test_tie_with_local() {
        let source = "tie local %ENV, 'Tie::EnvHash';";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            assert!(!statements.is_empty(), "tie with local should parse");
        }
    }

    #[test]
    fn test_tie_with_local_special_var_and_arg() {
        // Real-world pattern from corpus: tie the input record separator.
        let source = "tie local $/, 'Tie::Scalar', \\$custom_rs;";
        let ast_opt = parse_code(source);
        assert!(ast_opt.is_some());
        let ast = ast_opt.unwrap_or_else(|| {
            Node::new(NodeKind::UnknownRest, SourceLocation { start: 0, end: 0 })
        });
        if let NodeKind::Program { statements } = &ast.kind {
            let stmt = &statements[0];
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Tie { variable, package, args } = &expression.kind {
                    if let NodeKind::VariableDeclaration { declarator, variable: inner, .. } =
                        &variable.kind
                    {
                        assert_eq!(declarator, "local");
                        if let NodeKind::Variable { sigil, name } = &inner.kind {
                            assert_eq!(sigil, "$");
                            assert_eq!(name, "/");
                        } else {
                            unreachable!("Expected inner Variable, got {:?}", inner.kind);
                        }
                    } else {
                        unreachable!("Expected VariableDeclaration, got {:?}", variable.kind);
                    }
                    if let NodeKind::String { value, .. } = &package.kind {
                        assert!(value.contains("Tie::Scalar"));
                    } else {
                        unreachable!("Expected String package, got {:?}", package.kind);
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
}
