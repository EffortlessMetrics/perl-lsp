//! PIR v0 LoopShell lowering tests.
//!
//! Covers the second PIR control-flow slice from PLSP-SPEC-0025 (issue #8196):
//! HIR `LoopShell` now lowers into `PirOperation::Loop` instead of being
//! counted as an unsupported construct. Mirrors the BranchShell→Branch slice
//! in `pir_branch_tests.rs`.

use perl_parser_core::Parser;
use perl_parser_core::hir::{HirFile, lower_ast};
use perl_parser_core::pir::{PirAnchorKind, PirContext, PirGraph, PirOperation, lower_hir};
use perl_tdd_support::must_some;

fn lower(source: &str) -> PirGraph {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir: HirFile = lower_ast(&output.ast);
    lower_hir(&hir)
}

/// Helper: find the first Loop node in the graph.
fn first_loop_node(graph: &PirGraph) -> &perl_parser_core::pir::PirNode {
    must_some(graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Loop { .. })))
}

#[test]
fn while_block_lowers_to_one_loop_node_with_void_context() {
    // `while ($x) { 1 }` must produce exactly one Loop node.
    // Condition is not lowered to a separate PIR node in v0 — condition is None.
    // Context is Void: a while-statement is a control-flow loop at statement level.
    let graph = lower("while ($x) { 1 }");

    let loop_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Loop { .. })).collect();
    assert_eq!(loop_nodes.len(), 1, "while block should produce exactly one Loop node");

    let loop_node = &loop_nodes[0];

    // Context must be Void — the loop statement yields no value at this level.
    assert_eq!(loop_node.context, PirContext::Void, "Loop node must have Void context");

    // Source anchor must be set — we lower from an explicit HIR source range.
    assert!(loop_node.source_anchor.is_anchored(), "Loop node must preserve a source anchor");

    // Receipt must count the Loop operation.
    assert_eq!(
        graph.receipt.operation_counts.get("Loop"),
        Some(&1),
        "receipt must count one Loop operation"
    );
}

#[test]
fn until_block_lowers_to_one_loop_node() {
    // `until ($x) { 1 }` lowers to LoopShell (same HIR variant as `while`).
    // PIR v0 emits one Loop node regardless of the surface keyword.
    let graph = lower("until ($x) { 1 }");

    let loop_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Loop { .. })).collect();
    assert_eq!(loop_nodes.len(), 1, "until block should produce exactly one Loop node");

    assert_eq!(loop_nodes[0].context, PirContext::Void);
    assert!(loop_nodes[0].source_anchor.is_anchored());
}

#[test]
fn c_style_for_lowers_to_one_loop_node() {
    // `for (my $i=0; $i<3; $i++) { 1 }` is a CStyleFor LoopKind.
    // PIR v0 emits one Loop node.
    let graph = lower("for (my $i=0; $i<3; $i++) { 1 }");

    let loop_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Loop { .. })).collect();
    assert_eq!(loop_nodes.len(), 1, "C-style for should produce exactly one Loop node");

    assert_eq!(loop_nodes[0].context, PirContext::Void);
    assert!(loop_nodes[0].source_anchor.is_anchored());
}

#[test]
fn foreach_lowers_to_one_loop_node() {
    // `foreach my $e (@list) { 1 }` is a Foreach LoopKind.
    // PIR v0 emits one Loop node.
    let graph = lower("foreach my $e (@list) { 1 }");

    let loop_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Loop { .. })).collect();
    assert_eq!(loop_nodes.len(), 1, "foreach should produce exactly one Loop node");

    assert_eq!(loop_nodes[0].context, PirContext::Void);
    assert!(loop_nodes[0].source_anchor.is_anchored());

    assert_eq!(graph.receipt.operation_counts.get("Loop"), Some(&1));
}

#[test]
fn loop_node_condition_is_none_in_v0() {
    // PIR v0 defers condition-expression lowering. The condition field must be
    // None — it is a named follow-up, not a silent omission.
    let graph = lower("while ($x) { 1 }");
    let loop_node = first_loop_node(&graph);
    let condition = must_some(match &loop_node.operation {
        PirOperation::Loop { condition } => Some(condition),
        _ => None,
    });
    assert!(
        condition.is_none(),
        "PIR v0: condition must be None — lowering condition expressions is a named follow-up"
    );
}

#[test]
fn loop_with_inner_body_lowers_children_too() {
    // `while ($x) { my $y = bar(); }` should produce:
    // - one Loop node (for the while shell)
    // - one LexicalWrite node (for `my $y`)
    // - one Assign node (for the `= bar()`)
    // - one Call node (for `bar()`)
    // Verify Loop AND the inner-body operations all appear.
    let graph = lower("while ($x) { my $y = bar(); }");

    let has_loop = graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::Loop { .. }));
    let has_write =
        graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::LexicalWrite { .. }));
    let has_call = graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::Call { .. }));

    assert!(has_loop, "should have a Loop node for the while shell");
    assert!(has_write, "should have a LexicalWrite node for my $y");
    assert!(has_call, "should have a Call node for bar()");
}

#[test]
fn loop_shell_not_in_unsupported_construct_counts() {
    // After this slice, LoopShell lowers to a Loop operation. It must no
    // longer appear in unsupported_construct_counts.
    let graph = lower("while ($x) { 1 }");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("LoopShell"),
        None,
        "LoopShell must not appear in unsupported counts — it now lowers to Loop"
    );
}

#[test]
fn loop_receipt_counts_are_consistent() {
    // Verify that operation_counts, context_counts, and node_count all sum
    // consistently when a Loop node is present.
    let graph = lower("while ($x) { my $y = 1; }");

    let op_total: usize = graph.receipt.operation_counts.values().sum();
    assert_eq!(op_total, graph.nodes.len(), "operation_counts must sum to node count");

    let ctx_total: usize = graph.receipt.context_counts.values().sum();
    assert_eq!(ctx_total, graph.nodes.len(), "context_counts must sum to node count");

    assert_eq!(graph.receipt.node_count, graph.nodes.len(), "receipt.node_count must match nodes");
    assert_eq!(graph.receipt.edge_count, graph.edges.len(), "receipt.edge_count must match edges");
}

#[test]
fn loop_node_has_explicit_source_anchor_kind() {
    let graph = lower("while ($x) { 1 }");
    let loop_node = first_loop_node(&graph);
    assert_eq!(
        loop_node.source_anchor.kind,
        PirAnchorKind::ExplicitSource,
        "Loop node must anchor as ExplicitSource"
    );
}
