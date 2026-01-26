use perl_parser::{NodeKind, Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_pre_increment() -> TestResult {
    let mut parser = Parser::new("++$x");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Unary { op, operand } = &expression.kind {
                assert_eq!(op, "++");
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    assert_eq!(sigil, "$");
                    assert_eq!(name, "x");
                } else {
                    panic!("Expected variable operand");
                }
            } else {
                panic!("Expected unary expression");
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("Expected program node");
    }
    Ok(())
}

#[test]
fn test_pre_decrement() -> TestResult {
    let mut parser = Parser::new("--$y");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Unary { op, operand } = &expression.kind {
                assert_eq!(op, "--");
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    assert_eq!(sigil, "$");
                    assert_eq!(name, "y");
                } else {
                    panic!("Expected variable operand");
                }
            } else {
                panic!("Expected unary expression");
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("Expected program node");
    }
    Ok(())
}

#[test]
fn test_post_increment() -> TestResult {
    let mut parser = Parser::new("$x++");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Unary { op, operand } = &expression.kind {
                assert_eq!(op, "++");
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    assert_eq!(sigil, "$");
                    assert_eq!(name, "x");
                } else {
                    panic!("Expected variable operand");
                }
            } else {
                panic!("Expected unary expression");
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("Expected program node");
    }
    Ok(())
}

#[test]
fn test_post_decrement() -> TestResult {
    let mut parser = Parser::new("$y--");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Unary { op, operand } = &expression.kind {
                assert_eq!(op, "--");
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    assert_eq!(sigil, "$");
                    assert_eq!(name, "y");
                } else {
                    panic!("Expected variable operand");
                }
            } else {
                panic!("Expected unary expression");
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("Expected program node");
    }
    Ok(())
}

#[test]
fn test_complex_increment_decrement() -> TestResult {
    let mut parser = Parser::new("++$a + --$b");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Binary { op, left, right } = &expression.kind {
                assert_eq!(op, "+");

                // Check left side (++$a)
                if let NodeKind::Unary { op, operand } = &left.kind {
                    assert_eq!(op, "++");
                    if let NodeKind::Variable { sigil, name } = &operand.kind {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "a");
                    } else {
                        panic!("Expected variable in left operand");
                    }
                } else {
                    panic!("Expected unary expression on left");
                }

                // Check right side (--$b)
                if let NodeKind::Unary { op, operand } = &right.kind {
                    assert_eq!(op, "--");
                    if let NodeKind::Variable { sigil, name } = &operand.kind {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "b");
                    } else {
                        panic!("Expected variable in right operand");
                    }
                } else {
                    panic!("Expected unary expression on right");
                }
            } else {
                panic!("Expected binary expression");
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("Expected program node");
    }
    Ok(())
}

#[test]
fn test_chained_increment() -> TestResult {
    // Test that +++$x is parsed as ++(+$x) not as ++ +$x
    let mut parser = Parser::new("+++$x");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Unary { op, operand } = &expression.kind {
                assert_eq!(op, "++");
                // The operand should be +$x
                if let NodeKind::Unary { op: inner_op, operand: inner_operand } = &operand.kind {
                    assert_eq!(inner_op, "+");
                    if let NodeKind::Variable { sigil, name } = &inner_operand.kind {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "x");
                    } else {
                        panic!("Expected variable in inner operand");
                    }
                } else {
                    panic!("Expected unary + expression");
                }
            } else {
                panic!("Expected unary expression");
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("Expected program node");
    }
    Ok(())
}
