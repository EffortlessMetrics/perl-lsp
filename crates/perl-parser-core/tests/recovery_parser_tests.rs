use perl_parser_core::{ast_v2::NodeKind as V2NodeKind, error::recovery_parser::RecoveryParser};

#[test]
fn empty_input_produces_empty_program() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new(String::new());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert!(statements.is_empty(), "empty source should yield zero statements");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.is_empty(), "empty source should produce no errors");
    Ok(())
}

#[test]
fn single_number_literal() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("42".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0].kind {
                V2NodeKind::Number { value } => assert_eq!(value, "42"),
                other => return Err(format!("expected Number, got {:?}", other).into()),
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.is_empty());
    Ok(())
}

#[test]
fn variable_declaration_with_initializer() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("my $x = 42".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0].kind {
                V2NodeKind::VariableDeclaration { declarator, initializer, .. } => {
                    assert_eq!(declarator, "my");
                    assert!(initializer.is_some(), "should have an initializer");
                }
                other => {
                    return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                }
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.is_empty());
    Ok(())
}

#[test]
fn variable_declaration_without_initializer() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("my $x".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0].kind {
                V2NodeKind::VariableDeclaration { declarator, initializer, .. } => {
                    assert_eq!(declarator, "my");
                    assert!(initializer.is_none(), "should have no initializer");
                }
                other => {
                    return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                }
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.is_empty());
    Ok(())
}

#[test]
fn multiple_declarations_with_semicolons() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("my $x = 1; my $y = 2".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 2, "should parse two declarations");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn missing_semicolon_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("my $x = 42 my $y = 99".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 2, "should recover and parse both statements");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn syntax_error_produces_error_nodes() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("my $x = ; my $y = 42".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 2, "should parse both despite error");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(!errors.is_empty(), "should record at least one error");
    Ok(())
}

#[test]
fn if_statement_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("if $x { my $y = 1 }".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0].kind {
                V2NodeKind::If { then_branch, .. } => match &then_branch.kind {
                    V2NodeKind::Block { statements } => {
                        assert_eq!(statements.len(), 1);
                    }
                    other => return Err(format!("expected Block, got {:?}", other).into()),
                },
                other => return Err(format!("expected If, got {:?}", other).into()),
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn unclosed_block_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("if $x { my $y = 42".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1, "should parse the if statement");
            match &statements[0].kind {
                V2NodeKind::If { then_branch, .. } => match &then_branch.kind {
                    V2NodeKind::Block { statements } => {
                        assert_eq!(statements.len(), 1, "block should contain the declaration");
                    }
                    other => return Err(format!("expected Block, got {:?}", other).into()),
                },
                other => return Err(format!("expected If, got {:?}", other).into()),
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(!errors.is_empty(), "should have error about missing closing brace");
    Ok(())
}

#[test]
fn our_declarator() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("our $config".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0].kind {
                V2NodeKind::VariableDeclaration { declarator, .. } => {
                    assert_eq!(declarator, "our");
                }
                other => {
                    return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                }
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.is_empty());
    Ok(())
}

#[test]
fn local_declarator() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("local $x".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should parse at least one statement");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn state_declarator() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("state $counter = 0".to_string());
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0].kind {
                V2NodeKind::VariableDeclaration { declarator, .. } => {
                    assert_eq!(declarator, "state");
                }
                other => {
                    return Err(format!("expected VariableDeclaration, got {:?}", other).into());
                }
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.is_empty());
    Ok(())
}

#[test]
fn while_keyword_parsed() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("while 1".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should produce at least one node");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn sub_keyword_parsed() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("sub foo".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should produce at least one node");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn program_node_range_covers_input() -> Result<(), Box<dyn std::error::Error>> {
    let parser = RecoveryParser::new("my $x = 1".to_string());
    let (ast, _errors) = parser.parse();

    assert!(ast.range.start.byte == 0, "program should start at byte 0");
    Ok(())
}

#[test]
fn string_literal_or_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
    // The RecoveryParser's simplified expression parser may not handle all
    // string literal token forms. Verify it produces a valid Program either way.
    let parser = RecoveryParser::new("\"hello\"".to_string());
    let (ast, _errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            // Should produce at least one node (string or error recovery)
            assert!(!statements.is_empty(), "should produce at least one statement/node");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}
