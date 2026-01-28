use super::*;
use perl_tdd_support::must;

#[test]
fn test_recovery_missing_expression() {
    // missing expression after assignment
    let code = "my $x = ; print 1;";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // With recovery, parse() should return Ok(Program)
    // even if there were errors.
    // Wait, parse_program currently returns Ok(Node) but populates self.errors

    match result {
        Ok(ast) => {
            println!("AST: {}", ast.to_sexp());

            // Check that we have 2 statements
            if let NodeKind::Program { statements } = &ast.kind {
                assert_eq!(statements.len(), 2, "Should have 2 statements (1 error, 1 valid)");

                // First statement should be Error
                match &statements[0].kind {
                    NodeKind::Error { message, .. } => {
                        println!("Found expected error: {}", message);
                    }
                    _ => panic!(
                        "Expected Error node for first statement, got: {:?}",
                        statements[0].kind
                    ),
                }

                // Second statement should be ExpressionStatement
                match &statements[1].kind {
                    NodeKind::ExpressionStatement { .. } => {
                        println!("Found valid second statement");
                    }
                    _ => panic!(
                        "Expected ExpressionStatement for second statement, got: {:?}",
                        statements[1].kind
                    ),
                }
            } else {
                panic!("Expected Program node");
            }

            // Check errors list
            let errors = parser.errors();
            assert!(!errors.is_empty(), "Should have recorded errors");
            println!("Errors: {:?}", errors);
        }
        Err(e) => {
            panic!("Parser failed to recover: {}", e);
        }
    }
}

#[test]
fn test_recovery_multiple_errors() {
    let code = "
        my $a = ;   # Error 1
        print 1;    # Valid
        my $b = ;   # Error 2
        print 2;    # Valid
    ";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = must(result);

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 4, "Should have 4 statements");
        assert!(matches!(statements[0].kind, NodeKind::Error { .. }));
        assert!(matches!(statements[1].kind, NodeKind::ExpressionStatement { .. }));
        assert!(matches!(statements[2].kind, NodeKind::Error { .. }));
        assert!(matches!(statements[3].kind, NodeKind::ExpressionStatement { .. }));
    }

    // We expect 4 errors: 2 original parsing errors + 2 context errors from recovery
    assert_eq!(parser.errors().len(), 4);
}

#[test]
fn test_recovery_inside_block() {
    // Error inside a block
    let code = "sub foo { my $x = ; print 1; }";
    let mut parser = Parser::new(code);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            // Structure: Program -> Subroutine -> Block -> [Error, ExpressionStatement]
            if let NodeKind::Program { statements } = &ast.kind {
                // Should be 1 statement (the subroutine declaration)
                assert_eq!(statements.len(), 1);

                if let NodeKind::Subroutine { body, .. } = &statements[0].kind {
                    if let NodeKind::Block { statements } = &body.kind {
                        assert_eq!(
                            statements.len(),
                            2,
                            "Block should have 2 statements (1 error, 1 valid)"
                        );

                        match &statements[0].kind {
                            NodeKind::Error { message, .. } => {
                                println!("Found block error: {}", message)
                            }
                            _ => panic!("Expected Error node in block"),
                        }

                        match &statements[1].kind {
                            NodeKind::ExpressionStatement { .. } => {
                                println!("Found valid statement in block")
                            }
                            _ => panic!("Expected ExpressionStatement in block"),
                        }
                    } else {
                        panic!("Expected Block in subroutine body");
                    }
                } else {
                    panic!("Expected Subroutine node, got: {:?}", statements[0].kind);
                }
            }

            assert!(!parser.errors().is_empty());
        }
        Err(e) => panic!("Failed to recover from block error: {}", e),
    }
}
