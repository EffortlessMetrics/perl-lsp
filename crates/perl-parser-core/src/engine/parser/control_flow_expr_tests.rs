#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;
    use perl_ast::ast::NodeKind;

    fn parse_code(input: &str) -> Option<perl_ast::ast::Node> {
        let mut parser = Parser::new(input);
        parser.parse().ok()
    }

    /// Check that the parsed AST does not contain any ERROR nodes, meaning the
    /// code parsed cleanly without recovery.
    fn assert_no_errors(input: &str, ast: &perl_ast::ast::Node) {
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "Parse of {:?} produced ERROR nodes:\n{}", input, sexp);
    }

    // ---------------------------------------------------------------
    // return / next / last in ternary expression branches
    // ---------------------------------------------------------------

    #[test]
    fn test_return_in_ternary_then_branch() {
        // $cond ? return $x : $y
        let input = "$cond ? return $x : $y;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        let ast = ast.as_ref();
        assert_no_errors(input, ast.unwrap_or_else(|| unreachable!()));

        // Verify structure: the then branch should be a Return node
        let ast = parse_code(input);
        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                // Unwrap ExpressionStatement
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                // Should be a Ternary
                if let NodeKind::Ternary { then_expr, .. } = &expr.kind {
                    assert!(
                        matches!(then_expr.kind, NodeKind::Return { .. }),
                        "Expected Return in then branch, got {:?}",
                        then_expr.kind
                    );
                } else {
                    unreachable!("Expected Ternary, got {:?}", expr.kind);
                }
            }
        }
    }

    #[test]
    fn test_next_in_ternary_else_branch() {
        // $cond ? $x : next
        let input = "$cond ? $x : next;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Ternary { else_expr, .. } = &expr.kind {
                    assert!(
                        matches!(else_expr.kind, NodeKind::LoopControl { .. }),
                        "Expected LoopControl in else branch, got {:?}",
                        else_expr.kind
                    );
                } else {
                    unreachable!("Expected Ternary, got {:?}", expr.kind);
                }
            }
        }
    }

    #[test]
    fn test_return_and_next_in_ternary() {
        // $cond ? return $x : next
        let input = "$cond ? return $x : next;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Ternary { then_expr, else_expr, .. } = &expr.kind {
                    assert!(
                        matches!(then_expr.kind, NodeKind::Return { .. }),
                        "Expected Return in then branch, got {:?}",
                        then_expr.kind
                    );
                    assert!(
                        matches!(else_expr.kind, NodeKind::LoopControl { .. }),
                        "Expected LoopControl in else branch, got {:?}",
                        else_expr.kind
                    );
                } else {
                    unreachable!("Expected Ternary, got {:?}", expr.kind);
                }
            }
        }
    }

    #[test]
    fn test_last_in_ternary_else_branch() {
        // $cond ? $x : last
        let input = "$cond ? $x : last;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Ternary { else_expr, .. } = &expr.kind {
                    assert!(
                        matches!(else_expr.kind, NodeKind::LoopControl { .. }),
                        "Expected LoopControl in else branch, got {:?}",
                        else_expr.kind
                    );
                } else {
                    unreachable!("Expected Ternary, got {:?}", expr.kind);
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // return / next / last with short-circuit operators (&& / ||)
    // ---------------------------------------------------------------

    #[test]
    fn test_short_circuit_and_return() {
        // $cond && return
        let input = "$cond && return;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Binary { op, right, .. } = &expr.kind {
                    assert_eq!(op, "&&");
                    assert!(
                        matches!(right.kind, NodeKind::Return { value: None }),
                        "Expected Return with no value on RHS, got {:?}",
                        right.kind
                    );
                } else {
                    unreachable!("Expected Binary(&&), got {:?}", expr.kind);
                }
            }
        }
    }

    #[test]
    fn test_short_circuit_or_last() {
        // $cond || last
        let input = "$cond || last;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Binary { op, right, .. } = &expr.kind {
                    assert_eq!(op, "||");
                    assert!(
                        matches!(right.kind, NodeKind::LoopControl { .. }),
                        "Expected LoopControl on RHS, got {:?}",
                        right.kind
                    );
                } else {
                    unreachable!("Expected Binary(||), got {:?}", expr.kind);
                }
            }
        }
    }

    #[test]
    fn test_short_circuit_and_next() {
        // $cond && next
        let input = "$cond && next;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Binary { op, right, .. } = &expr.kind {
                    assert_eq!(op, "&&");
                    assert!(
                        matches!(right.kind, NodeKind::LoopControl { .. }),
                        "Expected LoopControl on RHS, got {:?}",
                        right.kind
                    );
                } else {
                    unreachable!("Expected Binary(&&), got {:?}", expr.kind);
                }
            }
        }
    }

    #[test]
    fn test_short_circuit_defined_or_return() {
        // $cond // return $x
        let input = "$cond // return $x;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Binary { op, right, .. } = &expr.kind {
                    assert_eq!(op, "//");
                    assert!(
                        matches!(right.kind, NodeKind::Return { .. }),
                        "Expected Return on RHS, got {:?}",
                        right.kind
                    );
                } else {
                    unreachable!("Expected Binary(//), got {:?}", expr.kind);
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // return in do block
    // ---------------------------------------------------------------

    #[test]
    fn test_return_inside_do_block() {
        // do { return $x if $cond; $y }
        let input = "do { return $x if $cond; $y };";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));
    }

    // ---------------------------------------------------------------
    // return in ternary inside do block
    // ---------------------------------------------------------------

    #[test]
    fn test_return_in_ternary_in_do_block() {
        // my $x = do { $cond ? return : $val }
        let input = "my $x = do { $cond ? return : $val };";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));
    }

    // ---------------------------------------------------------------
    // return with no value in ternary (bare return before colon)
    // ---------------------------------------------------------------

    #[test]
    fn test_bare_return_in_ternary_then() {
        // $cond ? return : $y
        let input = "$cond ? return : $y;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Ternary { then_expr, .. } = &expr.kind {
                    assert!(
                        matches!(then_expr.kind, NodeKind::Return { value: None }),
                        "Expected bare Return (no value) in then branch, got {:?}",
                        then_expr.kind
                    );
                } else {
                    unreachable!("Expected Ternary, got {:?}", expr.kind);
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // loop control with label in expression context
    // ---------------------------------------------------------------

    #[test]
    fn test_next_with_label_in_ternary() {
        // $cond ? next OUTER : $y
        let input = "$cond ? next OUTER : $y;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Ternary { then_expr, .. } = &expr.kind {
                    if let NodeKind::LoopControl { op, label } = &then_expr.kind {
                        assert_eq!(op, "next");
                        assert_eq!(label.as_deref(), Some("OUTER"));
                    } else {
                        unreachable!(
                            "Expected LoopControl in then branch, got {:?}",
                            then_expr.kind
                        );
                    }
                } else {
                    unreachable!("Expected Ternary, got {:?}", expr.kind);
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // redo in expression context
    // ---------------------------------------------------------------

    #[test]
    fn test_redo_in_short_circuit() {
        // $cond || redo
        let input = "$cond || redo;";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));

        if let Some(ref node) = ast {
            if let NodeKind::Program { ref statements } = node.kind {
                let stmt = &statements[0];
                let expr = match &stmt.kind {
                    NodeKind::ExpressionStatement { expression } => expression.as_ref(),
                    other => unreachable!("Expected ExpressionStatement, got {:?}", other),
                };
                if let NodeKind::Binary { op, right, .. } = &expr.kind {
                    assert_eq!(op, "||");
                    assert!(
                        matches!(right.kind, NodeKind::LoopControl { .. }),
                        "Expected LoopControl on RHS, got {:?}",
                        right.kind
                    );
                } else {
                    unreachable!("Expected Binary(||), got {:?}", expr.kind);
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // Keyword before fat arrow should still autoquote (regression guard)
    // ---------------------------------------------------------------

    #[test]
    fn test_return_before_fat_arrow_still_autoquotes() {
        // (return => 1) should autoquote "return" as a hash key
        let input = "my %h = (return => 1);";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));
    }

    #[test]
    fn test_next_before_fat_arrow_still_autoquotes() {
        // (next => 1) should autoquote "next" as a hash key
        let input = "my %h = (next => 1);";
        let ast = parse_code(input);
        assert!(ast.is_some(), "Failed to parse: {}", input);
        assert_no_errors(input, ast.as_ref().unwrap_or_else(|| unreachable!()));
    }
}
