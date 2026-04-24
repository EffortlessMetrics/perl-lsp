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
    use perl_parser_core::SourceLocation;
    use perl_parser_core::ast::{Node, NodeKind};

    use super::walk_node;

    fn loc() -> SourceLocation {
        SourceLocation { start: 0, end: 1 }
    }

    fn id(name: &str) -> Node {
        Node::new(NodeKind::Identifier { name: name.to_string() }, loc())
    }

    fn number(value: &str) -> Node {
        Node::new(NodeKind::Number { value: value.to_string() }, loc())
    }

    fn stmt_mod(modifier: &str, statement: Node, condition: Node) -> Node {
        Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(statement),
                modifier: modifier.to_string(),
                condition: Box::new(condition),
            },
            loc(),
        )
    }

    fn canonical_count(node: &Node) -> usize {
        1 + node.children().into_iter().map(canonical_count).sum::<usize>()
    }

    #[test]
    fn walks_statement_modifier_statement_and_condition() {
        let ast = stmt_mod(
            "if",
            Node::new(
                NodeKind::ExpressionStatement {
                    expression: Box::new(Node::new(
                        NodeKind::FunctionCall { name: "print".to_string(), args: vec![id("ok")] },
                        loc(),
                    )),
                },
                loc(),
            ),
            Node::new(
                NodeKind::ExpressionStatement {
                    expression: Box::new(Node::new(
                        NodeKind::Assignment {
                            lhs: Box::new(Node::new(
                                NodeKind::Variable {
                                    sigil: "$".to_string(),
                                    name: "x".to_string(),
                                },
                                loc(),
                            )),
                            op: "=".to_string(),
                            rhs: Box::new(number("5")),
                        },
                        loc(),
                    )),
                },
                loc(),
            ),
        );

        let mut visited_kinds = Vec::new();
        walk_node(&ast, &mut |node| visited_kinds.push(node.kind.kind_name().to_string()));

        assert!(visited_kinds.iter().any(|kind| kind == "FunctionCall"));
        assert!(visited_kinds.iter().any(|kind| kind == "Assignment"));
    }

    #[test]
    fn walks_nested_statement_modifiers_for_all_supported_modifiers() {
        let modifiers = ["if", "unless", "while", "until", "for", "foreach"];
        let mut current = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::FunctionCall { name: "print".to_string(), args: vec![id("ok")] },
                    loc(),
                )),
            },
            loc(),
        );

        for modifier in modifiers {
            current = stmt_mod(modifier, current, id(modifier));
        }

        let expected = canonical_count(&current);
        let mut actual = 0usize;
        walk_node(&current, &mut |_| actual += 1);
        assert_eq!(actual, expected);
    }
}
