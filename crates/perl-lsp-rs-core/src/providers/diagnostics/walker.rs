//! AST walker for diagnostics
//!
//! This module provides a generic AST walker function for traversing
//! Perl AST nodes and applying diagnostic checks.

use perl_parser_core::ast::{Node, walk_preorder};

/// Walk the AST and call a function for each node.
///
/// This function recursively walks the AST and calls the provided function
/// for each node. The function is called before visiting children (pre-order).
pub fn walk_node<F>(node: &Node, func: &mut F)
where
    F: FnMut(&Node),
{
    walk_preorder(node, func);
}

#[cfg(test)]
mod tests {
    use super::walk_node;
    use perl_parser_core::{
        Parser,
        ast::{Node, SourceLocation},
    };
    use perl_tdd_support::must;

    #[test]
    fn statement_modifier_walks_statement_and_condition() {
        let ast = must(Parser::new("print \"ok\" if $x = 5;").parse());

        let mut saw_print = false;
        let mut saw_assignment = false;

        walk_node(&ast, &mut |node| match &node.kind {
            perl_parser_core::ast::NodeKind::FunctionCall { name, .. } if name == "print" => {
                saw_print = true;
            }
            perl_parser_core::ast::NodeKind::Assignment { .. } => {
                saw_assignment = true;
            }
            _ => {}
        });

        assert!(saw_print);
        assert!(saw_assignment);
    }

    #[test]
    fn nested_statement_modifiers_and_all_variants_are_walked() {
        let source = r#"
            print "x" if $a;
            print "x" unless $b;
            print "x" while $c;
            print "x" until $d;
            print "x" for @e;
            print "x" foreach @f;
            print "nested" if do { print "inner" unless $g; 1 };
        "#;
        let ast = must(Parser::new(source).parse());

        let mut seen_modifiers = std::collections::BTreeSet::new();
        let mut assignment_count = 0usize;
        walk_node(&ast, &mut |node| match &node.kind {
            perl_parser_core::ast::NodeKind::StatementModifier { modifier, .. } => {
                seen_modifiers.insert(modifier.clone());
            }
            perl_parser_core::ast::NodeKind::Assignment { .. } => assignment_count += 1,
            _ => {}
        });

        assert!(seen_modifiers.contains("if"));
        assert!(seen_modifiers.contains("unless"));
        assert!(seen_modifiers.contains("while"));
        assert!(seen_modifiers.contains("until"));
        assert!(seen_modifiers.contains("for"));
        assert!(seen_modifiers.contains("foreach"));
        assert_eq!(assignment_count, 0);
    }

    #[test]
    fn traversal_conformance_matches_ast_children_contract() {
        let ast = sample_tree();

        let mut walked = Vec::new();
        walk_node(&ast, &mut |node| walked.push(node.kind.kind_name().to_string()));

        let mut expected = Vec::new();
        walk_with_children_contract(&ast, &mut expected);

        assert_eq!(walked, expected);
    }

    fn walk_with_children_contract(node: &Node, out: &mut Vec<String>) {
        out.push(node.kind.kind_name().to_string());
        node.for_each_child(|child| walk_with_children_contract(child, out));
    }

    fn loc() -> SourceLocation {
        SourceLocation::new(0, 0)
    }

    fn leaf(name: &str) -> Node {
        Node::new(perl_parser_core::ast::NodeKind::Identifier { name: name.to_string() }, loc())
    }

    fn sample_tree() -> Node {
        use perl_parser_core::ast::NodeKind;

        Node::new(
            NodeKind::Program {
                statements: vec![
                    Node::new(
                        NodeKind::ExpressionStatement { expression: Box::new(leaf("expr")) },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::VariableDeclaration {
                            declarator: "my".to_string(),
                            variable: Box::new(leaf("var")),
                            attributes: vec![],
                            initializer: Some(Box::new(leaf("init"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::VariableListDeclaration {
                            declarator: "my".to_string(),
                            variables: vec![leaf("a"), leaf("b")],
                            attributes: vec![],
                            initializer: Some(Box::new(leaf("init"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Assignment {
                            lhs: Box::new(leaf("lhs")),
                            rhs: Box::new(leaf("rhs")),
                            op: "=".to_string(),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Binary {
                            op: "+".to_string(),
                            left: Box::new(leaf("left")),
                            right: Box::new(leaf("right")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Ternary {
                            condition: Box::new(leaf("cond")),
                            then_expr: Box::new(leaf("then")),
                            else_expr: Box::new(leaf("else")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Unary { op: "!".to_string(), operand: Box::new(leaf("operand")) },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::ArrayLiteral { elements: vec![leaf("e1"), leaf("e2")] },
                        loc(),
                    ),
                    Node::new(NodeKind::HashLiteral { pairs: vec![(leaf("k"), leaf("v"))] }, loc()),
                    Node::new(NodeKind::Block { statements: vec![leaf("stmt")] }, loc()),
                    Node::new(NodeKind::Eval { block: Box::new(leaf("blk")) }, loc()),
                    Node::new(NodeKind::Do { block: Box::new(leaf("blk")) }, loc()),
                    Node::new(NodeKind::Defer { block: Box::new(leaf("blk")) }, loc()),
                    Node::new(
                        NodeKind::Try {
                            body: Box::new(leaf("body")),
                            catch_blocks: vec![(Some("$err".to_string()), Box::new(leaf("catch")))],
                            finally_block: Some(Box::new(leaf("finally"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::If {
                            condition: Box::new(leaf("if_cond")),
                            then_branch: Box::new(leaf("if_then")),
                            elsif_branches: vec![(
                                Box::new(leaf("elsif_cond")),
                                Box::new(leaf("elsif_body")),
                            )],
                            else_branch: Some(Box::new(leaf("if_else"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::While {
                            condition: Box::new(leaf("while_cond")),
                            body: Box::new(leaf("while_body")),
                            continue_block: Some(Box::new(leaf("while_continue"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::For {
                            init: Some(Box::new(leaf("for_init"))),
                            condition: Some(Box::new(leaf("for_cond"))),
                            update: Some(Box::new(leaf("for_update"))),
                            body: Box::new(leaf("for_body")),
                            continue_block: Some(Box::new(leaf("for_continue"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Foreach {
                            variable: Box::new(leaf("for_var")),
                            list: Box::new(leaf("for_list")),
                            body: Box::new(leaf("for_body")),
                            continue_block: Some(Box::new(leaf("for_continue"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Given {
                            expr: Box::new(leaf("given_expr")),
                            body: Box::new(leaf("given_body")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::When {
                            condition: Box::new(leaf("when_cond")),
                            body: Box::new(leaf("when_body")),
                        },
                        loc(),
                    ),
                    Node::new(NodeKind::Default { body: Box::new(leaf("default_body")) }, loc()),
                    Node::new(
                        NodeKind::StatementModifier {
                            statement: Box::new(leaf("stmt")),
                            modifier: "if".to_string(),
                            condition: Box::new(leaf("cond")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::LabeledStatement {
                            label: "LBL".to_string(),
                            statement: Box::new(leaf("stmt")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Subroutine {
                            name: Some("sub".to_string()),
                            name_span: Some(loc()),
                            prototype: Some(Box::new(Node::new(
                                NodeKind::Prototype { content: "$".to_string() },
                                loc(),
                            ))),
                            signature: Some(Box::new(Node::new(
                                NodeKind::Signature {
                                    parameters: vec![
                                        Node::new(
                                            NodeKind::MandatoryParameter {
                                                variable: Box::new(leaf("mp")),
                                            },
                                            loc(),
                                        ),
                                        Node::new(
                                            NodeKind::OptionalParameter {
                                                variable: Box::new(leaf("op")),
                                                default_value: Box::new(leaf("op_def")),
                                            },
                                            loc(),
                                        ),
                                        Node::new(
                                            NodeKind::SlurpyParameter {
                                                variable: Box::new(leaf("sp")),
                                            },
                                            loc(),
                                        ),
                                        Node::new(
                                            NodeKind::NamedParameter {
                                                variable: Box::new(leaf("np")),
                                            },
                                            loc(),
                                        ),
                                    ],
                                },
                                loc(),
                            ))),
                            attributes: vec![],
                            body: Box::new(leaf("sub_body")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Method {
                            name: "method".to_string(),
                            signature: Some(Box::new(Node::new(
                                NodeKind::Signature { parameters: vec![] },
                                loc(),
                            ))),
                            attributes: vec![],
                            body: Box::new(leaf("method_body")),
                        },
                        loc(),
                    ),
                    Node::new(NodeKind::Return { value: Some(Box::new(leaf("ret"))) }, loc()),
                    Node::new(NodeKind::Goto { target: Box::new(leaf("target")) }, loc()),
                    Node::new(
                        NodeKind::FunctionCall { name: "f".to_string(), args: vec![leaf("arg")] },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::MethodCall {
                            object: Box::new(leaf("obj")),
                            method: "m".to_string(),
                            args: vec![leaf("arg")],
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::IndirectCall {
                            method: "new".to_string(),
                            object: Box::new(leaf("class")),
                            args: vec![leaf("arg")],
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::VariableWithAttributes {
                            variable: Box::new(leaf("v")),
                            attributes: vec![],
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Match {
                            expr: Box::new(leaf("expr")),
                            pattern: "x".to_string(),
                            modifiers: String::new(),
                            has_embedded_code: false,
                            negated: false,
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Substitution {
                            expr: Box::new(leaf("expr")),
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
                            expr: Box::new(leaf("expr")),
                            search: "a".to_string(),
                            replace: "b".to_string(),
                            modifiers: String::new(),
                            negated: false,
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Tie {
                            variable: Box::new(leaf("var")),
                            package: Box::new(leaf("pkg")),
                            args: vec![leaf("arg")],
                        },
                        loc(),
                    ),
                    Node::new(NodeKind::Untie { variable: Box::new(leaf("var")) }, loc()),
                    Node::new(
                        NodeKind::Package {
                            name: "Pkg".to_string(),
                            name_span: loc(),
                            block: Some(Box::new(leaf("pkg_body"))),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::PhaseBlock {
                            phase: "BEGIN".to_string(),
                            phase_span: Some(loc()),
                            block: Box::new(leaf("phase_body")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Class {
                            name: "C".to_string(),
                            parents: vec![],
                            body: Box::new(leaf("class_body")),
                        },
                        loc(),
                    ),
                    Node::new(
                        NodeKind::Error {
                            message: "err".to_string(),
                            partial: Some(Box::new(leaf("partial"))),
                            expected: vec![],
                            found: None,
                        },
                        loc(),
                    ),
                ],
            },
            loc(),
        )
    }
}
