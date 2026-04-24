use perl_parser_core::engine::ast::walk_preorder;
use perl_parser_core::{Node, NodeKind, Parser, SourceLocation};

fn loc() -> SourceLocation {
    SourceLocation { start: 0, end: 0 }
}

fn leaf_number() -> Node {
    Node::new(NodeKind::Number { value: "1".to_string() }, loc())
}

fn leaf_variable(name: &str) -> Node {
    Node::new(NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() }, loc())
}

#[test]
fn walk_preorder_conformance_for_child_bearing_node_kinds() {
    let n = leaf_number();
    let v = leaf_variable("x");

    let cases: Vec<(&str, Node)> = vec![
        ("Program", Node::new(NodeKind::Program { statements: vec![n.clone()] }, loc())),
        (
            "ExpressionStatement",
            Node::new(NodeKind::ExpressionStatement { expression: Box::new(n.clone()) }, loc()),
        ),
        (
            "VariableDeclaration",
            Node::new(
                NodeKind::VariableDeclaration {
                    declarator: "my".to_string(),
                    variable: Box::new(v.clone()),
                    attributes: vec![],
                    initializer: Some(Box::new(n.clone())),
                },
                loc(),
            ),
        ),
        (
            "VariableWithAttributes",
            Node::new(
                NodeKind::VariableWithAttributes {
                    variable: Box::new(v.clone()),
                    attributes: vec!["foo".to_string()],
                },
                loc(),
            ),
        ),
        (
            "Assignment",
            Node::new(
                NodeKind::Assignment {
                    lhs: Box::new(v.clone()),
                    rhs: Box::new(n.clone()),
                    op: "=".to_string(),
                },
                loc(),
            ),
        ),
        (
            "Binary",
            Node::new(
                NodeKind::Binary {
                    op: "+".to_string(),
                    left: Box::new(n.clone()),
                    right: Box::new(n.clone()),
                },
                loc(),
            ),
        ),
        (
            "Unary",
            Node::new(NodeKind::Unary { op: "!".to_string(), operand: Box::new(n.clone()) }, loc()),
        ),
        ("Block", Node::new(NodeKind::Block { statements: vec![n.clone()] }, loc())),
        (
            "If",
            Node::new(
                NodeKind::If {
                    condition: Box::new(v.clone()),
                    then_branch: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    elsif_branches: vec![(
                        Box::new(v.clone()),
                        Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    )],
                    else_branch: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
        ),
        (
            "While",
            Node::new(
                NodeKind::While {
                    condition: Box::new(v.clone()),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
        ),
        (
            "For",
            Node::new(
                NodeKind::For {
                    init: Some(Box::new(n.clone())),
                    condition: Some(Box::new(v.clone())),
                    update: Some(Box::new(n.clone())),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
        ),
        (
            "Foreach",
            Node::new(
                NodeKind::Foreach {
                    variable: Box::new(v.clone()),
                    list: Box::new(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![n.clone()] },
                        loc(),
                    )),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
        ),
        (
            "StatementModifier",
            Node::new(
                NodeKind::StatementModifier {
                    statement: Box::new(Node::new(
                        NodeKind::ExpressionStatement { expression: Box::new(n.clone()) },
                        loc(),
                    )),
                    modifier: "if".to_string(),
                    condition: Box::new(v.clone()),
                },
                loc(),
            ),
        ),
        (
            "FunctionCall",
            Node::new(
                NodeKind::FunctionCall { name: "print".to_string(), args: vec![n.clone()] },
                loc(),
            ),
        ),
        (
            "MethodCall",
            Node::new(
                NodeKind::MethodCall {
                    object: Box::new(v.clone()),
                    method: "m".to_string(),
                    args: vec![n.clone()],
                },
                loc(),
            ),
        ),
        (
            "IndirectCall",
            Node::new(
                NodeKind::IndirectCall {
                    object: Box::new(v.clone()),
                    method: "m".to_string(),
                    args: vec![n.clone()],
                },
                loc(),
            ),
        ),
        (
            "Subroutine",
            Node::new(
                NodeKind::Subroutine {
                    name: Some("f".to_string()),
                    prototype: Some(Box::new(Node::new(
                        NodeKind::Prototype { content: "".to_string() },
                        loc(),
                    ))),
                    signature: Some(Box::new(Node::new(
                        NodeKind::Signature {
                            parameters: vec![Node::new(
                                NodeKind::MandatoryParameter { variable: Box::new(v.clone()) },
                                loc(),
                            )],
                        },
                        loc(),
                    ))),
                    attributes: vec![],
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    name_span: None,
                },
                loc(),
            ),
        ),
        ("Return", Node::new(NodeKind::Return { value: Some(Box::new(n.clone())) }, loc())),
        (
            "Given",
            Node::new(
                NodeKind::Given {
                    expr: Box::new(v.clone()),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
        ),
        (
            "When",
            Node::new(
                NodeKind::When {
                    condition: Box::new(v.clone()),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
        ),
        (
            "Try",
            Node::new(
                NodeKind::Try {
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    catch_blocks: vec![(
                        Some("$e".to_string()),
                        Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    )],
                    finally_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
        ),
        (
            "Error",
            Node::new(
                NodeKind::Error {
                    message: "x".to_string(),
                    expected: vec![],
                    found: None,
                    partial: Some(Box::new(n.clone())),
                },
                loc(),
            ),
        ),
    ];

    for (name, root) in cases {
        let mut visited = 0usize;
        walk_preorder(&root, &mut |_| {
            visited += 1;
        });
        assert_eq!(
            visited,
            root.count_nodes(),
            "walk_preorder should match count_nodes for {name}"
        );
    }
}

#[test]
fn walk_preorder_covers_all_statement_modifiers_and_nested_conditions() {
    let source = r#"
print "ok" if $x = 5;
print "ok" unless $x = 5;
print "ok" while $x = 5;
print "ok" until $x = 5;
print "ok" for @items;
print "ok" foreach @items;
print "ok" if ($x = 5) unless ($y = 7);
"#;

    let ast = Parser::new(source).parse_with_recovery().ast;

    let mut print_calls = 0usize;
    let mut modifiers = std::collections::BTreeSet::new();
    let mut visited = std::collections::BTreeSet::new();
    let mut expected_statement_nodes = std::collections::BTreeSet::new();
    let mut expected_condition_nodes = std::collections::BTreeSet::new();

    walk_preorder(&ast, &mut |node| match &node.kind {
        NodeKind::FunctionCall { name, .. } if name == "print" => {
            print_calls += 1;
            visited.insert(node as *const Node as usize);
        }
        NodeKind::StatementModifier { modifier, statement, condition } => {
            modifiers.insert(modifier.clone());
            expected_statement_nodes.insert(statement.as_ref() as *const Node as usize);
            expected_condition_nodes.insert(condition.as_ref() as *const Node as usize);
            visited.insert(node as *const Node as usize);
        }
        _ => {
            visited.insert(node as *const Node as usize);
        }
    });

    assert_eq!(print_calls, 7);
    assert!(expected_statement_nodes.is_subset(&visited));
    assert!(expected_condition_nodes.is_subset(&visited));
    assert_eq!(
        modifiers,
        ["for", "foreach", "if", "unless", "until", "while"]
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect()
    );
}
