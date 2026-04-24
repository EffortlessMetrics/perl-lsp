mod cpan_test_helpers;

use cpan_test_helpers::parse;
use perl_parser_core::NodeKind;

fn parse_program_statements(source: &str) -> Vec<perl_parser_core::Node> {
    let ast = parse(source);
    match ast.kind {
        NodeKind::Program { statements } => statements,
        other => panic!("expected Program node, got {:?}", other),
    }
}

#[test]
fn bare_call_condition_in_ternary_stays_outside_call() {
    let statements = parse_program_statements("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    assert_eq!(statements.len(), 2, "expected sub declaration and assignment statement");

    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &statements[1].kind
    else {
        panic!("expected variable declaration with initializer");
    };

    let NodeKind::Ternary { condition, .. } = &initializer.kind else {
        panic!("expected ternary initializer");
    };

    let NodeKind::FunctionCall { name, args } = &condition.kind else {
        panic!("expected ternary condition to be a function call");
    };

    assert_eq!(name, "is_ready");
    assert_eq!(args.len(), 1, "bare call should consume only one high-precedence arg");
    assert!(matches!(args[0].kind, NodeKind::Variable { .. }));
}

#[test]
fn bare_call_with_word_or_keeps_or_outside_call() {
    let statements = parse_program_statements("do_thing @args or die;");
    assert_eq!(statements.len(), 1, "expected one expression statement");

    let NodeKind::ExpressionStatement { expression } = &statements[0].kind else {
        panic!("expected expression statement");
    };

    let NodeKind::Binary { op, left, .. } = &expression.kind else {
        panic!("expected binary expression for word-or");
    };

    assert_eq!(op, "or");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
}

#[test]
fn bare_call_with_word_and_keeps_and_outside_call() {
    let statements = parse_program_statements("do_thing $x and return;");
    assert_eq!(statements.len(), 1, "expected one expression statement");

    let NodeKind::ExpressionStatement { expression } = &statements[0].kind else {
        panic!("expected expression statement");
    };

    let NodeKind::Binary { op, left, .. } = &expression.kind else {
        panic!("expected binary expression for word-and");
    };

    assert_eq!(op, "and");
    assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
}

#[test]
fn bare_call_with_defined_or_keeps_defined_or_outside_call() {
    let statements = parse_program_statements("my $v = transform $x // $fallback;");
    assert_eq!(statements.len(), 1, "expected one variable declaration statement");

    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &statements[0].kind
    else {
        panic!("expected variable declaration with initializer");
    };

    let NodeKind::Binary { op, left, .. } = &initializer.kind else {
        panic!("expected binary expression for defined-or");
    };

    assert_eq!(op, "//");
    let NodeKind::FunctionCall { name, args } = &left.kind else {
        panic!("expected left side of // to be function call");
    };

    assert_eq!(name, "transform");
    assert_eq!(args.len(), 1, "bare call should consume only one high-precedence arg");
}

#[test]
fn nested_bare_call_ternary_binds_inside_larger_expression() {
    let statements = parse_program_statements("my $z = 1 + (is_ready $obj ? 1 : 0);");
    assert_eq!(statements.len(), 1, "expected one variable declaration statement");

    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &statements[0].kind
    else {
        panic!("expected variable declaration with initializer");
    };

    let NodeKind::Binary { op, right, .. } = &initializer.kind else {
        panic!("expected addition expression");
    };

    assert_eq!(op, "+");
    let NodeKind::Ternary { condition, .. } = &right.kind else {
        panic!("expected ternary on right-hand side of addition");
    };

    assert!(matches!(condition.kind, NodeKind::FunctionCall { .. }));
}
