//! Tests for chained method calls after dereference constructs.
//!
//! Perl allows chaining method calls after hash/array dereferences:
//!   $obj->{key}->method()
//!   $obj->[0]->method()
//!   $self->{db}->resultset('Foo')->search({})
//!   $hash{key}->method->another
//!   $ref->method->{key}->[0]->final_method
//!
//! The parser must correctly continue the postfix chain after subscript
//! operations following an arrow.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use crate::parser::Parser;
    use perl_ast::ast::{Node, NodeKind};

    /// Helper: parse code and return the full AST.
    fn parse_program(code: &str) -> Node {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => ast,
            Err(e) => panic!("Parse failed for `{code}`: {e:?}"),
        }
    }

    /// Helper: parse code and return the first statement node.
    fn parse_first_stmt(code: &str) -> Node {
        let ast = parse_program(code);
        match ast.kind {
            NodeKind::Program { mut statements } if !statements.is_empty() => {
                statements.swap_remove(0)
            }
            _ => panic!("Expected Program with statements, got: {}", ast.to_sexp()),
        }
    }

    /// Helper: check that the AST sexp contains no ERROR nodes.
    fn assert_no_errors(code: &str) {
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes:\n{}", code, sexp);
    }

    /// Helper: extract expression from an ExpressionStatement.
    fn unwrap_expr_stmt(stmt: Node) -> Node {
        match stmt.kind {
            NodeKind::ExpressionStatement { expression } => *expression,
            _ => panic!(
                "Expected ExpressionStatement, got {} (sexp: {})",
                stmt.kind.kind_name(),
                stmt.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Method call after arrow-hash deref: $obj->{key}->method()
    // ---------------------------------------------------------------

    #[test]
    fn method_after_arrow_hash_deref() {
        let code = "$obj->{key}->method();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        match &expr.kind {
            NodeKind::MethodCall { object, method, .. } => {
                assert_eq!(method, "method", "Expected method name 'method', got '{}'", method);
                // The object should be a Binary{} (hash access) node wrapping $obj
                assert_eq!(
                    object.kind.kind_name(),
                    "Binary",
                    "Object of method call should be Binary (hash deref), got {}",
                    object.kind.kind_name(),
                );
            }
            _ => panic!(
                "Expected MethodCall for $obj->{{key}}->method(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Method call after arrow-array deref: $obj->[0]->method()
    // ---------------------------------------------------------------

    #[test]
    fn method_after_arrow_array_deref() {
        let code = "$obj->[0]->method();";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        match &expr.kind {
            NodeKind::MethodCall { object, method, .. } => {
                assert_eq!(method, "method", "Expected method name 'method', got '{}'", method);
                // The object should be a Binary[] (array access) node wrapping $obj
                assert_eq!(
                    object.kind.kind_name(),
                    "Binary",
                    "Object of method call should be Binary (array deref), got {}",
                    object.kind.kind_name(),
                );
            }
            _ => panic!(
                "Expected MethodCall for $obj->[0]->method(), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Deep chain: $self->{db}->resultset('Foo')->search({})
    // ---------------------------------------------------------------

    #[test]
    fn deep_chain_hash_deref_then_methods() {
        let code = "$self->{db}->resultset('Foo')->search({});";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost should be ->search({})
        match &expr.kind {
            NodeKind::MethodCall { object, method, args } => {
                assert_eq!(method, "search");
                assert!(!args.is_empty(), "search() should have at least one argument");
                // The object of search() should be ->resultset('Foo')
                match &object.kind {
                    NodeKind::MethodCall { method: inner_method, .. } => {
                        assert_eq!(inner_method, "resultset");
                    }
                    _ => panic!(
                        "Expected inner MethodCall (resultset), got {} (sexp: {})",
                        object.kind.kind_name(),
                        object.to_sexp()
                    ),
                }
            }
            _ => panic!(
                "Expected MethodCall for deep chain, got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Chain after bare hash access: $hash{key}->method->another
    // ---------------------------------------------------------------

    #[test]
    fn chain_after_bare_hash_access() {
        let code = "$hash{key}->method->another;";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost: ->another
        match &expr.kind {
            NodeKind::MethodCall { object, method, .. } => {
                assert_eq!(method, "another");
                // Inner: ->method
                match &object.kind {
                    NodeKind::MethodCall { object: inner_obj, method: inner_method, .. } => {
                        assert_eq!(inner_method, "method");
                        // Innermost: $hash{key} (Binary hash access)
                        assert_eq!(
                            inner_obj.kind.kind_name(),
                            "Binary",
                            "Innermost should be Binary (hash access), got {}",
                            inner_obj.kind.kind_name(),
                        );
                    }
                    _ => panic!(
                        "Expected inner MethodCall, got {} (sexp: {})",
                        object.kind.kind_name(),
                        object.to_sexp()
                    ),
                }
            }
            _ => panic!(
                "Expected MethodCall for chain after bare hash, got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Mixed chain: $ref->method->{key}->[0]->final_method
    // ---------------------------------------------------------------

    #[test]
    fn mixed_chain_method_hash_array_method() {
        let code = "$ref->method->{key}->[0]->final_method;";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost should be ->final_method
        match &expr.kind {
            NodeKind::MethodCall { method, .. } => {
                assert_eq!(method, "final_method");
            }
            _ => panic!(
                "Expected MethodCall (final_method), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Arrow-hash deref without method (baseline): $obj->{key}
    // ---------------------------------------------------------------

    #[test]
    fn arrow_hash_deref_alone() {
        let code = "$obj->{key};";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        assert_eq!(
            expr.kind.kind_name(),
            "Binary",
            "Expected Binary (hash access), got {} (sexp: {})",
            expr.kind.kind_name(),
            expr.to_sexp(),
        );
    }

    // ---------------------------------------------------------------
    // Arrow-array deref without method (baseline): $obj->[0]
    // ---------------------------------------------------------------

    #[test]
    fn arrow_array_deref_alone() {
        let code = "$obj->[0];";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        assert_eq!(
            expr.kind.kind_name(),
            "Binary",
            "Expected Binary (array access), got {} (sexp: {})",
            expr.kind.kind_name(),
            expr.to_sexp(),
        );
    }

    // ---------------------------------------------------------------
    // Hash deref then array deref: $obj->{key}->[0]
    // ---------------------------------------------------------------

    #[test]
    fn hash_deref_then_array_deref() {
        let code = "$obj->{key}->[0];";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost should be ->[] (arrow array deref)
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->[]", "Outer should be arrow array deref, got op={}", op);
                // Inner should be ->{} (arrow hash deref)
                match &left.kind {
                    NodeKind::Binary { op: inner_op, .. } => {
                        assert_eq!(
                            inner_op, "->{}",
                            "Inner should be arrow hash deref, got op={}",
                            inner_op
                        );
                    }
                    _ => panic!(
                        "Expected inner Binary (hash), got {} (sexp: {})",
                        left.kind.kind_name(),
                        left.to_sexp()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary (array subscript), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Array deref then hash deref: $obj->[0]->{key}
    // ---------------------------------------------------------------

    #[test]
    fn array_deref_then_hash_deref() {
        let code = "$obj->[0]->{key};";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost should be ->{} (arrow hash deref)
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->{}", "Outer should be arrow hash deref, got op={}", op);
                // Inner should be ->[] (arrow array deref)
                match &left.kind {
                    NodeKind::Binary { op: inner_op, .. } => {
                        assert_eq!(
                            inner_op, "->[]",
                            "Inner should be arrow array deref, got op={}",
                            inner_op
                        );
                    }
                    _ => panic!(
                        "Expected inner Binary (array), got {} (sexp: {})",
                        left.kind.kind_name(),
                        left.to_sexp()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary (hash subscript), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Method returning hashref chained: $obj->get_config->{timeout}
    // ---------------------------------------------------------------

    #[test]
    fn method_then_hash_deref() {
        let code = "$obj->get_config->{timeout};";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost should be ->{} (arrow hash deref on method result)
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->{}", "Outer should be arrow hash deref, got op={}", op);
                // Inner should be the MethodCall
                match &left.kind {
                    NodeKind::MethodCall { method, .. } => {
                        assert_eq!(method, "get_config");
                    }
                    _ => panic!(
                        "Expected inner MethodCall, got {} (sexp: {})",
                        left.kind.kind_name(),
                        left.to_sexp()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary (hash subscript), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Method returning arrayref chained: $obj->get_items->[0]
    // ---------------------------------------------------------------

    #[test]
    fn method_then_array_deref() {
        let code = "$obj->get_items->[0];";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));
        // Outermost should be ->[] (arrow array deref on method result)
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->[]", "Outer should be arrow array deref, got op={}", op);
                // Inner should be the MethodCall
                match &left.kind {
                    NodeKind::MethodCall { method, .. } => {
                        assert_eq!(method, "get_items");
                    }
                    _ => panic!(
                        "Expected inner MethodCall, got {} (sexp: {})",
                        left.kind.kind_name(),
                        left.to_sexp()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary (array subscript), got {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Arrow dereference should produce distinct op from bare access:
    //   $obj->{key} should produce op "->{}" (arrow hash deref)
    //   $obj{key}   should produce op "{}"  (bare hash access)
    //   $obj->[0]   should produce op "->[]" (arrow array deref)
    //   $obj[0]     should produce op "[]"  (bare array access)
    // ---------------------------------------------------------------

    #[test]
    fn arrow_hash_deref_distinct_from_bare_hash() {
        // Arrow dereference: $obj->{key}
        let code_arrow = "$obj->{key};";
        let expr_arrow = unwrap_expr_stmt(parse_first_stmt(code_arrow));
        let arrow_op = match &expr_arrow.kind {
            NodeKind::Binary { op, .. } => op.clone(),
            _ => panic!("Expected Binary for arrow deref, got {}", expr_arrow.to_sexp()),
        };

        // Bare hash access: $obj{key}
        let code_bare = "$obj{key};";
        let expr_bare = unwrap_expr_stmt(parse_first_stmt(code_bare));
        let bare_op = match &expr_bare.kind {
            NodeKind::Binary { op, .. } => op.clone(),
            _ => panic!("Expected Binary for bare hash, got {}", expr_bare.to_sexp()),
        };

        assert_eq!(
            arrow_op, "->{}",
            "Arrow hash deref should use op '->{{}}'  but got '{}'",
            arrow_op
        );
        assert_eq!(bare_op, "{}", "Bare hash access should use op '{{}}' but got '{}'", bare_op);
    }

    #[test]
    fn arrow_array_deref_distinct_from_bare_array() {
        // Arrow dereference: $obj->[0]
        let code_arrow = "$obj->[0];";
        let expr_arrow = unwrap_expr_stmt(parse_first_stmt(code_arrow));
        let arrow_op = match &expr_arrow.kind {
            NodeKind::Binary { op, .. } => op.clone(),
            _ => panic!("Expected Binary for arrow deref, got {}", expr_arrow.to_sexp()),
        };

        // Bare array access: $obj[0]
        let code_bare = "$obj[0];";
        let expr_bare = unwrap_expr_stmt(parse_first_stmt(code_bare));
        let bare_op = match &expr_bare.kind {
            NodeKind::Binary { op, .. } => op.clone(),
            _ => panic!("Expected Binary for bare array, got {}", expr_bare.to_sexp()),
        };

        assert_eq!(
            arrow_op, "->[]",
            "Arrow array deref should use op '->[]' but got '{}'",
            arrow_op
        );
        assert_eq!(bare_op, "[]", "Bare array access should use op '[]' but got '{}'", bare_op);
    }

    // ---------------------------------------------------------------
    // Arrow dereference in chains should use the distinct operators
    // ---------------------------------------------------------------

    #[test]
    fn mixed_chain_uses_arrow_deref_ops() {
        // $ref->method->{key}->[0]->final_method
        // Should produce: MethodCall(final_method,
        //   ->[](->{}(MethodCall(method, $ref), key), 0))
        let code = "$ref->method->{key}->[0]->final_method;";
        assert_no_errors(code);
        let expr = unwrap_expr_stmt(parse_first_stmt(code));

        // Walk the chain from outside in:
        // 1. Outermost: ->final_method (MethodCall)
        let method_call = match &expr.kind {
            NodeKind::MethodCall { object, method, .. } => {
                assert_eq!(method, "final_method");
                object
            }
            _ => panic!("Expected MethodCall, got {}", expr.to_sexp()),
        };

        // 2. Next: ->[0] (Binary with op "->[]")
        let array_deref = match &method_call.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->[]", "Expected arrow array deref '->[]', got '{}'", op);
                left
            }
            _ => panic!("Expected Binary (->[] array deref), got {}", method_call.to_sexp()),
        };

        // 3. Next: ->{key} (Binary with op "->{}")
        match &array_deref.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->{}", "Expected arrow hash deref '->{{}}', got '{}'", op);
                // 4. Innermost: ->method (MethodCall)
                match &left.kind {
                    NodeKind::MethodCall { method, .. } => {
                        assert_eq!(method, "method");
                    }
                    _ => panic!(
                        "Expected inner MethodCall, got {} (sexp: {})",
                        left.kind.kind_name(),
                        left.to_sexp()
                    ),
                }
            }
            _ => panic!("Expected Binary (arrow hash deref), got {}", array_deref.to_sexp()),
        }
    }
}
