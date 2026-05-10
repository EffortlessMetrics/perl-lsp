mod cpan_test_helpers;

use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

fn find_unary_op<'a>(node: &'a Node, op: &str) -> Option<&'a Node> {
    if matches!(&node.kind, NodeKind::Unary { op: unary_op, .. } if unary_op == op) {
        return Some(node);
    }

    node.children().into_iter().find_map(|child| find_unary_op(child, op))
}

fn program_statements(ast: &Node) -> Result<&[Node], String> {
    let NodeKind::Program { statements } = &ast.kind else {
        return Err(format!("expected program node, got {}", ast.kind.kind_name()));
    };
    Ok(statements)
}

fn first_statement<'a>(statements: &'a [Node], context: &str) -> Result<&'a Node, String> {
    statements.first().ok_or_else(|| format!("expected one statement for {context}"))
}

fn expression_statement<'a>(stmt: &'a Node, context: &str) -> Result<&'a Node, String> {
    let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
        return Err(format!(
            "expected expression statement for {context}, got {}",
            stmt.kind.kind_name()
        ));
    };
    Ok(expression)
}

fn function_call_name<'a>(expression: &'a Node, context: &str) -> Result<&'a str, String> {
    let NodeKind::FunctionCall { name, .. } = &expression.kind else {
        return Err(format!(
            "expected {context} to stay a function call, got {}",
            expression.kind.kind_name()
        ));
    };
    Ok(name)
}

#[test]
fn async_named_subroutine_carries_async_attribute() -> Result<(), String> {
    let ast = parse("use Future::AsyncAwait; async sub fetch { return await lookup(); }");
    let statements = program_statements(&ast)?;

    let sub = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::Subroutine { .. }))
        .ok_or_else(|| "expected named subroutine statement".to_string())?;

    let NodeKind::Subroutine { name, attributes, body, .. } = &sub.kind else {
        return Err(format!("expected subroutine node, got {}", sub.kind.kind_name()));
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
    Ok(())
}

#[test]
fn await_parses_as_unary_operator() -> Result<(), String> {
    let ast = parse("my $result = await fetch();");
    let statements = program_statements(&ast)?;

    let decl = first_statement(statements, "await variable declaration")?;
    let NodeKind::VariableDeclaration { initializer: Some(initializer), .. } = &decl.kind else {
        return Err(format!(
            "expected variable declaration with initializer, got {}",
            decl.kind.kind_name()
        ));
    };

    let NodeKind::Unary { op, .. } = &initializer.kind else {
        return Err(format!("expected unary initializer, got {}", initializer.kind.kind_name()));
    };

    assert_eq!(op, "await");
    Ok(())
}

#[test]
fn async_bareword_hash_key_stays_parseable() {
    assert_clean_parse("async => 1;");
}

#[test]
fn async_block_stays_parseable_as_a_call() -> Result<(), String> {
    let ast = parse("async { 1 };");
    let statements = program_statements(&ast)?;

    let stmt = first_statement(statements, "`async { ... }`")?;
    let expression = expression_statement(stmt, "`async { ... }`")?;
    let name = function_call_name(expression, "`async { ... }`")?;

    assert_eq!(name, "async");
    Ok(())
}

#[test]
fn package_qualified_await_stays_a_function_call() -> Result<(), String> {
    let ast = parse("await::helper();");
    let statements = program_statements(&ast)?;

    let stmt = first_statement(statements, "`await::helper()`")?;
    let expression = expression_statement(stmt, "`await::helper()`")?;

    assert!(
        find_unary_op(expression, "await").is_none(),
        "expected package-qualified `await::helper()` to avoid unary await parsing, got {}",
        expression.to_sexp()
    );

    let name = function_call_name(expression, "`await::helper()`")?;

    assert_eq!(name, "await::helper");
    Ok(())
}
