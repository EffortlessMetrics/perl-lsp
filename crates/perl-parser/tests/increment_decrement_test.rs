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
                    return Err("Expected variable operand".into());
                }
            } else {
                return Err("Expected unary expression".into());
            }
        } else {
            return Err("Expected expression statement".into());
        }
    } else {
        return Err("Expected program node".into());
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
                    return Err("Expected variable operand".into());
                }
            } else {
                return Err("Expected unary expression".into());
            }
        } else {
            return Err("Expected expression statement".into());
        }
    } else {
        return Err("Expected program node".into());
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
                    return Err("Expected variable operand".into());
                }
            } else {
                return Err("Expected unary expression".into());
            }
        } else {
            return Err("Expected expression statement".into());
        }
    } else {
        return Err("Expected program node".into());
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
                    return Err("Expected variable operand".into());
                }
            } else {
                return Err("Expected unary expression".into());
            }
        } else {
            return Err("Expected expression statement".into());
        }
    } else {
        return Err("Expected program node".into());
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
                        return Err("Expected variable in left operand".into());
                    }
                } else {
                    return Err("Expected unary expression on left".into());
                }

                // Check right side (--$b)
                if let NodeKind::Unary { op, operand } = &right.kind {
                    assert_eq!(op, "--");
                    if let NodeKind::Variable { sigil, name } = &operand.kind {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "b");
                    } else {
                        return Err("Expected variable in right operand".into());
                    }
                } else {
                    return Err("Expected unary expression on right".into());
                }
            } else {
                return Err("Expected binary expression".into());
            }
        } else {
            return Err("Expected expression statement".into());
        }
    } else {
        return Err("Expected program node".into());
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
                        return Err("Expected variable in inner operand".into());
                    }
                } else {
                    return Err("Expected unary + expression".into());
                }
            } else {
                return Err("Expected unary expression".into());
            }
        } else {
            return Err("Expected expression statement".into());
        }
    } else {
        return Err("Expected program node".into());
    }
    Ok(())
}
