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
