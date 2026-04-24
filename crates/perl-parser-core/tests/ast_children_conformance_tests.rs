use perl_parser_core::SourceLocation;
use perl_parser_core::ast::{Node, NodeKind};

fn loc() -> SourceLocation {
    SourceLocation { start: 0, end: 1 }
}

fn id(name: &str) -> Node {
    Node::new(NodeKind::Identifier { name: name.to_string() }, loc())
}

fn var(name: &str) -> Node {
    Node::new(NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() }, loc())
}

fn num(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc())
}

#[test]
fn child_bearing_node_kinds_expose_children_consistently() {
    let mut cases: Vec<(String, Node, usize)> = vec![
        (
            "Tie".to_string(),
            Node::new(
                NodeKind::Tie {
                    variable: Box::new(var("x")),
                    package: Box::new(id("Pkg")),
                    args: vec![num("1")],
                },
                loc(),
            ),
            3,
        ),
        (
            "Untie".to_string(),
            Node::new(NodeKind::Untie { variable: Box::new(var("x")) }, loc()),
            1,
        ),
        (
            "Program".to_string(),
            Node::new(NodeKind::Program { statements: vec![id("stmt")] }, loc()),
            1,
        ),
        (
            "ExpressionStatement".to_string(),
            Node::new(NodeKind::ExpressionStatement { expression: Box::new(id("expr")) }, loc()),
            1,
        ),
        (
            "VariableDeclaration".to_string(),
            Node::new(
                NodeKind::VariableDeclaration {
                    declarator: "my".to_string(),
                    variable: Box::new(var("x")),
                    attributes: vec![],
                    initializer: Some(Box::new(num("1"))),
                },
                loc(),
            ),
            2,
        ),
        (
            "VariableListDeclaration".to_string(),
            Node::new(
                NodeKind::VariableListDeclaration {
                    declarator: "my".to_string(),
                    variables: vec![var("x"), var("y")],
                    attributes: vec![],
                    initializer: Some(Box::new(num("1"))),
                },
                loc(),
            ),
            3,
        ),
        (
            "VariableWithAttributes".to_string(),
            Node::new(
                NodeKind::VariableWithAttributes {
                    variable: Box::new(var("x")),
                    attributes: vec!["ro".to_string()],
                },
                loc(),
            ),
            1,
        ),
        (
            "Binary".to_string(),
            Node::new(
                NodeKind::Binary {
                    left: Box::new(num("1")),
                    op: "+".to_string(),
                    right: Box::new(num("2")),
                },
                loc(),
            ),
            2,
        ),
        (
            "Ternary".to_string(),
            Node::new(
                NodeKind::Ternary {
                    condition: Box::new(id("cond")),
                    then_expr: Box::new(num("1")),
                    else_expr: Box::new(num("0")),
                },
                loc(),
            ),
            3,
        ),
        (
            "Unary".to_string(),
            Node::new(NodeKind::Unary { op: "!".to_string(), operand: Box::new(id("x")) }, loc()),
            1,
        ),
        (
            "Assignment".to_string(),
            Node::new(
                NodeKind::Assignment {
                    lhs: Box::new(var("x")),
                    op: "=".to_string(),
                    rhs: Box::new(num("5")),
                },
                loc(),
            ),
            2,
        ),
        (
            "Block".to_string(),
            Node::new(NodeKind::Block { statements: vec![id("stmt")] }, loc()),
            1,
        ),
        (
            "If".to_string(),
            Node::new(
                NodeKind::If {
                    condition: Box::new(id("cond")),
                    then_branch: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    elsif_branches: vec![(
                        Box::new(id("elsif")),
                        Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    )],
                    else_branch: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            5,
        ),
        (
            "While".to_string(),
            Node::new(
                NodeKind::While {
                    condition: Box::new(id("cond")),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            3,
        ),
        (
            "For".to_string(),
            Node::new(
                NodeKind::For {
                    init: Some(Box::new(id("init"))),
                    condition: Some(Box::new(id("cond"))),
                    update: Some(Box::new(id("upd"))),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            5,
        ),
        (
            "Foreach".to_string(),
            Node::new(
                NodeKind::Foreach {
                    variable: Box::new(var("item")),
                    list: Box::new(id("list")),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            4,
        ),
        (
            "Given".to_string(),
            Node::new(
                NodeKind::Given {
                    expr: Box::new(id("x")),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            2,
        ),
        (
            "When".to_string(),
            Node::new(
                NodeKind::When {
                    condition: Box::new(id("cond")),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            2,
        ),
        (
            "Default".to_string(),
            Node::new(
                NodeKind::Default {
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            1,
        ),
        (
            "LabeledStatement".to_string(),
            Node::new(
                NodeKind::LabeledStatement {
                    label: "L1".to_string(),
                    statement: Box::new(id("stmt")),
                },
                loc(),
            ),
            1,
        ),
        (
            "Eval".to_string(),
            Node::new(
                NodeKind::Eval {
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            1,
        ),
        (
            "Do".to_string(),
            Node::new(
                NodeKind::Do {
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            1,
        ),
        (
            "Defer".to_string(),
            Node::new(
                NodeKind::Defer {
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            1,
        ),
        (
            "Try".to_string(),
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
            3,
        ),
        (
            "FunctionCall".to_string(),
            Node::new(
                NodeKind::FunctionCall { name: "f".to_string(), args: vec![num("1")] },
                loc(),
            ),
            1,
        ),
        (
            "MethodCall".to_string(),
            Node::new(
                NodeKind::MethodCall {
                    object: Box::new(id("obj")),
                    method: "m".to_string(),
                    args: vec![num("1")],
                },
                loc(),
            ),
            2,
        ),
        (
            "IndirectCall".to_string(),
            Node::new(
                NodeKind::IndirectCall {
                    method: "m".to_string(),
                    object: Box::new(id("obj")),
                    args: vec![num("1")],
                },
                loc(),
            ),
            2,
        ),
        (
            "Subroutine".to_string(),
            Node::new(
                NodeKind::Subroutine {
                    name: Some("f".to_string()),
                    name_span: None,
                    prototype: Some(Box::new(Node::new(
                        NodeKind::Prototype { content: "$".to_string() },
                        loc(),
                    ))),
                    signature: Some(Box::new(Node::new(
                        NodeKind::Signature {
                            parameters: vec![Node::new(
                                NodeKind::MandatoryParameter { variable: Box::new(var("x")) },
                                loc(),
                            )],
                        },
                        loc(),
                    ))),
                    attributes: vec![],
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            3,
        ),
        (
            "Method".to_string(),
            Node::new(
                NodeKind::Method {
                    name: "m".to_string(),
                    signature: Some(Box::new(Node::new(
                        NodeKind::Signature {
                            parameters: vec![Node::new(
                                NodeKind::MandatoryParameter { variable: Box::new(var("x")) },
                                loc(),
                            )],
                        },
                        loc(),
                    ))),
                    attributes: vec![],
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            2,
        ),
        (
            "Return".to_string(),
            Node::new(NodeKind::Return { value: Some(Box::new(num("1"))) }, loc()),
            1,
        ),
        ("Goto".to_string(), Node::new(NodeKind::Goto { target: Box::new(id("L1")) }, loc()), 1),
        (
            "Signature".to_string(),
            Node::new(
                NodeKind::Signature {
                    parameters: vec![Node::new(
                        NodeKind::MandatoryParameter { variable: Box::new(var("x")) },
                        loc(),
                    )],
                },
                loc(),
            ),
            1,
        ),
        (
            "MandatoryParameter".to_string(),
            Node::new(NodeKind::MandatoryParameter { variable: Box::new(var("x")) }, loc()),
            1,
        ),
        (
            "OptionalParameter".to_string(),
            Node::new(
                NodeKind::OptionalParameter {
                    variable: Box::new(var("x")),
                    default_value: Box::new(num("1")),
                },
                loc(),
            ),
            2,
        ),
        (
            "SlurpyParameter".to_string(),
            Node::new(NodeKind::SlurpyParameter { variable: Box::new(var("x")) }, loc()),
            1,
        ),
        (
            "NamedParameter".to_string(),
            Node::new(NodeKind::NamedParameter { variable: Box::new(var("x")) }, loc()),
            1,
        ),
        (
            "Match".to_string(),
            Node::new(
                NodeKind::Match {
                    expr: Box::new(var("x")),
                    pattern: "x".to_string(),
                    modifiers: "".to_string(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc(),
            ),
            1,
        ),
        (
            "Substitution".to_string(),
            Node::new(
                NodeKind::Substitution {
                    expr: Box::new(var("x")),
                    pattern: "x".to_string(),
                    replacement: "y".to_string(),
                    modifiers: "".to_string(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc(),
            ),
            1,
        ),
        (
            "Transliteration".to_string(),
            Node::new(
                NodeKind::Transliteration {
                    expr: Box::new(var("x")),
                    search: "a".to_string(),
                    replace: "b".to_string(),
                    modifiers: "".to_string(),
                    negated: false,
                },
                loc(),
            ),
            1,
        ),
        (
            "ArrayLiteral".to_string(),
            Node::new(NodeKind::ArrayLiteral { elements: vec![num("1"), num("2")] }, loc()),
            2,
        ),
        (
            "HashLiteral".to_string(),
            Node::new(NodeKind::HashLiteral { pairs: vec![(id("k"), num("1"))] }, loc()),
            2,
        ),
        (
            "Package".to_string(),
            Node::new(
                NodeKind::Package {
                    name: "Pkg".to_string(),
                    name_span: loc(),
                    block: Some(Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc()))),
                },
                loc(),
            ),
            1,
        ),
        (
            "PhaseBlock".to_string(),
            Node::new(
                NodeKind::PhaseBlock {
                    phase: "BEGIN".to_string(),
                    phase_span: None,
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            1,
        ),
        (
            "Class".to_string(),
            Node::new(
                NodeKind::Class {
                    name: "C".to_string(),
                    parents: vec!["Base".to_string()],
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                },
                loc(),
            ),
            1,
        ),
        (
            "Error".to_string(),
            Node::new(
                NodeKind::Error {
                    message: "err".to_string(),
                    expected: vec![],
                    found: None,
                    partial: Some(Box::new(id("partial"))),
                },
                loc(),
            ),
            1,
        ),
    ];

    for modifier in ["if", "unless", "while", "until", "for", "foreach"] {
        cases.push((
            format!("StatementModifier({modifier})"),
            Node::new(
                NodeKind::StatementModifier {
                    statement: Box::new(Node::new(
                        NodeKind::ExpressionStatement {
                            expression: Box::new(Node::new(
                                NodeKind::FunctionCall {
                                    name: "print".to_string(),
                                    args: vec![id("ok")],
                                },
                                loc(),
                            )),
                        },
                        loc(),
                    )),
                    modifier: modifier.to_string(),
                    condition: Box::new(Node::new(
                        NodeKind::ExpressionStatement {
                            expression: Box::new(Node::new(
                                NodeKind::Assignment {
                                    lhs: Box::new(var("x")),
                                    op: "=".to_string(),
                                    rhs: Box::new(num("5")),
                                },
                                loc(),
                            )),
                        },
                        loc(),
                    )),
                },
                loc(),
            ),
            2,
        ));
    }

    for (name, node, expected_children) in cases {
        assert_eq!(
            node.children().len(),
            expected_children,
            "{name} should expose expected direct child count"
        );
    }
}
