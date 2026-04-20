#![allow(clippy::panic)]

mod cpan_test_helpers;

use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

fn find_unary_op<'a>(node: &'a Node, op: &str) -> Option<&'a Node> {
    if matches!(&node.kind, NodeKind::Unary { op: unary_op, .. } if unary_op == op) {
        return Some(node);
    }

    node.children().into_iter().find_map(|child| find_unary_op(child, op))
}

#[test]
fn async_named_subroutine_carries_async_attribute() {
    let ast = parse("use Future::AsyncAwait; async sub fetch { return await lookup(); }");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected program node, got {}", ast.kind.kind_name());
    };

    let sub = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::Subroutine { .. }))
        .unwrap();

    let NodeKind::Subroutine { name, attributes, body, .. } = &sub.kind else {
        panic!("expected subroutine node, got {}", sub.kind.kind_name());
    };

    assert_eq!(name.as_deref(), Some("fetch"));
    assert!(
        attributes.iter().any(|attr| attr == "async"),
        "expected `async` attribute on async subroutine, got {attributes:?}"
    );
    assert!(
        find_unary_op(body, "await").is_some(),
        "expected unary `await` inside async subroutine body, got {}",
        body.to_sexp()
    );
}

#[test]
fn await_parses_as_unary_operator() {
    let ast = parse("my $result = await fetch();");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected program node, got {}", ast.kind.kind_name());
    };

    let decl = statements.first().unwrap();
    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &decl.kind else {
        panic!("expected variable declaration with initializer, got {}", decl.kind.kind_name());
    };

    let NodeKind::Unary { op, .. } = &initializer.kind else {
        panic!("expected unary initializer, got {}", initializer.kind.kind_name());
    };

    assert_eq!(op, "await");
}

#[test]
fn async_bareword_hash_key_stays_parseable() {
    assert_clean_parse("async => 1;");
}

#[test]
fn async_block_stays_parseable_as_a_call() {
    let ast = parse("async { 1 };");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected program node, got {}", ast.kind.kind_name());
    };

    let stmt = statements.first().unwrap();
    let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
        panic!(
            "expected expression statement for `async {{ ... }}`, got {}",
            stmt.kind.kind_name()
        );
    };

    let NodeKind::FunctionCall { name, .. } = &expression.kind else {
        panic!(
            "expected `async {{ ... }}` to stay a function call, got {}",
            expression.kind.kind_name()
        );
    };

    assert_eq!(name, "async");
}

#[test]
fn package_qualified_await_stays_a_function_call() {
    let ast = parse("await::helper();");
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected program node, got {}", ast.kind.kind_name());
    };

    let stmt = statements.first().unwrap();
    let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
        panic!(
            "expected expression statement for `await::helper()`, got {}",
            stmt.kind.kind_name()
        );
    };

    assert!(
        find_unary_op(expression, "await").is_none(),
        "expected package-qualified `await::helper()` to avoid unary await parsing, got {}",
        expression.to_sexp()
    );

    let NodeKind::FunctionCall { name, .. } = &expression.kind else {
        panic!(
            "expected `await::helper()` to stay a function call, got {}",
            expression.kind.kind_name()
        );
    };

    assert_eq!(name, "await::helper");
}
