//! BDD-style parser acceptance coverage.
//!
//! These scenarios encode parser expectations in a Given/When/Then shape so
//! regressions read like behavior changes instead of implementation details.

use perl_parser::{Node, NodeKind, ParseError, Parser};

struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, text: &str) {
        eprintln!("[{}] Given {text}", self.name);
    }

    fn when(&self, text: &str) {
        eprintln!("[{}] When {text}", self.name);
    }

    fn then(&self, text: &str) {
        eprintln!("[{}] Then {text}", self.name);
    }
}

fn parse(source: &str) -> Result<Node, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse()
}

fn find_nodes(node: &Node, matches: impl Fn(&NodeKind) -> bool + Copy) -> Vec<&Node> {
    let mut found = Vec::new();
    find_nodes_recursive(node, matches, &mut found);
    found
}

fn find_nodes_recursive<'a>(
    node: &'a Node,
    matches: impl Fn(&NodeKind) -> bool + Copy,
    found: &mut Vec<&'a Node>,
) {
    if matches(&node.kind) {
        found.push(node);
    }

    for child in node.children() {
        find_nodes_recursive(child, matches, found);
    }
}

#[test]
fn given_variable_declaration_when_parsing_then_declaration_node_is_present()
-> Result<(), ParseError> {
    let scenario = BddScenario::new("variable declaration survives parse");
    scenario.given("a lexical declaration and a use site");

    let source = "my $name = 'perl';\nprint $name;";

    scenario.when("the parser builds an AST");
    let ast = parse(source)?;

    scenario.then("the AST contains a variable declaration node");
    let declarations =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::VariableDeclaration { .. }));
    assert_eq!(declarations.len(), 1);

    Ok(())
}

#[test]
fn given_nested_control_flow_when_parsing_then_if_and_loop_nodes_are_retained()
-> Result<(), ParseError> {
    let scenario = BddScenario::new("nested control flow remains structurally visible");
    scenario.given("an if-block that contains a while loop");

    let source = "if ($ready) { while ($count > 0) { $count--; } }";

    scenario.when("the parser processes the source");
    let ast = parse(source)?;

    scenario.then("both If and While nodes are available for downstream analysis");
    let if_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::If { .. }));
    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));

    assert_eq!(if_nodes.len(), 1);
    assert_eq!(while_nodes.len(), 1);

    Ok(())
}

#[test]
fn given_invalid_statement_when_parsing_then_error_node_is_emitted() -> Result<(), ParseError> {
    let scenario = BddScenario::new("syntax error is represented as an error node");
    scenario.given("an assignment with a missing right-hand expression");

    let source = "my $x = ;\nprint $x;";

    scenario.when("the parser attempts recovery");
    let ast = parse(source)?;

    scenario.then("an Error or MissingExpression node is present for diagnostics");
    let error_nodes = find_nodes(&ast, |kind| {
        matches!(
            kind,
            NodeKind::Error { .. } | NodeKind::MissingExpression | NodeKind::MissingStatement
        )
    });

    assert!(!error_nodes.is_empty());

    Ok(())
}

#[test]
fn given_package_and_subroutine_when_parsing_then_named_subroutine_is_discoverable()
-> Result<(), ParseError> {
    let scenario = BddScenario::new("package + sub declaration parse contract");
    scenario.given("a package containing a named subroutine");

    let source = "package Demo;\nsub greet { return 'hi'; }\n";

    scenario.when("the parser constructs the syntax tree");
    let ast = parse(source)?;

    scenario.then("the AST exposes a Subroutine node for symbol tooling");
    let named_sub_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::Subroutine { .. }));

    assert_eq!(named_sub_nodes.len(), 1);

    Ok(())
}
