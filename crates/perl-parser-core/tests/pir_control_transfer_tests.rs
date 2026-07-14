//! PIR v0 ControlTransfer (return) lowering tests.
//!
//! Covers the third PIR control-flow slice from PLSP-SPEC-0025 (issue #8196):
//! HIR `ControlTransfer` with `kind == Return` now lowers into
//! `PirOperation::Return` instead of being counted as an unsupported construct.
//! Non-return transfer verbs (`next`/`last`/`redo`/`goto`) deliberately stay in
//! `unsupported_construct_counts` — they are loop-control / goto transfers, not
//! subroutine returns. Mirrors the BranchShell→Branch and LoopShell→Loop slices
//! in `pir_branch_tests.rs` / `pir_loop_tests.rs`.

use perl_parser_core::Parser;
use perl_parser_core::hir::{HirFile, lower_ast};
use perl_parser_core::pir::{
    PirAnchorKind, PirContext, PirEdgeKind, PirGraph, PirOperation, lower_hir,
};
use perl_tdd_support::must_some;

fn lower(source: &str) -> PirGraph {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir: HirFile = lower_ast(&output.ast);
    lower_hir(&hir)
}

/// Helper: find the first Return node in the graph.
fn first_return_node(graph: &PirGraph) -> &perl_parser_core::pir::PirNode {
    must_some(graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Return)))
}

#[test]
fn return_lowers_to_one_return_node_with_void_context() {
    // `return 1;` must produce exactly one Return node in Void context.
    let graph = lower("sub f { return 1; }");

    let return_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Return)).collect();
    assert_eq!(return_nodes.len(), 1, "a single return should produce exactly one Return node");

    let return_node = &return_nodes[0];

    // Context must be Void — a return statement yields no value at this level.
    assert_eq!(return_node.context, PirContext::Void, "Return node must have Void context");

    // Source anchor must be set — we lower from an explicit HIR source range.
    assert!(return_node.source_anchor.is_anchored(), "Return node must preserve a source anchor");

    // Receipt must count the Return operation.
    assert_eq!(
        graph.receipt.operation_counts.get("Return"),
        Some(&1),
        "receipt must count one Return operation"
    );
}

#[test]
fn return_with_value_still_lowers_to_one_return_node() {
    // `return $x;` carries a value payload (HIR `has_value = true`). PIR v0 does
    // not lower the returned expression to a separate node yet, so the result is
    // still exactly one Return node — the value is a named follow-up, not an
    // extra node here.
    let graph = lower("sub f { return $x; }");

    let return_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Return)).collect();
    assert_eq!(return_nodes.len(), 1, "return-with-value should still produce one Return node");
    assert_eq!(return_nodes[0].context, PirContext::Void);
    assert!(return_nodes[0].source_anchor.is_anchored());
}

#[test]
fn bare_return_lowers_to_one_return_node() {
    // `return;` with no value also lowers to a Return node.
    let graph = lower("sub f { return; }");
    let return_nodes: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Return)).collect();
    assert_eq!(return_nodes.len(), 1, "bare return should produce one Return node");
    assert_eq!(graph.receipt.operation_counts.get("Return"), Some(&1));
}

#[test]
fn return_node_has_explicit_source_anchor_kind() {
    let graph = lower("sub f { return 1; }");
    let return_node = first_return_node(&graph);
    assert_eq!(
        return_node.source_anchor.kind,
        PirAnchorKind::ExplicitSource,
        "Return node must anchor as ExplicitSource"
    );
}

#[test]
fn return_alongside_inner_body_lowers_children_too() {
    // `sub f { my $y = bar(); return $y; }` should produce a Return node AND the
    // inner-body operations (LexicalWrite, Assign, Call). Verify Return does not
    // suppress sibling lowering.
    let graph = lower("sub f { my $y = bar(); return $y; }");

    let has_return = graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::Return));
    let has_write =
        graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::LexicalWrite { .. }));
    let has_call = graph.nodes.iter().any(|n| matches!(n.operation, PirOperation::Call { .. }));

    assert!(has_return, "should have a Return node for the return statement");
    assert!(has_write, "should have a LexicalWrite node for my $y");
    assert!(has_call, "should have a Call node for bar()");
}

#[test]
fn return_not_in_unsupported_construct_counts() {
    // After this slice, a `return` lowers to a Return operation. It must no
    // longer appear in unsupported_construct_counts.
    let graph = lower("sub f { return 1; }");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("ControlTransfer"),
        None,
        "a return must not appear in unsupported counts — it now lowers to Return"
    );
}

#[test]
fn last_stays_unsupported_and_is_not_a_return() {
    // `last` is a loop-control transfer, not a subroutine return. It must remain
    // in unsupported_construct_counts and must NOT produce a Return node.
    let graph = lower("while (1) { last; }");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("ControlTransfer"),
        Some(&1),
        "last must stay an unsupported ControlTransfer"
    );
    assert_eq!(
        graph.receipt.operation_counts.get("Return"),
        None,
        "last must not be lowered to a Return operation"
    );
}

#[test]
fn next_and_redo_stay_unsupported() {
    // `next` and `redo` are loop-control transfers — neither lowers to Return.
    // Two non-return transfers => unsupported ControlTransfer count of 2.
    let graph = lower("while (1) { next; redo; }");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("ControlTransfer"),
        Some(&2),
        "next and redo must both stay unsupported ControlTransfers"
    );
    assert_eq!(graph.receipt.operation_counts.get("Return"), None);
}

#[test]
fn return_and_loop_control_in_same_fixture_are_discriminated() {
    // A sub with both a `return` and a `last` must split: the return lowers to a
    // Return op, the last stays an unsupported ControlTransfer.
    let graph = lower("sub f { while (1) { last; } return 1; }");
    assert_eq!(
        graph.receipt.operation_counts.get("Return"),
        Some(&1),
        "the return must lower to exactly one Return op"
    );
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("ControlTransfer"),
        Some(&1),
        "the last must remain exactly one unsupported ControlTransfer"
    );
}

#[test]
fn goto_stays_unsupported_and_is_not_a_return() {
    // `goto &sub` is a Goto transfer, not a subroutine return. It must stay an
    // unsupported ControlTransfer and must NOT lower to a Return operation —
    // this locks the discrimination boundary so a future change can't silently
    // start treating goto as a return.
    let graph = lower("sub f { goto &handler; }");
    assert_eq!(
        graph.receipt.operation_counts.get("Return"),
        None,
        "goto must not be lowered to a Return operation"
    );
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("ControlTransfer"),
        Some(&1),
        "goto must stay an unsupported ControlTransfer"
    );
}

#[test]
fn return_is_terminal_no_outgoing_fallthrough_and_records_return_exit_edge() {
    // A `return` is terminal: control leaves the subroutine. The Return node
    // must NOT be a Fallthrough source, and it must record a Return exit edge
    // (to: None) leaving the modeled graph — mirroring the DynamicExit shape.
    let graph = lower("sub f { return 1; }");
    let return_id = first_return_node(&graph).id;

    let outgoing_fallthrough =
        graph.edges.iter().any(|e| e.from == return_id && e.kind == PirEdgeKind::Fallthrough);
    assert!(!outgoing_fallthrough, "a Return node must not be a Fallthrough source");

    let has_return_exit = graph
        .edges
        .iter()
        .any(|e| e.from == return_id && e.to.is_none() && e.kind == PirEdgeKind::Return);
    assert!(has_return_exit, "a Return node must record a Return exit edge (to: None)");
}

#[test]
fn return_with_call_does_not_fallthrough_from_return_to_callee() {
    // `return foo();` — HIR emits the ControlTransfer item *before* the returned
    // CallExpr sibling. The Return must not link to that Call (or to any later
    // sibling) by a spurious Fallthrough edge; control does not continue past a
    // terminal return.
    let graph = lower("sub f { return foo(); }");
    let return_id = first_return_node(&graph).id;
    let no_fallthrough_from_return =
        graph.edges.iter().all(|e| !(e.from == return_id && e.kind == PirEdgeKind::Fallthrough));
    assert!(
        no_fallthrough_from_return,
        "return must not fall through to the returned-call sibling: {:?}",
        graph.edges
    );
}

#[test]
fn return_receipt_counts_are_consistent() {
    // operation_counts and context_counts must each sum to the node count when a
    // Return node is present.
    let graph = lower("sub f { my $y = 1; return $y; }");

    let op_total: usize = graph.receipt.operation_counts.values().sum();
    assert_eq!(op_total, graph.nodes.len(), "operation_counts must sum to node count");

    let ctx_total: usize = graph.receipt.context_counts.values().sum();
    assert_eq!(ctx_total, graph.nodes.len(), "context_counts must sum to node count");

    assert_eq!(graph.receipt.node_count, graph.nodes.len(), "receipt.node_count must match nodes");
    assert_eq!(graph.receipt.edge_count, graph.edges.len(), "receipt.edge_count must match edges");
}
