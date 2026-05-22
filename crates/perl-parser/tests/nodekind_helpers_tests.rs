use std::collections::{BTreeMap, BTreeSet};

use perl_parser::ast::{Node, NodeKind, SourceLocation};

mod nodekind_helpers;
use nodekind_helpers::{
    ALL_NODE_KIND_NAMES, SYNTHETIC_NODE_KIND_NAMES, collect_node_kinds, collect_node_kinds_labeled,
    collect_node_kinds_with_parents, corpus_required_kinds, find_first_node_of_kind, has_node_kind,
};

fn loc() -> SourceLocation {
    SourceLocation::new(0, 0)
}

fn sample_ast() -> Node {
    Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::ExpressionStatement {
                    expression: Box::new(Node::new(
                        NodeKind::Binary {
                            op: "+".to_string(),
                            left: Box::new(Node::new(
                                NodeKind::Number { value: "1".to_string() },
                                loc(),
                            )),
                            right: Box::new(Node::new(
                                NodeKind::FunctionCall {
                                    name: "foo".to_string(),
                                    args: vec![Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "x".to_string(),
                                        },
                                        loc(),
                                    )],
                                },
                                loc(),
                            )),
                        },
                        loc(),
                    )),
                },
                loc(),
            )],
        },
        loc(),
    )
}

#[test]
fn collect_helpers_capture_expected_kinds_and_labels() -> Result<(), Box<dyn std::error::Error>> {
    let ast = sample_ast();

    let mut kinds = BTreeSet::new();
    collect_node_kinds(&ast, &mut kinds);

    assert!(kinds.contains("Program"));
    assert!(kinds.contains("ExpressionStatement"));
    assert!(kinds.contains("Binary"));
    assert!(kinds.contains("FunctionCall"));
    assert!(kinds.contains("Variable"));

    let mut labeled = BTreeMap::new();
    collect_node_kinds_labeled(&ast, "fixture.pl", &mut labeled);

    let function_labels = labeled.get("FunctionCall").ok_or("FunctionCall should be recorded")?;
    assert!(function_labels.contains("fixture.pl"));

    Ok(())
}

#[test]
fn parent_and_lookup_helpers_work_for_nested_nodes() -> Result<(), Box<dyn std::error::Error>> {
    let ast = sample_ast();

    let mut parents = BTreeMap::new();
    collect_node_kinds_with_parents(&ast, None, &mut parents);

    let binary_parents = parents.get("Binary").ok_or("Binary should have parents")?;
    assert!(binary_parents.contains("ExpressionStatement"));

    let variable_parents = parents.get("Variable").ok_or("Variable should have parents")?;
    assert!(variable_parents.contains("FunctionCall"));

    assert!(has_node_kind(&ast, "FunctionCall"));
    assert!(!has_node_kind(&ast, "Heredoc"));

    let found = find_first_node_of_kind(&ast, "FunctionCall").ok_or("FunctionCall should exist")?;
    assert!(matches!(found.kind, NodeKind::FunctionCall { .. }));

    Ok(())
}

#[test]
fn corpus_required_kinds_excludes_recovery_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let required = corpus_required_kinds();

    assert_eq!(required.len() + SYNTHETIC_NODE_KIND_NAMES.len(), ALL_NODE_KIND_NAMES.len());

    for recovery_kind in SYNTHETIC_NODE_KIND_NAMES {
        assert!(
            !required.contains(recovery_kind),
            "recovery kind {recovery_kind} should not be required"
        );
    }

    assert!(required.contains("Program"));
    assert!(required.contains("FunctionCall"));

    Ok(())
}
