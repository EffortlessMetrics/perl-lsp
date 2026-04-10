//! BDD-style tests for core `perl-ast` workflows.
//!
//! These scenarios cover realistic AST authoring flows using Given/When/Then
//! structure so failures are easier to interpret from a user-facing perspective.

use perl_ast::ast::{Node, NodeKind, SourceLocation};

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn num(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn var(name: &str) -> Node {
    Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() },
        loc(0, name.len() + 1),
    )
}

#[test]
fn bdd_build_assignment_program_and_render_sexp() {
    let scenario = Scenario::new("author expression statement");

    scenario.given("a variable node and a numeric literal");
    let lhs = var("count");
    let rhs = num("42");

    scenario.when("the nodes are assembled into an assignment expression statement");
    let assignment = Node::new(
        NodeKind::Assignment { lhs: Box::new(lhs), rhs: Box::new(rhs), op: "=".to_string() },
        loc(0, 11),
    );
    let program = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::ExpressionStatement { expression: Box::new(assignment) },
                loc(0, 11),
            )],
        },
        loc(0, 11),
    );

    scenario.then("to_sexp emits a source_file S-expression containing assignment details");
    let sexp = program.to_sexp();
    assert!(sexp.starts_with("(source_file"), "sexp: {sexp}");
    assert!(sexp.contains("assignment_assign"), "sexp: {sexp}");
    assert!(sexp.contains("(variable $ count)"), "sexp: {sexp}");
    assert!(sexp.contains("(number 42)"), "sexp: {sexp}");
}

#[test]
fn bdd_traverse_control_flow_children_in_order() {
    let scenario = Scenario::new("traverse if node children");

    scenario.given("an if node with elsif and else branches");
    let node = Node::new(
        NodeKind::If {
            condition: Box::new(var("flag")),
            then_branch: Box::new(Node::new(
                NodeKind::Block { statements: vec![num("1")] },
                loc(0, 3),
            )),
            elsif_branches: vec![(
                Box::new(var("fallback")),
                Box::new(Node::new(NodeKind::Block { statements: vec![num("2")] }, loc(0, 3))),
            )],
            else_branch: Some(Box::new(Node::new(
                NodeKind::Block { statements: vec![num("3")] },
                loc(0, 3),
            ))),
        },
        loc(0, 24),
    );

    scenario.when("for_each_child walks the direct children");
    let mut kinds = Vec::new();
    node.for_each_child(|child| kinds.push(child.kind.kind_name().to_string()));

    scenario.then("the traversal visits condition, then, elsif condition/body, and else");
    assert_eq!(kinds, vec!["Variable", "Block", "Variable", "Block", "Block"]);
}

#[test]
fn bdd_mutate_tree_using_for_each_child_mut() {
    let scenario = Scenario::new("rewrite numeric literals");

    scenario.given("a program with two numeric literals");
    let mut node = Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::ExpressionStatement { expression: Box::new(num("1")) },
                    loc(0, 1),
                ),
                Node::new(
                    NodeKind::ExpressionStatement { expression: Box::new(num("2")) },
                    loc(2, 3),
                ),
            ],
        },
        loc(0, 3),
    );

    scenario.when("a mutable traversal rewrites Number values");
    node.for_each_child_mut(|child| {
        if let NodeKind::ExpressionStatement { expression } = &mut child.kind
            && let NodeKind::Number { value } = &mut expression.kind
        {
            value.push('0');
        }
    });

    scenario.then("the tree reflects both updated literals");
    let rendered = node.to_sexp();
    assert!(rendered.contains("(number 10)"), "sexp: {rendered}");
    assert!(rendered.contains("(number 20)"), "sexp: {rendered}");
}
