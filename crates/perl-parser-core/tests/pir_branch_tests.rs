//! PIR v0 BranchShell lowering tests.
//!
//! Covers the first PIR control-flow slice from PLSP-SPEC-0025 (issue #8196):
//! HIR `BranchShell` now lowers into `PirOperation::Branch` instead of being
//! counted as an unsupported construct.

use perl_parser_core::Parser;
use perl_parser_core::hir::{HirFile, lower_ast};
use perl_parser_core::pir::{PirContext, PirGraph, PirOperation, lower_hir};
use perl_tdd_support::must_some;

fn lower(source: &str) -> PirGraph {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir: HirFile = lower_ast(&output.ast);
    lower_hir(&hir)
}

/// Helper: find the first Branch node in the graph.
fn first_branch_node(graph: &PirGraph) -> &perl_parser_core::pir::PirNode {
    must_some(graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Branch { .. })))
}

#[test]
fn if_block_lowers_to_one_branch_node_with_void_context() {
    // `if ($x) { 1 }` must produce exactly one Branch node.
    // Condition is not lowered to a separate PIR node in v0 — condition is None.
    // Context is Void: an if-statement is a control-flow fork at statement level.
    let graph = lower("if ($x) { 1 }");

    let branch_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Branch { .. })).collect();
    assert_eq!(branch_nodes.len(), 1, "if block should produce exactly one Branch node");

    let branch = &branch_nodes[0];

    // Context must be Void — the branch statement yields no value at this level.
    assert_eq!(branch.context, PirContext::Void, "Branch node must have Void context");

    // Source anchor must be set — we lower from an explicit HIR source range.
    assert!(branch.source_anchor.is_anchored(), "Branch node must preserve a source anchor");

    // Receipt must count the Branch operation.
    assert_eq!(
        graph.receipt.operation_counts.get("Branch"),
        Some(&1),
        "receipt must count one Branch operation"
    );
}

#[test]
fn unless_block_lowers_to_one_branch_node() {
    // `unless ($x) { 1 }` lowers to BranchShell (same HIR variant as `if`).
    // PIR v0 emits one Branch node regardless of the surface keyword.
    let graph = lower("unless ($x) { 1 }");

    let branch_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Branch { .. })).collect();
    assert_eq!(branch_nodes.len(), 1, "unless block should produce exactly one Branch node");

    assert_eq!(branch_nodes[0].context, PirContext::Void);
    assert!(branch_nodes[0].source_anchor.is_anchored());
}

#[test]
fn if_elsif_else_lowers_to_one_branch_node() {
    // `if ($a) {} elsif ($b) {} else {}` lowers to a single BranchShell HIR
    // item. PIR v0 emits one Branch node from that single shell.
    let graph = lower("if ($a) {} elsif ($b) {} else {}");

    let branch_count =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Branch { .. })).count();
    assert_eq!(
        branch_count, 1,
        "if/elsif/else chain is one BranchShell and must produce exactly one Branch node"
    );
}

#[test]
fn branch_node_condition_is_none_in_v0() {
    // PIR v0 defers condition-expression lowering. The condition field must be
    // None — it is a named follow-up, not a silent omission.
    let graph = lower("if ($x) { 1 }");
    let branch = first_branch_node(&graph);
    match &branch.operation {
        PirOperation::Branch { condition } => {
            assert!(
                condition.is_none(),
                "PIR v0: condition must be None — lowering condition expressions is a named follow-up"
            );
        }
        _ => panic!("expected Branch operation"),
    }
}

#[test]
fn branch_with_inner_body_lowers_children_too() {
    // `if ($x) { my $y = bar(); }` should produce:
    // - one Branch node (for the if shell)
    // - one LexicalWrite node (for `my $y`)
    // - one Assign node (for the `= bar()`)
    // - one Call node (for `bar()`)
    // Verify Branch AND the inner-body operations all appear.
    let graph = lower("if ($x) { my $y = bar(); }");

    let has_branch = graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::Branch { .. }));
    let has_write =
        graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::LexicalWrite { .. }));
    let has_call = graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::Call { .. }));

    assert!(has_branch, "should have a Branch node for the if shell");
    assert!(has_write, "should have a LexicalWrite node for my $y");
    assert!(has_call, "should have a Call node for bar()");
}

#[test]
fn branch_shell_not_in_unsupported_construct_counts() {
    // After this slice, BranchShell lowers to a Branch operation. It must no
    // longer appear in unsupported_construct_counts.
    let graph = lower("if ($x) { 1 }");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("BranchShell"),
        None,
        "BranchShell must not appear in unsupported counts — it now lowers to Branch"
    );
}

#[test]
fn branch_receipt_counts_are_consistent() {
    // Verify that operation_counts, context_counts, and node_count all sum
    // consistently when a Branch node is present.
    let graph = lower("if ($x) { my $y = 1; }");

    let op_total: usize = graph.receipt.operation_counts.values().sum();
    assert_eq!(op_total, graph.nodes.len(), "operation_counts must sum to node count");

    let ctx_total: usize = graph.receipt.context_counts.values().sum();
    assert_eq!(ctx_total, graph.nodes.len(), "context_counts must sum to node count");

    assert_eq!(graph.receipt.node_count, graph.nodes.len(), "receipt.node_count must match nodes");
    assert_eq!(graph.receipt.edge_count, graph.edges.len(), "receipt.edge_count must match edges");
}

#[test]
fn branch_node_has_explicit_source_anchor_kind() {
    use perl_parser_core::pir::PirAnchorKind;
    let graph = lower("if ($x) { 1 }");
    let branch = first_branch_node(&graph);
    assert_eq!(
        branch.source_anchor.kind,
        PirAnchorKind::ExplicitSource,
        "Branch node must anchor as ExplicitSource"
    );
}

#[test]
fn multiple_branches_each_produce_one_branch_node() {
    // Two independent if-statements must each produce one Branch node.
    let graph = lower("if ($a) { 1 } if ($b) { 2 }");

    let branch_count =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Branch { .. })).count();
    assert_eq!(branch_count, 2, "two if-statements must produce two Branch nodes");

    assert_eq!(graph.receipt.operation_counts.get("Branch"), Some(&2));
}
