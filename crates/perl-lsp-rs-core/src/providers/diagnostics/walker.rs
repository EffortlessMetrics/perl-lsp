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
    use perl_parser_core::{Node, NodeKind, Parser, SourceLocation};

    fn loc(start: usize) -> SourceLocation {
        SourceLocation { start, end: start + 1 }
    }

    fn leaf_number(start: usize) -> Node {
        Node::new(NodeKind::Number { value: start.to_string() }, loc(start))
    }

    fn leaf_variable(start: usize, name: &str) -> Node {
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() }, loc(start))
    }

    fn child_bearing_samples() -> Vec<Node> {
        let body =
            Box::new(Node::new(NodeKind::Block { statements: vec![leaf_number(100)] }, loc(99)));
        let sig = Box::new(Node::new(
            NodeKind::Signature {
                parameters: vec![Node::new(
                    NodeKind::MandatoryParameter { variable: Box::new(leaf_variable(101, "arg")) },
                    loc(101),
                )],
            },
            loc(101),
        ));
        vec![
            Node::new(NodeKind::Program { statements: vec![leaf_number(1)] }, loc(0)),
            Node::new(
                NodeKind::ExpressionStatement { expression: Box::new(leaf_number(2)) },
                loc(2),
            ),
            Node::new(
                NodeKind::VariableDeclaration {
                    declarator: "my".to_string(),
                    variable: Box::new(leaf_variable(3, "v")),
                    attributes: Vec::new(),
                    initializer: Some(Box::new(leaf_number(4))),
                },
                loc(3),
            ),
            Node::new(
                NodeKind::VariableListDeclaration {
                    declarator: "my".to_string(),
                    variables: vec![leaf_variable(5, "a"), leaf_variable(6, "b")],
                    attributes: Vec::new(),
                    initializer: Some(Box::new(leaf_number(7))),
                },
                loc(5),
            ),
            Node::new(
                NodeKind::VariableWithAttributes {
                    variable: Box::new(leaf_variable(8, "attrs")),
                    attributes: vec!["shared".to_string()],
                },
                loc(8),
            ),
            Node::new(
                NodeKind::Assignment {
                    lhs: Box::new(leaf_variable(9, "lhs")),
                    rhs: Box::new(leaf_number(10)),
                    op: "=".to_string(),
                },
                loc(9),
            ),
            Node::new(
                NodeKind::Binary {
                    op: "+".to_string(),
                    left: Box::new(leaf_number(11)),
                    right: Box::new(leaf_number(12)),
                },
                loc(11),
            ),
            Node::new(
                NodeKind::Ternary {
                    condition: Box::new(leaf_number(13)),
                    then_expr: Box::new(leaf_number(14)),
                    else_expr: Box::new(leaf_number(15)),
                },
                loc(13),
            ),
            Node::new(
                NodeKind::Unary { op: "!".to_string(), operand: Box::new(leaf_number(16)) },
                loc(16),
            ),
            Node::new(NodeKind::ArrayLiteral { elements: vec![leaf_number(17)] }, loc(17)),
            Node::new(
                NodeKind::HashLiteral { pairs: vec![(leaf_number(18), leaf_number(19))] },
                loc(18),
            ),
            Node::new(NodeKind::Block { statements: vec![leaf_number(20)] }, loc(20)),
            Node::new(
                NodeKind::Eval {
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(21))),
                },
                loc(21),
            ),
            Node::new(
                NodeKind::Do {
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(22))),
                },
                loc(22),
            ),
            Node::new(
                NodeKind::Defer {
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(23))),
                },
                loc(23),
            ),
            Node::new(
                NodeKind::Try {
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(24))),
                    catch_blocks: vec![(
                        Some("$e".to_string()),
                        Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(25))),
                    )],
                    finally_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(26),
                    ))),
                },
                loc(24),
            ),
            Node::new(
                NodeKind::If {
                    keyword: None,
                    condition: Box::new(leaf_number(27)),
                    then_branch: Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(28),
                    )),
                    elsif_branches: vec![(
                        Box::new(leaf_number(29)),
                        Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(30))),
                    )],
                    else_branch: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(31),
                    ))),
                },
                loc(27),
            ),
            Node::new(
                NodeKind::LabeledStatement {
                    label: "LBL".to_string(),
                    statement: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(32))),
                },
                loc(32),
            ),
            Node::new(
                NodeKind::While {
                    keyword: None,
                    condition: Box::new(leaf_number(33)),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(34))),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(35),
                    ))),
                },
                loc(33),
            ),
            Node::new(
                NodeKind::Tie {
                    variable: Box::new(leaf_variable(36, "tied")),
                    package: Box::new(Node::new(
                        NodeKind::String { value: "Pkg".to_string(), interpolated: false },
                        loc(37),
                    )),
                    args: vec![leaf_number(38)],
                },
                loc(36),
            ),
            Node::new(NodeKind::Untie { variable: Box::new(leaf_variable(39, "tied")) }, loc(39)),
            Node::new(
                NodeKind::For {
                    init: Some(Box::new(leaf_number(40))),
                    condition: Some(Box::new(leaf_number(41))),
                    update: Some(Box::new(leaf_number(42))),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(43))),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(44),
                    ))),
                },
                loc(40),
            ),
            Node::new(
                NodeKind::Foreach {
                    variable: Box::new(leaf_variable(45, "it")),
                    list: Box::new(Node::new(
                        NodeKind::ArrayLiteral { elements: vec![leaf_number(46)] },
                        loc(46),
                    )),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(47))),
                    continue_block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(48),
                    ))),
                },
                loc(45),
            ),
            Node::new(
                NodeKind::Given {
                    expr: Box::new(leaf_number(49)),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(50))),
                },
                loc(49),
            ),
            Node::new(
                NodeKind::When {
                    condition: Box::new(leaf_number(51)),
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(52))),
                },
                loc(51),
            ),
            Node::new(
                NodeKind::Default {
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(53))),
                },
                loc(53),
            ),
            Node::new(
                NodeKind::StatementModifier {
                    statement: Box::new(Node::new(
                        NodeKind::ExpressionStatement {
                            expression: Box::new(Node::new(
                                NodeKind::FunctionCall {
                                    name: "print".to_string(),
                                    args: vec![Node::new(
                                        NodeKind::String {
                                            value: "ok".to_string(),
                                            interpolated: false,
                                        },
                                        loc(54),
                                    )],
                                },
                                loc(54),
                            )),
                        },
                        loc(54),
                    )),
                    modifier: "if".to_string(),
                    condition: Box::new(Node::new(
                        NodeKind::Assignment {
                            lhs: Box::new(leaf_variable(55, "x")),
                            rhs: Box::new(leaf_number(56)),
                            op: "=".to_string(),
                        },
                        loc(55),
                    )),
                },
                loc(54),
            ),
            Node::new(
                NodeKind::Subroutine {
                    name: Some("foo".to_string()),
                    name_span: Some(loc(57)),
                    prototype: Some(Box::new(Node::new(
                        NodeKind::Prototype { content: "$".to_string() },
                        loc(58),
                    ))),
                    signature: Some(sig.clone()),
                    attributes: Vec::new(),
                    body: body.clone(),
                },
                loc(57),
            ),
            Node::new(
                NodeKind::Signature {
                    parameters: vec![Node::new(
                        NodeKind::SlurpyParameter { variable: Box::new(leaf_variable(59, "rest")) },
                        loc(59),
                    )],
                },
                loc(59),
            ),
            Node::new(
                NodeKind::MandatoryParameter { variable: Box::new(leaf_variable(60, "req")) },
                loc(60),
            ),
            Node::new(
                NodeKind::OptionalParameter {
                    variable: Box::new(leaf_variable(61, "opt")),
                    default_value: Box::new(leaf_number(62)),
                },
                loc(61),
            ),
            Node::new(
                NodeKind::SlurpyParameter { variable: Box::new(leaf_variable(63, "slurp")) },
                loc(63),
            ),
            Node::new(
                NodeKind::NamedParameter { variable: Box::new(leaf_variable(64, "named")) },
                loc(64),
            ),
            Node::new(
                NodeKind::Method {
                    name: "bar".to_string(),
                    signature: Some(sig),
                    attributes: Vec::new(),
                    body,
                },
                loc(65),
            ),
            Node::new(NodeKind::Return { value: Some(Box::new(leaf_number(66))) }, loc(66)),
            Node::new(
                NodeKind::Goto {
                    target: Box::new(Node::new(
                        NodeKind::Identifier { name: "LBL".to_string() },
                        loc(67),
                    )),
                },
                loc(67),
            ),
            Node::new(
                NodeKind::MethodCall {
                    object: Box::new(Node::new(
                        NodeKind::Identifier { name: "obj".to_string() },
                        loc(68),
                    )),
                    method: "run".to_string(),
                    args: vec![leaf_number(69)],
                },
                loc(68),
            ),
            Node::new(
                NodeKind::FunctionCall { name: "f".to_string(), args: vec![leaf_number(70)] },
                loc(70),
            ),
            Node::new(
                NodeKind::IndirectCall {
                    method: "new".to_string(),
                    object: Box::new(Node::new(
                        NodeKind::Identifier { name: "Class".to_string() },
                        loc(71),
                    )),
                    args: vec![leaf_number(72)],
                },
                loc(71),
            ),
            Node::new(
                NodeKind::Match {
                    expr: Box::new(leaf_variable(73, "s")),
                    pattern: "x".to_string(),
                    modifiers: String::new(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc(73),
            ),
            Node::new(
                NodeKind::Substitution {
                    expr: Box::new(leaf_variable(74, "s")),
                    pattern: "x".to_string(),
                    replacement: "y".to_string(),
                    modifiers: String::new(),
                    has_embedded_code: false,
                    negated: false,
                },
                loc(74),
            ),
            Node::new(
                NodeKind::Transliteration {
                    expr: Box::new(leaf_variable(75, "s")),
                    search: "a".to_string(),
                    replace: "b".to_string(),
                    modifiers: String::new(),
                    negated: false,
                },
                loc(75),
            ),
            Node::new(
                NodeKind::Package {
                    name: "Pkg".to_string(),
                    name_span: loc(76),
                    block: Some(Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        loc(77),
                    ))),
                },
                loc(76),
            ),
            Node::new(
                NodeKind::PhaseBlock {
                    phase: "BEGIN".to_string(),
                    phase_span: Some(loc(78)),
                    block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(79))),
                },
                loc(78),
            ),
            Node::new(
                NodeKind::Class {
                    name: "C".to_string(),
                    parents: vec!["Base".to_string()],
                    body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(80))),
                },
                loc(80),
            ),
            Node::new(
                NodeKind::Error {
                    message: "oops".to_string(),
                    expected: Vec::new(),
                    found: None,
                    partial: Some(Box::new(leaf_number(81))),
                },
                loc(81),
            ),
        ]
    }

    #[test]
    fn walker_visits_every_child_bearing_kind_sample() {
        for sample in child_bearing_samples() {
            let mut visited = 0usize;
            walk_node(&sample, &mut |_| {
                visited += 1;
            });
            assert_eq!(visited, sample.count_nodes(), "{}", sample.kind.kind_name());
        }
    }

    #[test]
    fn statement_modifier_traversal_covers_statement_and_condition()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = Parser::new("print \"ok\" if $x = 5;");
        let ast = parser.parse()?;

        let mut saw_print = false;
        let mut saw_assignment = false;
        walk_node(&ast, &mut |node| match &node.kind {
            NodeKind::FunctionCall { name, .. } if name == "print" => saw_print = true,
            NodeKind::Assignment { .. } => saw_assignment = true,
            _ => {}
        });

        assert!(saw_print, "statement subtree should be visited");
        assert!(saw_assignment, "condition subtree should be visited");
        Ok(())
    }

    #[test]
    fn statement_modifier_traversal_handles_all_modifiers_and_nesting()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
print "ok" if $a;
print "ok" unless $b;
print "ok" while $c;
print "ok" until $d;
print "ok" for @xs;
print "ok" foreach @ys;
print "ok" if $x if $y;
"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let mut modifiers = Vec::new();
        walk_node(&ast, &mut |node| {
            if let NodeKind::StatementModifier { modifier, .. } = &node.kind {
                modifiers.push(modifier.clone());
            }
        });

        for expected in ["if", "unless", "while", "until", "for", "foreach"] {
            assert!(modifiers.iter().any(|modifier| modifier == expected), "missing {expected}");
        }
        assert!(
            modifiers.iter().filter(|modifier| modifier.as_str() == "if").count() >= 2,
            "nested modifiers should traverse both levels"
        );
        Ok(())
    }
}
