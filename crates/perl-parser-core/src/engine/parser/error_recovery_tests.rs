use super::*;
use perl_tdd_support::must;

#[test]
fn test_recovery_missing_expression() {
    // Phase 2 recovery: missing RHS after `=` emits Recovered { InfixRhs, MissingOperand }
    // and produces a VariableDeclaration with MissingExpression initializer — not an Error node.
    let code = "my $x = ; print 1;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("AST: {}", ast.to_sexp());

            // Check that we have 2 statements
            if let NodeKind::Program { statements } = &ast.kind {
                assert_eq!(
                    statements.len(),
                    2,
                    "Should have 2 statements (1 recovered decl, 1 valid)"
                );

                // Phase 2: first statement is a VariableDeclaration with MissingExpression
                // (not a raw Error node — Phase 2 recovers with a structured node)
                assert!(
                    matches!(
                        statements[0].kind,
                        NodeKind::VariableDeclaration { .. } | NodeKind::Error { .. }
                    ),
                    "Expected VariableDeclaration or Error for first statement, got: {:?}",
                    statements[0].kind
                );

                // Second statement should be ExpressionStatement
                match &statements[1].kind {
                    NodeKind::ExpressionStatement { .. } => {
                        println!("Found valid second statement");
                    }
                    _ => unreachable!(
                        "Expected ExpressionStatement for second statement, got: {:?}",
                        statements[1].kind
                    ),
                }
            } else {
                unreachable!("Expected Program node");
            }

            // Check errors list — at minimum one Recovered error
            let errors = parser.errors();
            assert!(!errors.is_empty(), "Should have recorded errors");
            println!("Errors: {:?}", errors);
        }
        Err(e) => {
            unreachable!("Parser failed to recover: {}", e);
        }
    }
}

#[test]
fn test_recovery_missing_rhs_before_sub_declaration_keyword() {
    // Missing RHS before `sub foo { ... }` should recover at `sub` as a
    // statement boundary, instead of consuming `sub` as an identifier.
    let code = "my $x = sub foo { print 1; }";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should recover missing assignment RHS");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 2, "Should recover and keep the following sub declaration");
        assert!(
            matches!(statements[0].kind, NodeKind::VariableDeclaration { .. }),
            "First statement should stay a recovered variable declaration"
        );
        assert!(
            matches!(statements[1].kind, NodeKind::Subroutine { .. }),
            "Second statement should parse as subroutine declaration"
        );
    } else {
        unreachable!("Expected program root");
    }

    assert!(!parser.errors().is_empty(), "Recovery should record a missing operand diagnostic");
}

#[test]
fn test_no_recovery_for_anonymous_sub_assignment_rhs() {
    let code = "local $SIG{__WARN__} = sub { };";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should accept anonymous sub assignment RHS");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "Anonymous sub RHS should stay in a single statement");
    } else {
        unreachable!("Expected program root");
    }

    assert!(
        !ast.to_sexp().contains("missing_expression"),
        "Anonymous sub assignment should not create MissingExpression recovery nodes"
    );
}

#[test]
fn test_recovery_multiple_errors() {
    // Phase 2: `my $x = ;` now produces a VariableDeclaration with MissingExpression
    // instead of an Error node. Each missing RHS emits exactly 1 Recovered error.
    let code = "
        my $a = ;   # Recovered 1
        print 1;    # Valid
        my $b = ;   # Recovered 2
        print 2;    # Valid
    ";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 4, "Should have 4 statements");
        // Phase 2: VariableDeclaration with MissingExpression (not raw Error)
        assert!(
            matches!(
                statements[0].kind,
                NodeKind::VariableDeclaration { .. } | NodeKind::Error { .. }
            ),
            "Expected VariableDeclaration or Error, got: {:?}",
            statements[0].kind
        );
        assert!(matches!(statements[1].kind, NodeKind::ExpressionStatement { .. }));
        assert!(
            matches!(
                statements[2].kind,
                NodeKind::VariableDeclaration { .. } | NodeKind::Error { .. }
            ),
            "Expected VariableDeclaration or Error, got: {:?}",
            statements[2].kind
        );
        assert!(matches!(statements[3].kind, NodeKind::ExpressionStatement { .. }));
    }

    // Phase 2: 2 Recovered errors (one per missing operand), down from 4 raw errors
    assert!(!parser.errors().is_empty(), "Should have errors");
    assert!(parser.errors().len() >= 2, "Expected at least 2 errors, got: {:?}", parser.errors());
}

#[test]
fn test_recovery_inside_block() {
    // Phase 2: `my $x = ;` inside a block produces VariableDeclaration with MissingExpression
    let code = "sub foo { my $x = ; print 1; }";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            // Structure: Program -> Subroutine -> Block -> [VariableDeclaration, ExpressionStatement]
            if let NodeKind::Program { statements } = &ast.kind {
                assert_eq!(statements.len(), 1);

                if let NodeKind::Subroutine { body, .. } = &statements[0].kind {
                    if let NodeKind::Block { statements } = &body.kind {
                        assert_eq!(
                            statements.len(),
                            2,
                            "Block should have 2 statements (1 recovered decl, 1 valid)"
                        );

                        // Phase 2: first stmt is VariableDeclaration (not Error)
                        assert!(
                            matches!(
                                statements[0].kind,
                                NodeKind::VariableDeclaration { .. } | NodeKind::Error { .. }
                            ),
                            "Expected VariableDeclaration or Error in block, got: {:?}",
                            statements[0].kind
                        );

                        match &statements[1].kind {
                            NodeKind::ExpressionStatement { .. } => {
                                println!("Found valid statement in block")
                            }
                            _ => unreachable!("Expected ExpressionStatement in block"),
                        }
                    } else {
                        unreachable!("Expected Block in subroutine body");
                    }
                } else {
                    unreachable!("Expected Subroutine node, got: {:?}", statements[0].kind);
                }
            }

            assert!(!parser.errors().is_empty());
        }
        Err(e) => unreachable!("Failed to recover from block error: {}", e),
    }
}

// Issue #451: AC1 - Parser maintains internal errors collection
#[test]
fn test_451_ac1_maintains_error_collection() {
    let code = "my $x = ; my $y = 10;";
    let mut parser = Parser::new(code);
    let _result = parser.parse();

    let errors = parser.errors();
    assert!(!errors.is_empty(), "AC1: Parser should maintain errors collection");
}

// Issue #451: AC2 - parse_with_recovery method returns both AST and errors
#[test]
fn test_451_ac2_parse_with_recovery_method() {
    let code = "my $x = ; print 1;";
    let mut parser = Parser::new(code);

    let output = parser.parse_with_recovery();

    assert!(matches!(output.ast.kind, NodeKind::Program { .. }), "AC2: Should return AST");
    assert!(!output.diagnostics.is_empty(), "AC2: Should return collected errors");
}

// Issue #451: AC3 - ParseOutput includes ast and diagnostics fields
#[test]
fn test_451_ac3_parse_output_structure() {
    let code = "my $x = ;";
    let mut parser = Parser::new(code);
    let output = parser.parse_with_recovery();

    assert!(matches!(output.ast.kind, NodeKind::Program { .. }), "AC3: ast field present");
    assert!(!output.diagnostics.is_empty(), "AC3: diagnostics field present");
}

// Issue #451: AC4 - Parser continues after storing error (non-fail-fast)
#[test]
fn test_451_ac4_continues_after_error() {
    let code = "my $a = ; print 'hello'; my $b = ; print 'world';";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "AC4: Parser should continue after errors");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 4, "AC4: Should continue parsing after each error");
    }
}

// Issue #451: AC5 - Error limit enforcement prevents unbounded collection
#[test]
fn test_451_ac5_error_limit_enforcement() {
    let mut code = String::new();
    for i in 0..150 {
        code.push_str(&format!("my $x{} = ;\n", i));
    }

    let mut parser = Parser::new(&code);
    let _result = parser.parse();

    let errors = parser.errors();
    assert!(errors.len() < 500, "AC5: Should limit error collection (found {})", errors.len());
}

// Issue #451: AC6 - Recovery doesn't recurse infinitely
#[test]
fn test_451_ac6_recovery_prevents_infinite_loops() {
    // Test that recovery has bounded behavior even with pathological input
    let code = ";;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Should complete successfully without hanging or stack overflow
    assert!(result.is_ok(), "AC6: Recovery should complete on pathological input");

    // Test with many syntax errors that recovery can handle
    let code2 = "{ { { { { { { { { {";
    let mut parser2 = Parser::new(code2);
    let result2 = parser2.parse();

    // Should complete without infinite recursion
    assert!(result2.is_ok(), "AC6: Should handle nested unclosed blocks");
}

// Issue #451: AC7 - Statement-level parsing collects errors and continues
#[test]
fn test_451_ac7_statement_level_recovery() {
    // Phase 2: `my $bad = ;` produces VariableDeclaration with MissingExpression,
    // not a raw Error node. The parser still recovers and parses all statements.
    let code = "
        print 1;
        my $bad = ;
        print 2;
        my $good = 42;
    ";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "AC7: Statement-level parsing should recover");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 4, "AC7: Should parse all statements");

        // Phase 2: the bad declaration is recovered as VariableDeclaration, not Error.
        // We still verify recovery happened via the errors collection.
        let has_valid = statements.iter().any(|s| !matches!(s.kind, NodeKind::Error { .. }));
        assert!(has_valid, "AC7: Should have valid statements after error");
    }
    assert!(!parser.errors().is_empty(), "AC7: Should have recorded errors");
}

// Issue #451: AC8 - Expression-level recovery creates error nodes
#[test]
fn test_451_ac8_expression_level_recovery() {
    // Phase 2: `my $x = ;` now recovers with a VariableDeclaration+MissingExpression
    // instead of a raw Error node. The errors collection still has a Recovered entry.
    let code = "my $x = ;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "AC8: Should recover from expression errors");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert!(!statements.is_empty(), "AC8: Should have statement");

        // Phase 2: either a VariableDeclaration (new, better) or Error (old fallback)
        let has_recovered = statements.iter().any(|s| {
            matches!(
                s.kind,
                NodeKind::VariableDeclaration { .. }
                    | NodeKind::Error { .. }
                    | NodeKind::MissingExpression
            )
        });
        assert!(has_recovered, "AC8: Should produce a recovered or error node");
    }

    assert!(!parser.errors().is_empty(), "AC8: Should record expression-level error");
}

// Issue #451: AC9 - Block-level parsing collects errors for each statement
#[test]
fn test_451_ac9_block_level_recovery() {
    // Phase 2: `my $a = ;` inside a block produces VariableDeclaration+MissingExpression.
    // The block still has all 4 statements and the errors collection has Recovered entries.
    let code = "
        sub test {
            my $a = ;
            print 1;
            my $b = ;
            print 2;
        }
    ";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "AC9: Block-level parsing should recover");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        if let Some(sub_node) = statements.first() {
            if let NodeKind::Subroutine { body, .. } = &sub_node.kind {
                if let NodeKind::Block { statements: block_stmts } = &body.kind {
                    assert_eq!(block_stmts.len(), 4, "AC9: Block should have all statements");

                    // Phase 2: the bad declarations are VariableDeclaration (not Error).
                    // The two print statements are ExpressionStatement.
                    let print_count = block_stmts
                        .iter()
                        .filter(|s| matches!(s.kind, NodeKind::ExpressionStatement { .. }))
                        .count();
                    assert_eq!(
                        print_count, 2,
                        "AC9: Should have 2 valid ExpressionStatement in block"
                    );
                }
            }
        }
    }

    let errors = parser.errors();
    assert!(errors.len() >= 2, "AC9: Should collect multiple errors from block");
}

// Issue #451: AC10 - Multiple error collection scenarios
#[test]
fn test_451_ac10_comprehensive_scenarios() {
    // Scenario 1: Interleaved errors and valid code
    let code1 = "
        my $a = ;
        print 'valid';
        my $b = ;
        my $c = 10;
        my $d = ;
    ";
    let mut parser1 = Parser::new(code1);
    let result1 = parser1.parse();
    assert!(result1.is_ok(), "AC10: Should handle interleaved errors");
    assert!(parser1.errors().len() >= 3, "AC10: Should collect all 3 errors");

    // Scenario 2: Nested blocks with errors
    let code2 = "
        if (1) {
            my $x = ;
            print 1;
        }
        while (1) {
            my $y = ;
            print 2;
        }
    ";
    let mut parser2 = Parser::new(code2);
    let result2 = parser2.parse();
    assert!(result2.is_ok(), "AC10: Should handle nested block errors");
    assert!(parser2.errors().len() >= 2, "AC10: Should collect errors from nested blocks");

    // Scenario 3: Different error types
    let code3 = "my $x = ; my $y = ";
    let mut parser3 = Parser::new(code3);
    let _result3 = parser3.parse();
    assert!(!parser3.errors().is_empty(), "AC10: Should handle different error types");
}

#[test]
fn test_no_recovery_for_my_code_eq_anon_sub() {
    // Regression test for #5017: `my $code = sub { ... }` must parse as a
    // single VariableDeclaration with an AnonymousSubroutineExpression RHS.
    // Before the fix, this produced MissingExpression + dangling anon-sub.
    let code = "my $code = sub { my ($x) = @_; return $x * 2; };";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should accept `my $var = sub { ... };`");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(
            statements.len(),
            1,
            "my $code = sub {{...}} must be a single statement, not split in two"
        );
        // Verify no error nodes and no missing_expression
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("missing_expression"),
            "RHS anonymous sub must not produce MissingExpression: {sexp}"
        );
        assert!(
            !sexp.contains("error"),
            "RHS anonymous sub must not produce Error nodes: {sexp}"
        );
    } else {
        unreachable!("Expected program root");
    }

    assert!(
        parser.errors().is_empty(),
        "No recovery errors expected for valid anonymous sub assignment: {:?}",
        parser.errors()
    );
}

#[test]
fn test_local_as_assignment_rhs() {
    // `local` is a valid expression and must be allowed as an assignment RHS.
    // Regression test: removing Local from is_infix_rhs_absent must not break
    // recovery for genuine missing-RHS cases that happen to follow a `local` expr.
    let code = "local(*RS) = local(*/);";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should accept `local(x) = local(y);`");
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "`local(x) = local(y)` must parse as a single statement");
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("missing_expression"),
            "local-as-RHS must not produce MissingExpression: {sexp}"
        );
    } else {
        unreachable!("Expected program root");
    }

    assert!(
        parser.errors().is_empty(),
        "No recovery errors expected for `local(x) = local(y)`: {:?}",
        parser.errors()
    );
}
