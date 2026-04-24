mod cpan_test_helpers;

use cpan_test_helpers::parse;
use perl_parser_core::{Node, NodeKind};

fn first_non_sub_stmt(ast: &Node) -> &Node {
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program root, got {:?}", ast.kind);
    };
    let stmt = statements
        .iter()
        .find(|node| !matches!(node.kind, NodeKind::Subroutine { .. }))
        .expect("expected at least one non-subroutine statement");
    stmt
}

fn assert_call_name(node: &Node, expected: &str) {
    let NodeKind::FunctionCall { name, .. } = &node.kind else {
        panic!("expected function call `{expected}`, got {:?}", node.kind);
    };
    assert_eq!(name, expected, "unexpected function call name");
}

#[test]
fn bare_named_unary_does_not_swallow_ternary() {
    let ast = parse("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    let stmt = first_non_sub_stmt(&ast);

    let NodeKind::VariableDeclaration { initializer, .. } = &stmt.kind else {
        panic!("expected variable declaration, got {:?}", stmt.kind);
    };
    let Some(rhs) = initializer.as_deref() else {
        panic!("expected declaration initializer");
    };
    let NodeKind::Ternary { condition, .. } = &rhs.kind else {
        panic!("expected ternary rhs, got {:?}", rhs.kind);
    };

    assert_call_name(condition, "is_ready");

    let NodeKind::FunctionCall { args, .. } = &condition.kind else {
        unreachable!("checked by assert_call_name");
    };
    assert_eq!(args.len(), 1, "bare call should keep exactly one high-precedence arg");
}

#[test]
fn bare_call_or_die_binds_or_outside_call() {
    let ast = parse("do_thing @args or die;");
    let stmt = first_non_sub_stmt(&ast);
    let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
        panic!("expected expression statement, got {:?}", stmt.kind);
    };
    let expr = expression.as_ref();

    let NodeKind::Binary { op, left, right } = &expr.kind else {
        panic!("expected binary op for `or`, got {:?}", expr.kind);
    };
    assert_eq!(op, "or", "word `or` should be the outer operator");
    assert_call_name(left, "do_thing");
    assert_call_name(right, "die");
}

#[test]
fn bare_call_and_return_binds_and_outside_call() {
    let ast = parse("do_thing $x and return;");
    let stmt = first_non_sub_stmt(&ast);
    let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
        panic!("expected expression statement, got {:?}", stmt.kind);
    };
    let expr = expression.as_ref();

    let NodeKind::Binary { op, left, right } = &expr.kind else {
        panic!("expected binary op for `and`, got {:?}", expr.kind);
    };
    assert_eq!(op, "and", "word `and` should be the outer operator");
    assert_call_name(left, "do_thing");

    let NodeKind::Return { .. } = right.kind else {
        panic!("expected `return` on right side of `and`, got {:?}", right.kind);
    };
}

#[test]
fn bare_call_defined_or_binds_outside_call() {
    let ast = parse("my $v = transform $x // $fallback;");
    let stmt = first_non_sub_stmt(&ast);

    let NodeKind::VariableDeclaration { initializer, .. } = &stmt.kind else {
        panic!("expected variable declaration, got {:?}", stmt.kind);
    };
    let Some(rhs) = initializer.as_deref() else {
        panic!("expected declaration initializer");
    };
    let NodeKind::Binary { op, left, .. } = &rhs.kind else {
        panic!("expected defined-or binary op, got {:?}", rhs.kind);
    };

    assert_eq!(op, "//", "defined-or should bind outside bare call");
    assert_call_name(left, "transform");
}

#[test]
fn nested_bare_call_ternary_stays_condition_only() {
    let ast = parse("my $result = ($ok && is_ready $obj ? 1 : 0) + 2;");
    let stmt = first_non_sub_stmt(&ast);

    let NodeKind::VariableDeclaration { initializer, .. } = &stmt.kind else {
        panic!("expected variable declaration, got {:?}", stmt.kind);
    };
    let Some(rhs) = initializer.as_deref() else {
        panic!("expected declaration initializer");
    };
    let NodeKind::Binary { op, left, .. } = &rhs.kind else {
        panic!("expected outer additive expression, got {:?}", rhs.kind);
    };
    assert_eq!(op, "+", "outer expression should remain additive");

    let NodeKind::Ternary { condition, .. } = &left.kind else {
        panic!("expected ternary on left side of addition, got {:?}", left.kind);
    };
    let NodeKind::Binary { op: cond_op, right, .. } = &condition.kind else {
        panic!("expected binary condition, got {:?}", condition.kind);
    };
    assert_eq!(cond_op, "&&", "condition should remain `$ok && ...`");
    assert_call_name(right, "is_ready");
}
