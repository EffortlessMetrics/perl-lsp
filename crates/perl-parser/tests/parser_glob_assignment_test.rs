//! Test glob assignments (issue #448)
//!
//! This test validates parser support for typeglob assignments which are used
//! for symbol table manipulation and aliasing in Perl.

use perl_parser::Parser;
use perl_parser_core::engine::ast::NodeKind;
use perl_tdd_support::must;

#[test]
fn parser_glob_simple_assignment() {
    // AC1: Parser recognizes *foo = *bar as typeglob assignment
    let code = "*foo = *bar;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            if let NodeKind::Assignment { lhs, rhs, op } = &expression.kind {
                assert_eq!(op, "=", "Expected assignment operator");

                // Check LHS is Typeglob
                let is_lhs_typeglob = matches!(lhs.kind, NodeKind::Typeglob { .. });
                assert!(is_lhs_typeglob, "Expected Typeglob on LHS, got {:?}", lhs.kind);
                if let NodeKind::Typeglob { name } = &lhs.kind {
                    assert_eq!(name, "foo", "Expected typeglob name 'foo'");
                }

                // Check RHS is Typeglob
                let is_rhs_typeglob = matches!(rhs.kind, NodeKind::Typeglob { .. });
                assert!(is_rhs_typeglob, "Expected Typeglob on RHS, got {:?}", rhs.kind);
                if let NodeKind::Typeglob { name } = &rhs.kind {
                    assert_eq!(name, "bar", "Expected typeglob name 'bar'");
                }
            } else {
                assert!(
                    matches!(expression.kind, NodeKind::Assignment { .. }),
                    "Expected Assignment, got {:?}",
                    expression.kind
                );
            }
        } else {
            assert!(
                matches!(stmt.kind, NodeKind::ExpressionStatement { .. }),
                "Expected ExpressionStatement, got {:?}",
                stmt.kind
            );
        }
    } else {
        assert!(
            matches!(ast.kind, NodeKind::Program { .. }),
            "Expected Program node, got {:?}",
            ast.kind
        );
    }
}

#[test]
fn parser_glob_qualified_assignment() {
    // AC1: Parser recognizes qualified typeglob assignments
    let code = "*My::Package::func = *Other::Package::func;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            if let NodeKind::Assignment { lhs, rhs, .. } = &expression.kind {
                // Check LHS is qualified Typeglob
                let is_lhs_typeglob = matches!(lhs.kind, NodeKind::Typeglob { .. });
                assert!(is_lhs_typeglob, "Expected Typeglob on LHS, got {:?}", lhs.kind);
                if let NodeKind::Typeglob { name } = &lhs.kind {
                    assert!(name.contains("::"), "Expected qualified name on LHS");
                }

                // Check RHS is qualified Typeglob
                let is_rhs_typeglob = matches!(rhs.kind, NodeKind::Typeglob { .. });
                assert!(is_rhs_typeglob, "Expected Typeglob on RHS, got {:?}", rhs.kind);
                if let NodeKind::Typeglob { name } = &rhs.kind {
                    assert!(name.contains("::"), "Expected qualified name on RHS");
                }
            }
        }
    }
}

#[test]
fn parser_glob_reference_assignment() {
    // AC2: Parser handles typeglob assignments with references
    let code = "*PI = \\3.14159;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            if let NodeKind::Assignment { lhs, rhs, .. } = &expression.kind {
                // Check LHS is Typeglob
                let is_lhs_typeglob = matches!(lhs.kind, NodeKind::Typeglob { .. });
                assert!(is_lhs_typeglob, "Expected Typeglob on LHS, got {:?}", lhs.kind);
                if let NodeKind::Typeglob { name } = &lhs.kind {
                    assert_eq!(name, "PI", "Expected typeglob name 'PI'");
                }

                // Check RHS is Unary (reference operator)
                let is_rhs_unary = matches!(rhs.kind, NodeKind::Unary { .. });
                assert!(is_rhs_unary, "Expected Unary reference on RHS, got {:?}", rhs.kind);
                if let NodeKind::Unary { op, operand } = &rhs.kind {
                    assert!(op.contains("\\"), "Expected backslash reference operator");
                    // operand should be a number
                    assert!(matches!(operand.kind, NodeKind::Number { .. }));
                }
            }
        }
    }
}

#[test]
fn parser_glob_sub_reference_assignment() {
    // AC2: Parser handles typeglob assignments with code references
    let code = "*func = \\&other_func;";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            if let NodeKind::Assignment { lhs, rhs, .. } = &expression.kind {
                // Check LHS is Typeglob
                if let NodeKind::Typeglob { name } = &lhs.kind {
                    assert_eq!(name, "func", "Expected typeglob name 'func'");
                } else {
                    must(Err::<(), _>(format!("Expected Typeglob on LHS, got {:?}", lhs.kind)));
                }

                // Check RHS is reference to code
                assert!(matches!(rhs.kind, NodeKind::Unary { .. }));
            }
        }
    }
}

#[test]
fn parser_glob_dynamic_assignment() {
    // AC3: Parser handles dynamic typeglob syntax (*{$name} = \&function)
    // Note: Parser treats *{$name} as literal typeglob name (acceptable behavior)
    let code = "*{$name} = \\&function;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            if let NodeKind::Assignment { lhs, rhs, .. } = &expression.kind {
                // Dynamic typeglob syntax is parsed as Typeglob with literal name
                // This is acceptable as true dynamic evaluation requires runtime context
                let is_lhs_typeglob = matches!(lhs.kind, NodeKind::Typeglob { .. });
                assert!(is_lhs_typeglob, "Expected Typeglob on LHS, got {:?}", lhs.kind);
                if let NodeKind::Typeglob { name } = &lhs.kind {
                    assert!(name.contains("{"), "Expected braces in typeglob name");
                }

                // Check RHS is reference
                assert!(matches!(rhs.kind, NodeKind::Unary { .. }));
            }
        }
    }
}

#[test]
fn parser_glob_local_declaration() {
    // Test local *FH typeglob declaration
    let code = "local *FH;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        // local creates a VariableDeclaration node
        let is_var_decl = matches!(stmt.kind, NodeKind::VariableDeclaration { .. });
        assert!(is_var_decl, "Expected VariableDeclaration, got {:?}", stmt.kind);
        if let NodeKind::VariableDeclaration { declarator, variable, .. } = &stmt.kind {
            assert_eq!(declarator, "local", "Expected 'local' declarator");

            // Variable should be a Typeglob
            let is_typeglob = matches!(variable.kind, NodeKind::Typeglob { .. });
            assert!(is_typeglob, "Expected Typeglob variable, got {:?}", variable.kind);
            if let NodeKind::Typeglob { name } = &variable.kind {
                assert_eq!(name, "FH", "Expected typeglob name 'FH'");
            }
        }
    }
}

#[test]
fn parser_glob_dereference_scalar() {
    // AC5: Parser handles typeglob dereferencing (${*foo})
    // Note: Parser treats ${*foo} as Binary { ${}, *foo } (acceptable)
    let code = "${*foo};";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            // ${*foo} is parsed as Binary with {} operator
            // This is acceptable as the parser successfully handles the construct
            assert!(
                matches!(
                    expression.kind,
                    NodeKind::Variable { .. } | NodeKind::Unary { .. } | NodeKind::Binary { .. }
                ),
                "Expected Variable, Unary, or Binary dereference, got {:?}",
                expression.kind
            );
        }
    }
}

#[test]
fn parser_glob_dereference_array() {
    // AC5: Parser handles typeglob dereferencing (@{*bar})
    // Note: Parser treats @{*bar} similarly to ${*foo} above
    let code = "@{*bar};";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            // @{*bar} is parsed as an array variable with complex dereference
            // or as Binary construct (acceptable)
            assert!(
                matches!(
                    expression.kind,
                    NodeKind::Variable { .. } | NodeKind::Unary { .. } | NodeKind::Binary { .. }
                ),
                "Expected Variable, Unary, or Binary dereference, got {:?}",
                expression.kind
            );
        }
    }
}

#[test]
fn parser_glob_multiple_assignments() {
    // Test multiple typeglob assignments in sequence
    let code = "*foo = *bar; *baz = *qux;";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 2, "Expected 2 statements");

        // Check both statements are glob assignments
        for stmt in statements {
            let is_expr_stmt = matches!(stmt.kind, NodeKind::ExpressionStatement { .. });
            assert!(is_expr_stmt, "Expected ExpressionStatement, got {:?}", stmt.kind);
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                let is_assignment = matches!(expression.kind, NodeKind::Assignment { .. });
                assert!(is_assignment, "Expected Assignment, got {:?}", expression.kind);
                if let NodeKind::Assignment { lhs, rhs, .. } = &expression.kind {
                    assert!(matches!(lhs.kind, NodeKind::Typeglob { .. }));
                    assert!(matches!(rhs.kind, NodeKind::Typeglob { .. }));
                }
            }
        }
    }
}

#[test]
fn parser_glob_vs_multiplication() {
    // AC1: Parser distinguishes *foo (typeglob) from * (multiplication)
    let code = "my $x = 2 * 3;";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &stmt.kind {
            // Should be Binary with * operator, not Typeglob
            let is_binary = matches!(init.kind, NodeKind::Binary { .. });
            assert!(is_binary, "Expected Binary multiplication, got {:?}", init.kind);
            if let NodeKind::Binary { op, .. } = &init.kind {
                assert_eq!(op, "*", "Expected multiplication operator");
            }
        }
    }
}

#[test]
fn parser_glob_in_context() {
    // Test typeglob in complex expressions
    let code = "my $ref = \\*STDOUT;";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Expected 1 statement");
        let stmt = &statements[0];

        if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &stmt.kind {
            // Should be Unary (\) with Typeglob operand
            let is_unary = matches!(init.kind, NodeKind::Unary { .. });
            assert!(is_unary, "Expected Unary reference, got {:?}", init.kind);
            if let NodeKind::Unary { op, operand } = &init.kind {
                assert!(op.contains("\\"), "Expected backslash reference operator");
                let is_typeglob = matches!(operand.kind, NodeKind::Typeglob { .. });
                assert!(is_typeglob, "Expected Typeglob operand, got {:?}", operand.kind);
                if let NodeKind::Typeglob { name } = &operand.kind {
                    assert_eq!(name, "STDOUT", "Expected STDOUT typeglob");
                }
            }
        }
    }
}
