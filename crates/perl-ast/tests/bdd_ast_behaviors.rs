//! Behavior-driven scenarios for the `perl-ast` crate.
//!
//! These scenarios focus on externally visible behavior from the perspective of
//! parser/editor integrations that consume AST nodes.

use perl_ast::ast::{Node, NodeKind, SourceLocation};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn number(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn variable(sigil: &str, name: &str) -> Node {
    Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        loc(0, sigil.len() + name.len()),
    )
}

fn block(statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements }, loc(0, 1))
}

#[test]
fn scenario_given_a_program_when_requesting_first_child_then_the_first_statement_is_returned()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let first = number("1");
    let second = number("2");
    let program = Node::new(NodeKind::Program { statements: vec![first, second] }, loc(0, 3));

    // When
    let maybe_first = program.first_child();

    // Then
    let child = maybe_first.ok_or("expected first child")?;
    match &child.kind {
        NodeKind::Number { value } => assert_eq!(value, "1"),
        _ => return Err("expected Number child".into()),
    }

    Ok(())
}

#[test]
fn scenario_given_nested_control_flow_when_counting_nodes_then_total_includes_all_descendants()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let ast = Node::new(
        NodeKind::If {
            condition: Box::new(variable("$", "ok")),
            then_branch: Box::new(block(vec![Node::new(
                NodeKind::While {
                    condition: Box::new(number("1")),
                    body: Box::new(block(vec![number("42")])),
                    continue_block: None,
                },
                loc(0, 12),
            )])),
            elsif_branches: vec![],
            else_branch: Some(Box::new(block(vec![number("0")]))),
        },
        loc(0, 20),
    );

    // When
    let total = ast.count_nodes();

    // Then
    // if + condition + then_block + while + while_condition + while_body + literal + else_block + else_literal
    assert_eq!(total, 9);
    Ok(())
}

#[test]
fn scenario_given_nested_binary_expression_when_collecting_children_then_order_matches_source_shape()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let expr = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(Node::new(
                NodeKind::Binary {
                    op: "*".to_string(),
                    left: Box::new(number("2")),
                    right: Box::new(number("3")),
                },
                loc(0, 3),
            )),
            right: Box::new(number("4")),
        },
        loc(0, 5),
    );

    // When
    let children = expr.children();

    // Then
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].kind.kind_name(), "Binary");
    assert_eq!(children[1].kind.kind_name(), "Number");
    Ok(())
}

#[test]
fn scenario_given_string_literal_with_quotes_when_serializing_then_quotes_are_escaped()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let node = Node::new(
        NodeKind::String { value: "say \"hi\"".to_string(), interpolated: false },
        loc(0, 9),
    );

    // When
    let sexp = node.to_sexp();

    // Then
    assert_eq!(sexp, "(string \"say \\\"hi\\\"\")");
    Ok(())
}

#[test]
fn scenario_given_interpolated_string_when_serializing_then_interpolated_form_is_used()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let node = Node::new(
        NodeKind::String { value: "hello $name".to_string(), interpolated: true },
        loc(0, 11),
    );

    // When
    let sexp = node.to_sexp();

    // Then
    assert_eq!(sexp, "(string_interpolated \"hello $name\")");
    Ok(())
}
