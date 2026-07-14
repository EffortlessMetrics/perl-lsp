//! Perl 5.44 named parameters (PPC0024): the `NamedParameter` AST node must
//! carry the external argument name, the default operator/value, and whether
//! the parameter is required — not just the bound variable.
//!
//! Before this enrichment the parser discarded the default of `:$x = 1`
//! entirely and exposed only the variable, so named-argument semantics could
//! not be modeled downstream (signature help, completion, diagnostics).

use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::{must, must_some};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Find the first `NamedParameter` node in the tree, if any.
fn find_named_parameter(node: &Node) -> Option<&NodeKind> {
    if matches!(node.kind, NodeKind::NamedParameter { .. }) {
        return Some(&node.kind);
    }
    let mut found = None;
    node.for_each_child(|child| {
        if found.is_none() {
            found = find_named_parameter(child);
        }
    });
    found
}

fn parse(source: &str) -> Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

#[test]
fn required_named_parameter_has_no_default_and_is_required() -> TestResult {
    let ast = parse("sub f (:$alpha) { }");
    let kind = must_some(find_named_parameter(&ast));

    let NodeKind::NamedParameter {
        external_name, default_operator, default_value, required, ..
    } = kind
    else {
        return Err("expected a NamedParameter for :$alpha".into());
    };

    assert_eq!(external_name, "alpha", "external name is the var name without sigil");
    assert!(default_operator.is_none(), "no default operator when undefaulted");
    assert!(default_value.is_none(), "no default value when undefaulted");
    assert!(*required, ":$alpha with no default is required");
    Ok(())
}

#[test]
fn defaulted_named_parameter_preserves_its_default() -> TestResult {
    // Regression: the `= 1` default used to be parsed and then thrown away.
    let ast = parse("sub f (:$beta = 1) { }");
    let kind = must_some(find_named_parameter(&ast));

    let NodeKind::NamedParameter {
        external_name, default_operator, default_value, required, ..
    } = kind
    else {
        return Err("expected a NamedParameter for :$beta".into());
    };

    assert_eq!(external_name, "beta");
    assert_eq!(default_operator.as_deref(), Some("="), "records the `=` default operator");
    assert!(default_value.is_some(), "the default expression must be preserved");
    assert!(!*required, ":$beta with a default is optional");
    Ok(())
}

#[test]
fn named_parameters_coexist_with_positional_and_slurpy() {
    let ast = parse("sub f ($x, $y = 0, :$alpha, :$beta = 2, %rest) { }");

    let mut count = 0;
    fn walk(node: &Node, count: &mut usize) {
        if matches!(node.kind, NodeKind::NamedParameter { .. }) {
            *count += 1;
        }
        node.for_each_child(|c| walk(c, count));
    }
    walk(&ast, &mut count);

    assert_eq!(count, 2, "both :$alpha and :$beta surface as NamedParameter nodes");
}
