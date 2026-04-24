mod cpan_test_helpers;

use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

fn expression_statement<'a>(ast: &'a Node, index: usize) -> &'a Node {
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program, got {}", ast.kind.kind_name());
    };

    let statement = statements.get(index).unwrap_or_else(|| {
        panic!("expected statement at index {index}, got {} statements", statements.len())
    });

    let NodeKind::ExpressionStatement { expression } = &statement.kind else {
        panic!("expected ExpressionStatement at index {index}, got {}", statement.kind.kind_name());
    };

    expression
}

#[test]
fn bare_call_before_ternary_keeps_ternary_outside_call() {
    let ast = parse("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program, got {}", ast.kind.kind_name());
    };

    let declaration = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::VariableDeclaration { .. }))
        .expect("expected variable declaration after sub declaration");

    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &declaration.kind
    else {
        panic!("expected declaration initializer, got {}", declaration.kind.kind_name());
    };

    let NodeKind::Ternary { condition, .. } = &initializer.kind else {
        panic!(
            "expected ternary initializer for bare-call condition, got {}\n{}",
            initializer.kind.kind_name(),
            initializer.to_sexp()
        );
    };

    let NodeKind::FunctionCall { name, args } = &condition.kind else {
        panic!(
            "expected ternary condition to be function call, got {}\n{}",
            condition.kind.kind_name(),
            condition.to_sexp()
        );
    };

    assert_eq!(name, "is_ready");
    assert_eq!(args.len(), 1, "expected one bare argument: {}", condition.to_sexp());
}

#[test]
fn bare_call_with_or_keeps_word_operator_outside_call() {
    let ast = parse("do_thing @args or die;");
    let expression = expression_statement(&ast, 0);

    let NodeKind::Binary { op, left, right } = &expression.kind else {
        panic!(
            "expected top-level binary `or`, got {}\n{}",
            expression.kind.kind_name(),
            expression.to_sexp()
        );
    };

    assert_eq!(op, "or");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
    assert!(matches!(right.kind, NodeKind::FunctionCall { .. }));
}

#[test]
fn bare_call_with_and_keeps_word_operator_outside_call() {
    let ast = parse("do_thing $x and return;");
    let expression = expression_statement(&ast, 0);

    let NodeKind::Binary { op, left, right } = &expression.kind else {
        panic!(
            "expected top-level binary `and`, got {}\n{}",
            expression.kind.kind_name(),
            expression.to_sexp()
        );
    };

    assert_eq!(op, "and");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
    assert!(matches!(right.kind, NodeKind::Return { .. }));
}

#[test]
fn bare_call_with_defined_or_keeps_operator_outside_call() {
    let ast = parse("my $v = transform $x // $fallback;");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program, got {}", ast.kind.kind_name());
    };

    let declaration = statements.first().expect("expected declaration statement");
    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &declaration.kind
    else {
        panic!("expected declaration with initializer, got {}", declaration.kind.kind_name());
    };

    let NodeKind::Binary { op, left, .. } = &initializer.kind else {
        panic!(
            "expected defined-or binary initializer, got {}\n{}",
            initializer.kind.kind_name(),
            initializer.to_sexp()
        );
    };

    assert_eq!(op, "//");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
}

#[test]
fn nested_bare_call_ternary_stays_stable_inside_larger_expression() {
    let ast = parse("my $z = ($flag and is_ready $obj ? 1 : 0) + 5;");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program, got {}", ast.kind.kind_name());
    };

    let declaration = statements.first().expect("expected declaration statement");
    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &declaration.kind
    else {
        panic!("expected declaration with initializer, got {}", declaration.kind.kind_name());
    };

    let NodeKind::Binary { op, left, .. } = &initializer.kind else {
        panic!(
            "expected outer addition expression, got {}\n{}",
            initializer.kind.kind_name(),
            initializer.to_sexp()
        );
    };

    assert_eq!(op, "+");

    let NodeKind::Binary {
        op: left_op,
        right: ternary_candidate,
        ..
    } = &left.kind
    else {
        panic!(
            "expected left side of outer expression to remain `and` binary, got {}\n{}",
            left.kind.kind_name(),
            left.to_sexp()
        );
    };
    assert_eq!(left_op, "and");

    let NodeKind::Ternary { condition, .. } = &ternary_candidate.kind else {
        panic!(
            "expected right side of `and` to remain ternary, got {}\n{}",
            ternary_candidate.kind.kind_name(),
            ternary_candidate.to_sexp()
        );
    };

    assert!(matches!(condition.kind, NodeKind::FunctionCall { .. }));
}
