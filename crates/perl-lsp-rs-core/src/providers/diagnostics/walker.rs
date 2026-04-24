//! AST walker for diagnostics
//!
//! This module provides a generic AST walker function for traversing
//! Perl AST nodes and applying diagnostic checks.

use perl_parser_core::ast::Node;

/// Walk the AST and call a function for each node
///
/// This function recursively walks the AST and calls the provided function
/// for each node. The function is called before visiting children (pre-order).
pub fn walk_node<F>(node: &Node, func: &mut F)
where
    F: FnMut(&Node),
{
    func(node);
    for child in node.children() {
        walk_node(child, func);
    }
}

#[cfg(test)]
mod tests {
    use super::walk_node;
    use perl_parser_core::Parser;
    use perl_parser_core::ast::{Node, NodeKind, SourceLocation};

    fn loc() -> SourceLocation {
        SourceLocation { start: 0, end: 1 }
    }

    fn number(value: &str) -> Node {
        Node::new(NodeKind::Number { value: value.to_string() }, loc())
    }

    fn variable(name: &str) -> Node {
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() }, loc())
    }

    fn block() -> Node {
        Node::new(NodeKind::Block { statements: vec![number("1")] }, loc())
    }

    fn collect_kind_names(source: &str) -> Vec<&'static str> {
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("statement modifier fixture should parse");
        let mut kinds = Vec::new();
        walk_node(&ast, &mut |node| kinds.push(node.kind.kind_name()));
        kinds
    }

    #[test]
    fn statement_modifier_walk_visits_statement_and_condition_for_all_modifiers() {
        for modifier in ["if", "unless", "while", "until", "for", "foreach"] {
            let source = format!("print \"ok\" {modifier} $x = 5;");
            let kind_names = collect_kind_names(&source);
            assert!(
                kind_names.iter().filter(|kind_name| **kind_name == "StatementModifier").count()
                    >= 1,
                "expected StatementModifier in AST for modifier {modifier}, got: {kind_names:?}",
            );
            assert!(
                kind_names.contains(&"FunctionCall"),
                "expected print statement subtree to be traversed for modifier {modifier}, got: {kind_names:?}",
            );
            assert!(
                kind_names.contains(&"Assignment"),
                "expected assignment condition subtree to be traversed for modifier {modifier}, got: {kind_names:?}",
            );
        }
    }

    #[test]
    fn nested_statement_modifiers_are_walked_recursively() {
        let nested = Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(Node::new(
                    NodeKind::StatementModifier {
                        statement: Box::new(Node::new(
                            NodeKind::ExpressionStatement {
                                expression: Box::new(Node::new(
                                    NodeKind::FunctionCall {
                                        name: "print".to_string(),
                                        args: vec![number("1")],
                                    },
                                    loc(),
                                )),
                            },
                            loc(),
                        )),
                        modifier: "if".to_string(),
                        condition: Box::new(Node::new(
                            NodeKind::Assignment {
                                lhs: Box::new(variable("x")),
                                rhs: Box::new(number("5")),
                                op: "=".to_string(),
                            },
                            loc(),
                        )),
                    },
                    loc(),
                )),
                modifier: "unless".to_string(),
                condition: Box::new(Node::new(
                    NodeKind::Assignment {
                        lhs: Box::new(variable("ready")),
                        rhs: Box::new(number("1")),
                        op: "=".to_string(),
                    },
                    loc(),
                )),
            },
            loc(),
        );

        let mut kind_names = Vec::new();
        walk_node(&nested, &mut |node| kind_names.push(node.kind.kind_name()));
        let statement_modifier_count =
            kind_names.iter().filter(|kind_name| **kind_name == "StatementModifier").count();
        assert_eq!(statement_modifier_count, 2, "expected nested statement modifiers");
        let assignment_count =
            kind_names.iter().filter(|kind_name| **kind_name == "Assignment").count();
        assert_eq!(assignment_count, 2, "expected both modifier conditions to be traversed");
        assert!(
            kind_names.iter().any(|kind_name| *kind_name == "FunctionCall"),
            "expected statement subtree under nested modifiers to be traversed"
        );
    }

    #[test]
    fn walker_traversal_conforms_for_all_child_bearing_node_kinds() {
        let proto = Node::new(NodeKind::Prototype { content: "$".to_string() }, loc());
        let signature = Node::new(
            NodeKind::Signature {
                parameters: vec![Node::new(
                    NodeKind::MandatoryParameter { variable: Box::new(variable("arg")) },
                    loc(),
                )],
            },
            loc(),
        );
        let child_bearing_roots = vec![
            Node::new(
                NodeKind::Tie {
                    variable: Box::new(variable("x")),
                    package: Box::new(number("1")),
                    args: vec![number("2")],
                },
                loc(),
            ),
            Node::new(NodeKind::Untie { variable: Box::new(variable("x")) }, loc()),
            Node::new(NodeKind::Program { statements: vec![number("1")] }, loc()),
            Node::new(NodeKind::ExpressionStatement { expression: Box::new(number("1")) }, loc()),
            Node::new(
                NodeKind::VariableDeclaration {
                    declarator: "my".to_string(),
                    variable: Box::new(variable("x")),
                    attributes: vec![],
                    initializer: Some(Box::new(number("1"))),
                },
                loc(),
            ),
            Node::new(
                NodeKind::VariableListDeclaration {
                    declarator: "my".to_string(),
                    variables: vec![variable("x"), variable("y")],
                    attributes: vec![],
                    initializer: Some(Box::new(number("1"))),
                },
                loc(),
            ),
            Node::new(
                NodeKind::VariableWithAttributes {
                    variable: Box::new(variable("x")),
                    attributes: vec!["ro".to_string()],
                },
                loc(),
            ),
            Node::new(
                NodeKind::Binary {
                    op: "+".to_string(),
                    left: Box::new(number("1")),
                    right: Box::new(number("2")),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Ternary {
                    condition: Box::new(number("1")),
                    then_expr: Box::new(number("2")),
                    else_expr: Box::new(number("3")),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Unary { op: "!".to_string(), operand: Box::new(number("1")) },
                loc(),
            ),
            Node::new(
                NodeKind::Assignment {
                    lhs: Box::new(variable("x")),
                    rhs: Box::new(number("1")),
                    op: "=".to_string(),
                },
                loc(),
            ),
            block(),
            Node::new(
                NodeKind::If {
                    condition: Box::new(number("1")),
                    then_branch: Box::new(block()),
                    elsif_branches: vec![(Box::new(number("2")), Box::new(block()))],
                    else_branch: Some(Box::new(block())),
                },
                loc(),
            ),
            Node::new(
                NodeKind::While {
                    condition: Box::new(number("1")),
                    body: Box::new(block()),
                    continue_block: Some(Box::new(block())),
                },
                loc(),
            ),
            Node::new(
                NodeKind::For {
                    init: Some(Box::new(number("0"))),
                    condition: Some(Box::new(number("1"))),
                    update: Some(Box::new(number("2"))),
                    body: Box::new(block()),
                    continue_block: Some(Box::new(block())),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Foreach {
                    variable: Box::new(variable("item")),
                    list: Box::new(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![number("1")] },
                        loc(),
                    )),
                    body: Box::new(block()),
                    continue_block: Some(Box::new(block())),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Given { expr: Box::new(number("1")), body: Box::new(block()) },
                loc(),
            ),
            Node::new(
                NodeKind::When { condition: Box::new(number("1")), body: Box::new(block()) },
                loc(),
            ),
            Node::new(NodeKind::Default { body: Box::new(block()) }, loc()),
            Node::new(
                NodeKind::StatementModifier {
                    statement: Box::new(Node::new(
                        NodeKind::ExpressionStatement {
                            expression: Box::new(Node::new(
                                NodeKind::FunctionCall {
                                    name: "print".to_string(),
                                    args: vec![number("1")],
                                },
                                loc(),
                            )),
                        },
                        loc(),
                    )),
                    modifier: "if".to_string(),
                    condition: Box::new(number("1")),
                },
                loc(),
            ),
            Node::new(
                NodeKind::LabeledStatement {
                    label: "LBL".to_string(),
                    statement: Box::new(block()),
                },
                loc(),
            ),
            Node::new(NodeKind::Eval { block: Box::new(block()) }, loc()),
            Node::new(NodeKind::Do { block: Box::new(block()) }, loc()),
            Node::new(NodeKind::Defer { block: Box::new(block()) }, loc()),
            Node::new(
                NodeKind::Try {
                    body: Box::new(block()),
                    catch_blocks: vec![(Some("$e".to_string()), Box::new(block()))],
                    finally_block: Some(Box::new(block())),
                },
                loc(),
            ),
            Node::new(
                NodeKind::FunctionCall { name: "f".to_string(), args: vec![number("1")] },
                loc(),
            ),
            Node::new(
                NodeKind::MethodCall {
                    object: Box::new(variable("obj")),
                    method: "run".to_string(),
                    args: vec![number("1")],
                },
                loc(),
            ),
            Node::new(
                NodeKind::IndirectCall {
                    method: "run".to_string(),
                    object: Box::new(variable("obj")),
                    args: vec![number("1")],
                },
                loc(),
            ),
            Node::new(
                NodeKind::Subroutine {
                    name: Some("demo".to_string()),
                    name_span: None,
                    prototype: Some(Box::new(proto)),
                    signature: Some(Box::new(signature)),
                    attributes: vec![],
                    body: Box::new(block()),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Method {
                    name: "m".to_string(),
                    signature: Some(Box::new(Node::new(
                        NodeKind::Signature { parameters: vec![] },
                        loc(),
                    ))),
                    attributes: vec![],
                    body: Box::new(block()),
                },
                loc(),
            ),
            Node::new(NodeKind::Return { value: Some(Box::new(number("1"))) }, loc()),
            Node::new(NodeKind::Goto { target: Box::new(number("1")) }, loc()),
            Node::new(
                NodeKind::Signature {
                    parameters: vec![Node::new(
                        NodeKind::NamedParameter { variable: Box::new(variable("name")) },
                        loc(),
                    )],
                },
                loc(),
            ),
            Node::new(NodeKind::MandatoryParameter { variable: Box::new(variable("m")) }, loc()),
            Node::new(
                NodeKind::OptionalParameter {
                    variable: Box::new(variable("o")),
                    default_value: Box::new(number("1")),
                },
                loc(),
            ),
            Node::new(NodeKind::SlurpyParameter { variable: Box::new(variable("s")) }, loc()),
            Node::new(NodeKind::NamedParameter { variable: Box::new(variable("n")) }, loc()),
            Node::new(
                NodeKind::Match {
                    expr: Box::new(variable("x")),
                    pattern: "a".to_string(),
                    modifiers: String::new(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc(),
            ),
            Node::new(
                NodeKind::Substitution {
                    expr: Box::new(variable("x")),
                    pattern: "a".to_string(),
                    replacement: "b".to_string(),
                    modifiers: String::new(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc(),
            ),
            Node::new(
                NodeKind::Transliteration {
                    expr: Box::new(variable("x")),
                    search: "a".to_string(),
                    replace: "b".to_string(),
                    modifiers: String::new(),
                    negated: false,
                },
                loc(),
            ),
            Node::new(NodeKind::ArrayLiteral { elements: vec![number("1")] }, loc()),
            Node::new(NodeKind::HashLiteral { pairs: vec![(number("1"), number("2"))] }, loc()),
            Node::new(
                NodeKind::Package {
                    name: "Pkg".to_string(),
                    name_span: loc(),
                    block: Some(Box::new(block())),
                },
                loc(),
            ),
            Node::new(
                NodeKind::PhaseBlock {
                    phase: "BEGIN".to_string(),
                    phase_span: None,
                    block: Box::new(block()),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Class { name: "C".to_string(), parents: vec![], body: Box::new(block()) },
                loc(),
            ),
            Node::new(
                NodeKind::Error {
                    message: "x".to_string(),
                    expected: vec![],
                    found: None,
                    partial: Some(Box::new(number("1"))),
                },
                loc(),
            ),
        ];

        for root in child_bearing_roots {
            let mut expected = Vec::new();
            fn walk_expected<'a>(node: &'a Node, out: &mut Vec<&'a str>) {
                out.push(node.kind.kind_name());
                node.for_each_child(|child| walk_expected(child, out));
            }
            walk_expected(&root, &mut expected);

            let mut actual = Vec::new();
            walk_node(&root, &mut |node| actual.push(node.kind.kind_name()));

            assert_eq!(
                actual,
                expected,
                "diagnostics walker diverged from AST traversal contract at {}",
                root.kind.kind_name(),
            );
        }
    }
}
