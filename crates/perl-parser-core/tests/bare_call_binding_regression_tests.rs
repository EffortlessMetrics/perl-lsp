mod cpan_test_helpers;

use cpan_test_helpers::{assert_clean_parse, parse};
use perl_parser_core::NodeKind;

fn program_statements(source: &str) -> Vec<perl_parser_core::Node> {
    let ast = parse(source);
    match ast.kind {
        NodeKind::Program { statements } => statements,
        other => {
            assert!(false, "expected Program node, got {:?}", other);
            Vec::new()
        }
    }
}

#[test]
fn bare_call_condition_stays_outside_ternary() {
    let source = "sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;";
    assert_clean_parse(source);

    let statements = program_statements(source);
    assert!(statements.len() >= 2, "expected at least 2 statements");

    let second = &statements[1];
    let initializer = match &second.kind {
        NodeKind::VariableDeclaration { initializer, .. } => match initializer.as_deref() {
            Some(node) => node,
            None => {
                assert!(false, "expected declaration initializer");
                return;
            }
        },
        other => {
            assert!(false, "expected VariableDeclaration, got {:?}", other);
            return;
        }
    };

    match &initializer.kind {
        NodeKind::Ternary { condition, .. } => match &condition.kind {
            NodeKind::FunctionCall { name, args } => {
                assert_eq!(name, "is_ready");
                assert_eq!(args.len(), 1, "expected one bare-call argument");
            }
            other => assert!(false, "expected FunctionCall condition, got {:?}", other),
        },
        other => assert!(false, "expected Ternary initializer, got {:?}", other),
    }
}

#[test]
fn bare_call_stops_before_word_or_rhs() {
    let source = "do_thing @args or die;";
    assert_clean_parse(source);

    let statements = program_statements(source);
    let expr = match &statements[0].kind {
        NodeKind::ExpressionStatement { expression } => expression.as_ref(),
        other => {
            assert!(false, "expected ExpressionStatement, got {:?}", other);
            return;
        }
    };

    match &expr.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "or");
            match &right.kind {
                NodeKind::FunctionCall { name, .. } => assert_eq!(name, "die"),
                other => assert!(false, "expected die FunctionCall rhs, got {:?}", other),
            }
            match &left.kind {
                NodeKind::FunctionCall { name, args } => {
                    assert_eq!(name, "do_thing");
                    assert_eq!(args.len(), 1);
                }
                other => assert!(false, "expected left FunctionCall, got {:?}", other),
            }
        }
        other => assert!(false, "expected Binary(or), got {:?}", other),
    }
}

#[test]
fn bare_call_stops_before_word_and_rhs() {
    let source = "do_thing $x and return;";
    assert_clean_parse(source);

    let statements = program_statements(source);
    let expr = match &statements[0].kind {
        NodeKind::ExpressionStatement { expression } => expression.as_ref(),
        other => {
            assert!(false, "expected ExpressionStatement, got {:?}", other);
            return;
        }
    };

    match &expr.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "and");
            assert!(matches!(right.kind, NodeKind::Return { .. }));
            assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
        }
        other => assert!(false, "expected Binary(and), got {:?}", other),
    }
}

#[test]
fn bare_call_stops_before_defined_or() {
    let source = "my $v = transform $x // $fallback;";
    assert_clean_parse(source);

    let statements = program_statements(source);
    let initializer = match &statements[0].kind {
        NodeKind::VariableDeclaration { initializer, .. } => match initializer.as_deref() {
            Some(node) => node,
            None => {
                assert!(false, "expected declaration initializer");
                return;
            }
        },
        other => {
            assert!(false, "expected VariableDeclaration, got {:?}", other);
            return;
        }
    };

    match &initializer.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "//");
            assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
        }
        other => assert!(false, "expected Binary(//), got {:?}", other),
    }
}

#[test]
fn nested_bare_call_ternary_inside_larger_expression() {
    let source = "my $z = 1 + (is_ready $obj ? 1 : 0);";
    assert_clean_parse(source);

    let statements = program_statements(source);
    let initializer = match &statements[0].kind {
        NodeKind::VariableDeclaration { initializer, .. } => match initializer.as_deref() {
            Some(node) => node,
            None => {
                assert!(false, "expected declaration initializer");
                return;
            }
        },
        other => {
            assert!(false, "expected VariableDeclaration, got {:?}", other);
            return;
        }
    };

    match &initializer.kind {
        NodeKind::Binary { op, right, .. } => {
            assert_eq!(op, "+");
            match &right.kind {
                NodeKind::Ternary { condition, .. } => {
                    assert!(matches!(condition.kind, NodeKind::FunctionCall { .. }));
                }
                other => assert!(false, "expected ternary rhs, got {:?}", other),
            }
        }
        other => assert!(false, "expected Binary(+), got {:?}", other),
    }
}

/// When a sort (or other builtin) is immediately followed by `?` it should parse
/// cleanly as `sort()` (no-arg call) and the ternary should apply to the result,
/// rather than blowing up with a MissingExpression / parse error.
#[test]
fn builtin_directly_before_ternary_no_args() {
    // sort with no args followed by ternary: should parse without errors
    let source = "my @x = sort ? @a : @b;";
    // We only require a clean parse (no error nodes). The exact AST shape is
    // implementation-defined but must not contain Error / MissingExpression nodes.
    assert_clean_parse(source);
}

/// Verify that the block-list function (grep) correctly absorbs the ternary
/// as an argument when @arr appears before the ternary, matching Perl semantics:
///   grep { ... } @arr ? 1 : 0
/// must parse as: grep(BLOCK, @arr ? 1 : 0)  — ternary binds tighter than list op
#[test]
fn block_list_func_absorbs_ternary_arg_after_array() {
    let source = "my $r = grep { $_ > 0 } @arr ? 1 : 0;";
    assert_clean_parse(source);
    let statements = program_statements(source);
    let initializer = match &statements[0].kind {
        NodeKind::VariableDeclaration { initializer, .. } => match initializer.as_deref() {
            Some(node) => node,
            None => {
                assert!(false, "expected declaration initializer");
                return;
            }
        },
        other => {
            assert!(false, "expected VariableDeclaration, got {:?}", other);
            return;
        }
    };
    // Correct Perl semantics: ternary binds tighter than list operators, so
    // grep collects the ternary expression as its list argument.
    match &initializer.kind {
        NodeKind::FunctionCall { name, args } => {
            assert_eq!(name, "grep");
            assert_eq!(args.len(), 2, "expected block + one list arg");
            // second arg should be the ternary @arr ? 1 : 0
            assert!(
                matches!(args[1].kind, NodeKind::Ternary { .. }),
                "expected Ternary second arg, got {:?}",
                args[1].kind.kind_name()
            );
        }
        other => assert!(false, "expected FunctionCall(grep), got {:?}", other),
    }
}

/// When `?` follows immediately after a block (no list argument in between),
/// `should_continue_bare_call_after_block` must NOT treat `?` as a continuation.
///   my $r = do { 1 } ? "yes" : "no";
/// must parse cleanly — the `?` starts the ternary over the block's value.
#[test]
fn bare_block_directly_before_ternary_no_error() {
    let source = "my $r = do { 1 } ? \"yes\" : \"no\";";
    assert_clean_parse(source);
}

/// Nested ternary in the bare-call condition — no parens.
///   foo $x ? $a ? $b : $c : $d
/// must parse as: (foo $x) ? ($a ? $b : $c) : $d
/// i.e. the outer ternary's condition is FunctionCall(foo, [$x]).
#[test]
fn bare_call_nested_ternary_outside_call() {
    let source = "my $r = is_ready $obj ? 1 ? 2 : 3 : 4;";
    assert_clean_parse(source);
    let statements = program_statements(source);
    let initializer = match &statements[0].kind {
        NodeKind::VariableDeclaration { initializer, .. } => match initializer.as_deref() {
            Some(node) => node,
            None => {
                assert!(false, "expected declaration initializer");
                return;
            }
        },
        other => {
            assert!(false, "expected VariableDeclaration, got {:?}", other);
            return;
        }
    };
    match &initializer.kind {
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            // Outer condition must be the bare call
            assert!(
                matches!(condition.kind, NodeKind::FunctionCall { .. }),
                "outer ternary condition must be FunctionCall, got {:?}",
                condition.kind.kind_name()
            );
            // then-branch is another ternary (1 ? 2 : 3)
            assert!(
                matches!(then_expr.kind, NodeKind::Ternary { .. }),
                "then-branch must be nested Ternary, got {:?}",
                then_expr.kind.kind_name()
            );
            // else-branch is a literal 4
            assert!(
                matches!(else_expr.kind, NodeKind::Number { .. }),
                "else-branch must be Number, got {:?}",
                else_expr.kind.kind_name()
            );
        }
        other => assert!(false, "expected outer Ternary, got {:?}", other),
    }
}
