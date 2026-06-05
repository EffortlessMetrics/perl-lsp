//! BDD-style behavior specification tests for `perl-ast`.
//!
//! These tests focus on externally observable AST behavior: serialization,
//! traversal shape, mutation hooks, and convenience APIs.

use perl_ast::{Node, NodeKind, SourceLocation};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn number(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn ident(name: &str) -> Node {
    Node::new(NodeKind::Identifier { name: name.to_string() }, loc(0, name.len()))
}

fn block(statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements }, loc(0, 1))
}

#[test]
fn when_serializing_a_program_then_output_uses_source_file_root() {
    let node = Node::new(NodeKind::Program { statements: vec![number("42")] }, loc(0, 2));

    let sexp = node.to_sexp();

    assert!(sexp.starts_with("(source_file"), "expected source_file root, got: {sexp}");
    assert!(sexp.contains("(number 42)"), "expected nested number, got: {sexp}");
}

#[test]
fn when_serializing_variable_declaration_with_initializer_then_both_parts_are_emitted() {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(3, 5),
            )),
            attributes: vec![],
            initializer: Some(Box::new(number("1"))),
        },
        loc(0, 10),
    );

    let sexp = decl.to_sexp();

    assert!(sexp.contains("(my_declaration"), "expected declarator tag, got: {sexp}");
    assert!(sexp.contains("(variable $ x)"), "expected declared variable, got: {sexp}");
    assert!(sexp.contains("(number 1)"), "expected initializer number, got: {sexp}");
}

#[test]
fn when_calling_children_on_if_with_elsif_and_else_then_all_direct_children_are_returned() {
    let node = Node::new(
        NodeKind::If {
            condition: Box::new(ident("cond")),
            then_branch: Box::new(block(vec![])),
            elsif_branches: vec![(Box::new(ident("other")), Box::new(block(vec![])))],
            else_branch: Some(Box::new(block(vec![]))),
            keyword: None,
        },
        loc(0, 30),
    );

    let children = node.children();

    assert_eq!(children.len(), 5, "expected condition + then + elsif(cond/body) + else");
    assert_eq!(children[0].kind.kind_name(), "Identifier");
    assert_eq!(children[1].kind.kind_name(), "Block");
}

#[test]
fn when_requesting_first_child_on_leaf_node_then_none_is_returned() {
    let leaf = number("9");

    assert!(leaf.first_child().is_none());
}

#[test]
fn when_requesting_first_child_on_non_leaf_then_first_direct_child_is_returned() {
    let node = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(number("1")),
            right: Box::new(number("2")),
        },
        loc(0, 3),
    );

    let child = node.first_child();

    assert!(child.is_some());
    assert_eq!(child.map(|c| c.kind.kind_name()), Some("Number"));
}

#[test]
fn when_counting_nodes_in_nested_tree_then_total_includes_all_descendants() {
    let tree = Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::ExpressionStatement {
                        expression: Box::new(Node::new(
                            NodeKind::Binary {
                                op: "+".to_string(),
                                left: Box::new(number("1")),
                                right: Box::new(number("2")),
                            },
                            loc(0, 3),
                        )),
                    },
                    loc(0, 3),
                ),
                Node::new(
                    NodeKind::ExpressionStatement { expression: Box::new(number("3")) },
                    loc(4, 5),
                ),
            ],
        },
        loc(0, 5),
    );

    assert_eq!(tree.count_nodes(), 7);
}

#[test]
fn when_mutating_children_with_for_each_child_mut_then_changes_are_persisted() {
    let mut array =
        Node::new(NodeKind::ArrayLiteral { elements: vec![number("1"), number("2")] }, loc(0, 5));

    array.for_each_child_mut(|child| {
        if let NodeKind::Number { value } = &mut child.kind {
            value.push('0');
        }
    });

    let values = array
        .children()
        .into_iter()
        .filter_map(|child| match &child.kind {
            NodeKind::Number { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(values, vec!["10".to_string(), "20".to_string()]);
}

#[test]
fn when_traversing_for_node_without_optional_parts_then_only_body_is_visited() {
    let node = Node::new(
        NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        loc(0, 8),
    );

    let mut visited = 0usize;
    node.for_each_child(|_| visited += 1);

    assert_eq!(visited, 1);
}

#[test]
fn when_checking_contains_offset_then_start_is_inclusive_and_end_is_exclusive() {
    let node = Node::new(NodeKind::Identifier { name: "foo".to_string() }, loc(3, 7));

    assert!(node.contains_offset(3));
    assert!(node.contains_offset(6));
    assert!(!node.contains_offset(7));
    assert!(!node.contains_offset(2));
}

#[test]
fn when_requesting_span_len_then_it_matches_location_width() {
    let node = Node::new(NodeKind::Identifier { name: "foo".to_string() }, loc(10, 14));
    assert_eq!(node.span_len(), 4);
}

#[test]
fn when_requesting_last_child_on_leaf_node_then_none_is_returned() {
    let leaf = number("9");

    assert!(leaf.last_child().is_none());
}

#[test]
fn when_requesting_last_child_on_single_child_node_then_that_child_is_returned() {
    let node = Node::new(
        NodeKind::Unary { op: "-".to_string(), operand: Box::new(number("1")) },
        loc(0, 2),
    );

    let child = node.last_child();

    assert!(child.is_some());
    assert_eq!(child.map(|c| c.kind.kind_name()), Some("Number"));
}

#[test]
fn when_requesting_last_child_on_binary_node_then_right_operand_is_returned() {
    let node = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(ident("a")),
            right: Box::new(number("2")),
        },
        loc(0, 3),
    );

    let child = node.last_child();

    assert_eq!(child.map(|c| c.kind.kind_name()), Some("Number"));
}

#[test]
fn when_requesting_last_child_on_program_then_last_statement_is_returned() {
    let node = Node::new(
        NodeKind::Program { statements: vec![number("1"), number("2"), ident("last")] },
        loc(0, 10),
    );

    let child = node.last_child();

    assert_eq!(child.map(|c| c.kind.kind_name()), Some("Identifier"));
}

#[test]
fn when_finding_deepest_node_at_offset_then_smallest_containing_descendant_is_returned()
-> TestResult {
    let tree = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::ExpressionStatement {
                    expression: Box::new(Node::new(
                        NodeKind::Binary {
                            op: "+".to_string(),
                            left: Box::new(Node::new(
                                NodeKind::Identifier { name: "left".to_string() },
                                loc(0, 4),
                            )),
                            right: Box::new(Node::new(
                                NodeKind::Number { value: "42".to_string() },
                                loc(7, 9),
                            )),
                        },
                        loc(0, 9),
                    )),
                },
                loc(0, 10),
            )],
        },
        loc(0, 10),
    );

    let found = tree.find_deepest_containing_offset(8);

    assert_eq!(found.map(|node| node.kind.kind_name()), Some("Number"));
    Ok(())
}

#[test]
fn when_finding_deepest_node_at_gap_then_nearest_containing_ancestor_is_returned() -> TestResult {
    let tree = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(Node::new(NodeKind::Identifier { name: "left".to_string() }, loc(0, 4))),
            right: Box::new(Node::new(NodeKind::Number { value: "42".to_string() }, loc(7, 9))),
        },
        loc(0, 9),
    );

    let found = tree.find_deepest_containing_offset(5);

    assert_eq!(found.map(|node| node.kind.kind_name()), Some("Binary"));
    Ok(())
}

#[test]
fn when_finding_deepest_node_outside_span_then_none_is_returned() -> TestResult {
    let tree = Node::new(NodeKind::Identifier { name: "left".to_string() }, loc(0, 4));

    assert_eq!(tree.find_deepest_containing_offset(4), None);
    Ok(())
}
