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

    for child in node.children() {
        walk_node(child, func);
    }
}

#[cfg(test)]
mod tests {
    use super::walk_node;
    use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
    use perl_parser_core::parser::Parser;

    fn loc() -> SourceLocation {
        SourceLocation { start: 0, end: 0 }
    }

    fn leaf(name: &str) -> Node {
        Node::new(NodeKind::Identifier { name: name.to_string() }, loc())
    }

    fn expr_stmt(name: &str) -> Node {
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(leaf(name)) }, loc())
    }

    fn collect_dfs_kind_names(node: &Node, out: &mut Vec<&'static str>) {
        out.push(node.kind.kind_name());
        for child in node.children() {
            collect_dfs_kind_names(child, out);
        }
    }

    #[test]
    fn walk_node_matches_canonical_children_traversal_for_child_bearing_nodes()
    -> Result<(), Box<dyn std::error::Error>> {
        let child_bearing_nodes = vec![
            Node::new(NodeKind::Program { statements: vec![expr_stmt("a")] }, loc()),
            Node::new(NodeKind::Block { statements: vec![expr_stmt("a")] }, loc()),
            Node::new(
                NodeKind::If {
                    condition: Box::new(leaf("cond")),
                    then_branch: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    elsif_branches: vec![(
                        Box::new(leaf("elsif")),
                        Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    )],
                    else_branch: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            Node::new(
                NodeKind::While {
                    condition: Box::new(leaf("cond")),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            Node::new(
                NodeKind::For {
                    init: Some(Box::new(leaf("init"))),
                    condition: Some(Box::new(leaf("cond"))),
                    update: Some(Box::new(leaf("update"))),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            Node::new(
                NodeKind::Foreach {
                    variable: Box::new(leaf("v")),
                    list: Box::new(leaf("items")),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc())),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(),
                    ))),
                },
                loc(),
            ),
            Node::new(
                NodeKind::StatementModifier {
                    statement: Box::new(expr_stmt("stmt")),
                    modifier: "if".to_string(),
                    condition: Box::new(leaf("cond")),
                },
                loc(),
            ),
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
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "x".to_string(),
                                        },
                                        loc(),
                                    )),
                                },
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
            Node::new(
                NodeKind::MethodCall {
                    object: Box::new(leaf("obj")),
                    method: "m".to_string(),
                    args: vec![leaf("a")],
                },
                loc(),
            ),
            Node::new(
                NodeKind::Error {
                    message: "oops".to_string(),
                    expected: vec![],
                    found: None,
                    partial: Some(Box::new(expr_stmt("partial"))),
                },
                loc(),
            ),
        ];

        for node in child_bearing_nodes {
            let mut via_walker = Vec::new();
            walk_node(&node, &mut |n| via_walker.push(n.kind.kind_name()));

            let mut via_children = Vec::new();
            collect_dfs_kind_names(&node, &mut via_children);

            assert_eq!(
                via_walker,
                via_children,
                "kind {} drifted from Node::children()",
                node.kind.kind_name()
            );
        }
        Ok(())
    }

    #[test]
    fn statement_modifier_walks_statement_and_condition_for_all_modifiers()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
            print "ok" if $x = 5;
            print "ok" unless $x = 5;
            print "ok" while $x = 5;
            print "ok" until $x = 5;
            print "ok" for $x = 5;
            print "ok" foreach $x = 5;
            print "ok" if $a if $b;
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let mut statement_modifiers = 0usize;
        let mut assignment_nodes = 0usize;
        let mut print_calls = 0usize;

        walk_node(&ast, &mut |node| match &node.kind {
            NodeKind::StatementModifier { .. } => statement_modifiers += 1,
            NodeKind::Assignment { .. } => assignment_nodes += 1,
            NodeKind::FunctionCall { name, .. } if name == "print" => print_calls += 1,
            _ => {}
        });

        assert!(
            statement_modifiers >= 7,
            "expected all six modifiers plus nested modifier to be visited"
        );
        assert!(
            assignment_nodes >= 6,
            "expected each postfix conditional/loop condition assignment to be visited"
        );
        assert!(print_calls >= 7, "expected print statements under modifiers to be visited");
        Ok(())
    }
}
