//! AST walker for diagnostics
//!
//! This module provides a generic AST walker function for traversing
//! Perl AST nodes and applying diagnostic checks.

use perl_parser_core::ast::Node;

/// Walk the AST and call a function for each node
///
/// This function recursively walks the AST and calls the provided function
/// for each node. The function is called before visiting children (pre-order).
#[allow(clippy::only_used_in_recursion)]
pub fn walk_node<F>(node: &Node, func: &mut F)
where
    F: FnMut(&Node),
{
    func(node);
    node.for_each_child(|child| walk_node(child, func));
}

#[cfg(test)]
mod tests {
    use super::walk_node;
    use perl_parser_core::ast::{Node, NodeKind};
    use perl_parser_core::{Parser, SourceLocation};
    use perl_tdd_support::must;

    fn count_with_walker(node: &Node) -> usize {
        let mut count = 0usize;
        walk_node(node, &mut |_| count += 1);
        count
    }

    #[test]
    fn statement_modifier_if_visits_statement_and_condition() {
        let ast = must(Parser::new("print \"ok\" if $x = 5;").parse());
        let mut saw_print_statement = false;
        let mut saw_assignment_condition = false;
        walk_node(&ast, &mut |node| match &node.kind {
            NodeKind::FunctionCall { name, .. } if name == "print" => saw_print_statement = true,
            NodeKind::Assignment { .. } => saw_assignment_condition = true,
            _ => {}
        });
        assert!(saw_print_statement);
        assert!(saw_assignment_condition);
    }

    #[test]
    fn nested_statement_modifiers_are_traversed() {
        let loc = SourceLocation { start: 0, end: 1 };
        let ast = Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(Node::new(
                    NodeKind::StatementModifier {
                        statement: Box::new(Node::new(
                            NodeKind::ExpressionStatement {
                                expression: Box::new(Node::new(
                                    NodeKind::FunctionCall {
                                        name: "print".to_string(),
                                        args: vec![Node::new(
                                            NodeKind::Variable {
                                                sigil: "$".to_string(),
                                                name: "x".to_string(),
                                            },
                                            loc,
                                        )],
                                    },
                                    loc,
                                )),
                            },
                            loc,
                        )),
                        modifier: "if".to_string(),
                        condition: Box::new(Node::new(
                            NodeKind::Variable { sigil: "$".to_string(), name: "y".to_string() },
                            loc,
                        )),
                    },
                    loc,
                )),
                modifier: "while".to_string(),
                condition: Box::new(Node::new(
                    NodeKind::Variable { sigil: "$".to_string(), name: "z".to_string() },
                    loc,
                )),
            },
            loc,
        );
        let mut modifiers = 0usize;
        walk_node(&ast, &mut |node| {
            if matches!(&node.kind, NodeKind::StatementModifier { .. }) {
                modifiers += 1;
            }
        });
        assert_eq!(modifiers, 2);
    }

    #[test]
    fn all_statement_modifier_keywords_traverse() {
        for keyword in ["if", "unless", "while", "until", "for", "foreach"] {
            let source = format!("print $x {keyword} $y;");
            let ast = must(Parser::new(&source).parse());
            let mut saw_statement = false;
            let mut saw_condition = false;
            walk_node(&ast, &mut |node| match &node.kind {
                NodeKind::FunctionCall { name, .. } if name == "print" => saw_statement = true,
                NodeKind::Variable { name, .. } if name == "y" => saw_condition = true,
                _ => {}
            });
            assert!(saw_statement, "missing statement visit for {keyword}");
            assert!(saw_condition, "missing condition visit for {keyword}");
        }
    }

    #[test]
    fn traversal_conforms_to_children_contract_for_child_bearing_node_kinds() {
        let loc = SourceLocation { start: 0, end: 1 };
        let leaf = || Node::new(NodeKind::Number { value: "1".to_string() }, loc);
        let block = || Node::new(NodeKind::Block { statements: vec![leaf()] }, loc);

        let samples = vec![
            Node::new(NodeKind::Program { statements: vec![leaf()] }, loc),
            Node::new(NodeKind::ExpressionStatement { expression: Box::new(leaf()) }, loc),
            Node::new(
                NodeKind::VariableDeclaration {
                    declarator: "my".to_string(),
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                        loc,
                    )),
                    attributes: vec![],
                    initializer: Some(Box::new(leaf())),
                },
                loc,
            ),
            Node::new(
                NodeKind::VariableListDeclaration {
                    declarator: "my".to_string(),
                    variables: vec![Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                        loc,
                    )],
                    attributes: vec![],
                    initializer: Some(Box::new(leaf())),
                },
                loc,
            ),
            Node::new(
                NodeKind::VariableWithAttributes {
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                        loc,
                    )),
                    attributes: vec!["readonly".to_string()],
                },
                loc,
            ),
            Node::new(
                NodeKind::Assignment {
                    lhs: Box::new(leaf()),
                    rhs: Box::new(leaf()),
                    op: "=".to_string(),
                },
                loc,
            ),
            Node::new(
                NodeKind::Binary {
                    op: "+".to_string(),
                    left: Box::new(leaf()),
                    right: Box::new(leaf()),
                },
                loc,
            ),
            Node::new(
                NodeKind::Ternary {
                    condition: Box::new(leaf()),
                    then_expr: Box::new(leaf()),
                    else_expr: Box::new(leaf()),
                },
                loc,
            ),
            Node::new(NodeKind::Unary { op: "-".to_string(), operand: Box::new(leaf()) }, loc),
            Node::new(NodeKind::ArrayLiteral { elements: vec![leaf()] }, loc),
            Node::new(NodeKind::HashLiteral { pairs: vec![(leaf(), leaf())] }, loc),
            Node::new(NodeKind::Block { statements: vec![leaf()] }, loc),
            Node::new(NodeKind::Eval { block: Box::new(block()) }, loc),
            Node::new(NodeKind::Do { block: Box::new(block()) }, loc),
            Node::new(NodeKind::Defer { block: Box::new(block()) }, loc),
            Node::new(
                NodeKind::Try {
                    body: Box::new(block()),
                    catch_blocks: vec![(Some("$e".to_string()), Box::new(block()))],
                    finally_block: Some(Box::new(block())),
                },
                loc,
            ),
            Node::new(
                NodeKind::If {
                    condition: Box::new(leaf()),
                    then_branch: Box::new(block()),
                    elsif_branches: vec![(Box::new(leaf()), Box::new(block()))],
                    else_branch: Some(Box::new(block())),
                },
                loc,
            ),
            Node::new(
                NodeKind::While {
                    condition: Box::new(leaf()),
                    body: Box::new(block()),
                    continue_block: Some(Box::new(block())),
                },
                loc,
            ),
            Node::new(
                NodeKind::For {
                    init: Some(Box::new(leaf())),
                    condition: Some(Box::new(leaf())),
                    update: Some(Box::new(leaf())),
                    body: Box::new(block()),
                    continue_block: Some(Box::new(block())),
                },
                loc,
            ),
            Node::new(
                NodeKind::Foreach {
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                        loc,
                    )),
                    list: Box::new(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![leaf()] },
                        loc,
                    )),
                    body: Box::new(block()),
                    continue_block: Some(Box::new(block())),
                },
                loc,
            ),
            Node::new(NodeKind::Given { expr: Box::new(leaf()), body: Box::new(block()) }, loc),
            Node::new(NodeKind::When { condition: Box::new(leaf()), body: Box::new(block()) }, loc),
            Node::new(NodeKind::Default { body: Box::new(block()) }, loc),
            Node::new(
                NodeKind::StatementModifier {
                    statement: Box::new(Node::new(
                        NodeKind::ExpressionStatement { expression: Box::new(leaf()) },
                        loc,
                    )),
                    modifier: "if".to_string(),
                    condition: Box::new(leaf()),
                },
                loc,
            ),
            Node::new(
                NodeKind::Subroutine {
                    name: Some("f".to_string()),
                    name_span: Some(loc),
                    prototype: Some(Box::new(Node::new(
                        NodeKind::Prototype { content: "$".to_string() },
                        loc,
                    ))),
                    signature: Some(Box::new(Node::new(
                        NodeKind::Signature {
                            parameters: vec![Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "x".to_string(),
                                        },
                                        loc,
                                    )),
                                },
                                loc,
                            )],
                        },
                        loc,
                    ))),
                    attributes: vec![],
                    body: Box::new(block()),
                },
                loc,
            ),
            Node::new(NodeKind::Signature { parameters: vec![leaf()] }, loc),
            Node::new(NodeKind::MandatoryParameter { variable: Box::new(leaf()) }, loc),
            Node::new(
                NodeKind::OptionalParameter {
                    variable: Box::new(leaf()),
                    default_value: Box::new(leaf()),
                },
                loc,
            ),
            Node::new(NodeKind::SlurpyParameter { variable: Box::new(leaf()) }, loc),
            Node::new(NodeKind::NamedParameter { variable: Box::new(leaf()) }, loc),
            Node::new(
                NodeKind::Method {
                    name: "m".to_string(),
                    signature: Some(Box::new(Node::new(
                        NodeKind::Signature { parameters: vec![leaf()] },
                        loc,
                    ))),
                    attributes: vec![],
                    body: Box::new(block()),
                },
                loc,
            ),
            Node::new(NodeKind::FunctionCall { name: "f".to_string(), args: vec![leaf()] }, loc),
            Node::new(
                NodeKind::MethodCall {
                    object: Box::new(leaf()),
                    method: "m".to_string(),
                    args: vec![leaf()],
                },
                loc,
            ),
            Node::new(
                NodeKind::IndirectCall {
                    object: Box::new(leaf()),
                    method: "m".to_string(),
                    args: vec![leaf()],
                },
                loc,
            ),
            Node::new(NodeKind::Return { value: Some(Box::new(leaf())) }, loc),
            Node::new(NodeKind::Goto { target: Box::new(leaf()) }, loc),
            Node::new(
                NodeKind::Match {
                    expr: Box::new(leaf()),
                    pattern: "x".to_string(),
                    modifiers: String::new(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc,
            ),
            Node::new(
                NodeKind::Substitution {
                    expr: Box::new(leaf()),
                    pattern: "x".to_string(),
                    replacement: "y".to_string(),
                    modifiers: String::new(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc,
            ),
            Node::new(
                NodeKind::Transliteration {
                    expr: Box::new(leaf()),
                    search: "a".to_string(),
                    replace: "b".to_string(),
                    modifiers: String::new(),
                    negated: false,
                },
                loc,
            ),
            Node::new(
                NodeKind::Tie {
                    variable: Box::new(leaf()),
                    package: Box::new(leaf()),
                    args: vec![leaf()],
                },
                loc,
            ),
            Node::new(NodeKind::Untie { variable: Box::new(leaf()) }, loc),
            Node::new(
                NodeKind::LabeledStatement { label: "L".to_string(), statement: Box::new(block()) },
                loc,
            ),
            Node::new(
                NodeKind::Package {
                    name: "P".to_string(),
                    name_span: loc,
                    block: Some(Box::new(block())),
                },
                loc,
            ),
            Node::new(
                NodeKind::PhaseBlock {
                    phase: "BEGIN".to_string(),
                    phase_span: Some(loc),
                    block: Box::new(block()),
                },
                loc,
            ),
            Node::new(
                NodeKind::Class { name: "C".to_string(), parents: vec![], body: Box::new(block()) },
                loc,
            ),
            Node::new(
                NodeKind::Error {
                    message: "err".to_string(),
                    expected: vec![],
                    found: None,
                    partial: Some(Box::new(leaf())),
                },
                loc,
            ),
        ];

        for sample in samples {
            assert_eq!(count_with_walker(&sample), sample.count_nodes());
        }
    }
}
