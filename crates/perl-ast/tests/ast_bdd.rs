use perl_ast::{Node, NodeKind, SourceLocation};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn num(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn block(statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements }, loc(0, 1))
}

#[test]
fn given_program_with_expression_statement_when_rendering_sexp_then_source_file_wraps_statement()
-> Result<(), Box<dyn std::error::Error>> {
    let stmt =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(num("42")) }, loc(0, 2));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 2));

    let sexp = program.to_sexp();

    assert!(sexp.starts_with("(source_file"), "expected source_file root, got: {sexp}");
    assert!(
        sexp.contains("(number 42)"),
        "expected expression statement payload, got: {sexp}"
    );
    Ok(())
}

#[test]
fn given_if_with_elsif_and_else_when_counting_nodes_then_subtree_count_is_inclusive()
-> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::If {
            condition: Box::new(num("1")),
            then_branch: Box::new(block(vec![num("2")])),
            elsif_branches: vec![(Box::new(num("3")), Box::new(block(vec![num("4")])))],
            else_branch: Some(Box::new(block(vec![num("5")]))),
        },
        loc(0, 20),
    );

    // if + condition + then_block + stmt + elsif_condition + elsif_block + stmt + else_block + stmt
    assert_eq!(node.count_nodes(), 9);
    Ok(())
}

#[test]
fn given_binary_expression_when_collecting_children_then_children_are_returned_in_semantic_order()
-> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num("10")),
            right: Box::new(num("20")),
        },
        loc(0, 5),
    );

    let children = expr.children();

    assert_eq!(children.len(), 2);
    assert!(matches!(children[0].kind, NodeKind::Number { .. }));
    assert!(matches!(children[1].kind, NodeKind::Number { .. }));
    assert_eq!(children[0].location.end, 2);
    assert_eq!(children[1].location.end, 2);
    Ok(())
}

#[test]
fn given_leaf_node_when_requesting_first_child_then_none_is_returned()
-> Result<(), Box<dyn std::error::Error>> {
    let leaf = num("7");

    assert!(leaf.first_child().is_none());
    Ok(())
}

#[test]
fn given_program_children_when_visiting_mutably_then_each_direct_child_is_mutated()
-> Result<(), Box<dyn std::error::Error>> {
    let mut program =
        Node::new(NodeKind::Program { statements: vec![num("1"), num("2"), num("3")] }, loc(0, 3));

    program.for_each_child_mut(|child| {
        if let NodeKind::Number { value } = &mut child.kind {
            value.push('0');
        }
    });

    match &program.kind {
        NodeKind::Program { statements } => {
            let values = statements
                .iter()
                .filter_map(|node| match &node.kind {
                    NodeKind::Number { value } => Some(value.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(values, vec!["10", "20", "30"]);
        }
        other => {
            return Err(format!("expected Program node, got {}", other.kind_name()).into());
        }
    }

    Ok(())
}
