mod cpan_test_helpers;

use cpan_test_helpers::parse;
use perl_parser_core::{Node, NodeKind};

fn program_statements(ast: &Node) -> &[Node] {
    match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => panic!("Expected Program node, got {:?}", other),
    }
}

fn expr_from_stmt(stmt: &Node) -> &Node {
    match &stmt.kind {
        NodeKind::ExpressionStatement { expression } => expression,
        other => panic!("Expected ExpressionStatement, got {:?}", other),
    }
}

fn initializer_from_var_decl(stmt: &Node) -> &Node {
    match &stmt.kind {
        NodeKind::VariableDeclaration { initializer: Some(initializer), .. } => initializer,
        other => panic!("Expected VariableDeclaration with initializer, got {:?}", other),
    }
}

#[test]
fn test_bare_call_sigil_arg_does_not_absorb_ternary() {
    let ast = parse("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    let statements = program_statements(&ast);

    let init = initializer_from_var_decl(&statements[1]);
    let NodeKind::Ternary { condition, .. } = &init.kind else {
        panic!("Expected ternary initializer, got {:?}", init.kind);
    };

    let NodeKind::FunctionCall { name, args } = &condition.kind else {
        panic!("Expected bare call condition, got {:?}", condition.kind);
    };
    assert_eq!(name, "is_ready");
    assert_eq!(args.len(), 1, "bare call should keep only one high-precedence arg");
}

#[test]
fn test_bare_call_array_arg_or_die_binds_outside_call() {
    let ast = parse("do_thing @args or die;");
    let statements = program_statements(&ast);
    let expr = expr_from_stmt(&statements[0]);

    let NodeKind::Binary { op, left, right } = &expr.kind else {
        panic!("Expected word-op binary, got {:?}", expr.kind);
    };
    assert_eq!(op, "or");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
    assert!(matches!(right.kind, NodeKind::FunctionCall { name: ref n, .. } if n == "die"));
}

#[test]
fn test_bare_call_scalar_arg_and_return_binds_outside_call() {
    let ast = parse("do_thing $x and return;");
    let statements = program_statements(&ast);
    let expr = expr_from_stmt(&statements[0]);

    let NodeKind::Binary { op, left, right } = &expr.kind else {
        panic!("Expected word-op binary, got {:?}", expr.kind);
    };
    assert_eq!(op, "and");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
    assert!(matches!(right.kind, NodeKind::Return { value: None }));
}

#[test]
fn test_bare_call_defined_or_operator_binds_outside_call() {
    let ast = parse("my $v = transform $x // $fallback;");
    let statements = program_statements(&ast);

    let init = initializer_from_var_decl(&statements[0]);
    let NodeKind::Binary { op, left, .. } = &init.kind else {
        panic!("Expected defined-or binary, got {:?}", init.kind);
    };
    assert_eq!(op, "//");
    assert!(
        matches!(left.kind, NodeKind::FunctionCall { name: ref n, args: ref a } if n == "transform" && a.len() == 1)
    );
}

#[test]
fn test_nested_bare_call_ternary_inside_larger_expression() {
    let ast = parse("my $n = 1 + (is_ready $obj ? 1 : 0);");
    let statements = program_statements(&ast);

    let init = initializer_from_var_decl(&statements[0]);
    let NodeKind::Binary { op, right, .. } = &init.kind else {
        panic!("Expected top-level binary expression, got {:?}", init.kind);
    };
    assert_eq!(op, "+");
    assert!(matches!(right.kind, NodeKind::Ternary { .. }));
}
