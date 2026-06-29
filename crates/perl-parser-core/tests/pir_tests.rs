//! PIR v0 lowering tests.
//!
//! These cover the [`PLSP-SPEC-0025`](../../../docs/specs/PLSP-SPEC-0025-pir-v0.md)
//! acceptance surface: source-anchor preservation, dynamic-boundary
//! preservation and links, visible unknown context, the operation families PIR
//! v0 lowers, conservative control-flow edges, and the lowering receipt.

use perl_parser_core::Parser;
use perl_parser_core::hir::{HirFile, lower_ast};
use perl_parser_core::pir::{
    PIR_RECEIPT_VERSION, PirCallee, PirContext, PirDynamicBoundaryKind, PirEdgeKind, PirGraph,
    PirLoweringMode, PirMethod, PirOperation, lower_hir, lower_hir_with_identity,
};
use perl_tdd_support::must_some;

fn lower(source: &str) -> PirGraph {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir: HirFile = lower_ast(&output.ast);
    lower_hir(&hir)
}

fn op_names(graph: &PirGraph) -> Vec<&'static str> {
    graph.nodes.iter().map(|node| node.operation.name()).collect()
}

fn first_op<'a, T>(graph: &'a PirGraph, select: impl Fn(&'a PirOperation) -> Option<T>) -> T {
    must_some(graph.nodes.iter().find_map(|node| select(&node.operation)))
}

#[test]
fn empty_source_yields_empty_graph() {
    let graph = lower("");
    assert!(graph.is_empty());
    assert_eq!(graph.nodes.len(), 0);
    assert_eq!(graph.edges.len(), 0);
    assert_eq!(graph.receipt.node_count, 0);
    assert_eq!(graph.receipt.edge_count, 0);
    assert_eq!(graph.receipt.schema_version, PIR_RECEIPT_VERSION);
    assert_eq!(graph.receipt.lowering_mode, PirLoweringMode::HirV0);
    assert!(!graph.receipt.provider_behavior_changed);
}

#[test]
fn lexical_declaration_writes_lvalue_and_assigns() {
    let graph = lower("my $x = 1;");
    assert_eq!(op_names(&graph), vec!["LexicalWrite", "Assign"]);

    // The declared write target is a known lvalue; the statement-level
    // assignment is void. Neither is silently promoted past what HIR proves.
    assert_eq!(graph.nodes[0].context, PirContext::Lvalue);
    assert_eq!(graph.nodes[1].context, PirContext::Void);

    let name = first_op(&graph, |op| match op {
        PirOperation::LexicalWrite { name } => Some(name.clone()),
        _ => None,
    });
    assert_eq!(name.sigil, "$");
    assert_eq!(name.name, "x");
}

#[test]
fn our_declaration_is_a_stash_write_with_package() {
    let graph = lower("package Acme; our @items = (1, 2);");
    let symbol = first_op(&graph, |op| match op {
        PirOperation::StashWrite { symbol } => Some(symbol.clone()),
        _ => None,
    });
    assert_eq!(symbol.sigil, "@");
    assert_eq!(symbol.name, "items");
    assert_eq!(symbol.package.as_deref(), Some("Acme"));
}

#[test]
fn local_declaration_is_a_stash_write() {
    // `local` dynamically scopes a package/global slot, so it lowers to a stash
    // write, not a lexical write (unlike `my`/`state`).
    let graph = lower("local $x;");
    let symbol = first_op(&graph, |op| match op {
        PirOperation::StashWrite { symbol } => Some(symbol.clone()),
        _ => None,
    });
    assert_eq!(symbol.sigil, "$");
    assert_eq!(symbol.name, "x");
}

#[test]
fn named_call_splits_package_qualifier() {
    let graph = lower("Bar::baz();");
    let callee = first_op(&graph, |op| match op {
        PirOperation::Call { callee, .. } => Some(callee.clone()),
        _ => None,
    });
    assert_eq!(
        callee,
        PirCallee::Named { name: "baz".to_string(), package: Some("Bar".to_string()) }
    );
}

#[test]
fn deep_qualified_call_preserves_full_package_path() {
    // The qualifier split keeps the full package path (`rsplit_once`), so the
    // method name is just the final segment and the package is everything else.
    let graph = lower("A::B::foo();");
    let callee = first_op(&graph, |op| match op {
        PirOperation::Call { callee, .. } => Some(callee.clone()),
        _ => None,
    });
    assert_eq!(
        callee,
        PirCallee::Named { name: "foo".to_string(), package: Some("A::B".to_string()) }
    );
}

#[test]
fn unqualified_call_has_no_package() {
    let graph = lower("foo(1, 2, 3);");
    let (callee, arg_count) = first_op(&graph, |op| match op {
        PirOperation::Call { callee, arg_count } => Some((callee.clone(), *arg_count)),
        _ => None,
    });
    assert_eq!(callee, PirCallee::Named { name: "foo".to_string(), package: None });
    assert_eq!(arg_count, 3);

    // A call's runtime context is not provable from HIR; it stays Unknown.
    let context = must_some(
        graph
            .nodes
            .iter()
            .find(|node| matches!(node.operation, PirOperation::Call { .. }))
            .map(|node| node.context),
    );
    assert_eq!(context, PirContext::Unknown);
}

#[test]
fn method_call_lowers_receiver_and_method() {
    let graph = lower("$obj->frobnicate(1, 2);");
    let (method, arg_count) = first_op(&graph, |op| match op {
        PirOperation::MethodCall { method, arg_count, .. } => Some((method.clone(), *arg_count)),
        _ => None,
    });
    assert_eq!(method, PirMethod::Named("frobnicate".to_string()));
    assert_eq!(arg_count, 2);
}

#[test]
fn coderef_call_links_to_dynamic_boundary() {
    let graph = lower("my $cb; $cb->(1);");

    let call = must_some(graph.nodes.iter().find(|node| {
        matches!(node.operation, PirOperation::Call { callee: PirCallee::Dynamic, .. })
    }));

    // The dynamic call preserves a link to its boundary instead of guessing.
    let boundary_id = must_some(call.dynamic_boundary);
    let boundary = must_some(graph.node(boundary_id));
    let kind = must_some(match &boundary.operation {
        PirOperation::DynamicBoundary { kind, .. } => Some(*kind),
        _ => None,
    });
    assert_eq!(kind, PirDynamicBoundaryKind::DynamicCallee);

    // Exactly one boundary — PIR links HIR's boundary, it does not duplicate it.
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("DynamicCallee"), Some(&1));

    // Control may not return through the boundary; the exit edge is preserved.
    assert!(graph.edges.iter().any(|edge| edge.from == boundary_id
        && edge.to.is_none()
        && edge.kind == PirEdgeKind::DynamicExit));
}

#[test]
fn each_coderef_call_links_its_own_boundary() {
    // Two independent coderef calls and a nested one. Each dynamic Call must
    // link to a distinct DynamicCallee boundary — never a stale one from an
    // earlier or unrelated call.
    let graph = lower("$a->(); $b->($c->());");

    let dynamic_calls: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(node.operation, PirOperation::Call { callee: PirCallee::Dynamic, .. })
        })
        .collect();
    assert_eq!(dynamic_calls.len(), 3);

    let mut linked_boundaries = Vec::new();
    for call in dynamic_calls {
        let boundary_id = must_some(call.dynamic_boundary);
        let boundary = must_some(graph.node(boundary_id));
        let kind = must_some(match &boundary.operation {
            PirOperation::DynamicBoundary { kind, .. } => Some(*kind),
            _ => None,
        });
        assert_eq!(kind, PirDynamicBoundaryKind::DynamicCallee);
        linked_boundaries.push(boundary_id);
    }

    // Every link is to a distinct boundary node — no stale reuse.
    linked_boundaries.sort_by_key(|id| id.index());
    linked_boundaries.dedup();
    assert_eq!(linked_boundaries.len(), 3);
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("DynamicCallee"), Some(&3));
}

#[test]
fn eval_string_is_a_dynamic_boundary() {
    let graph = lower(r#"eval "$code";"#);
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("EvalExpression"), Some(&1));
}

#[test]
fn symbolic_reference_is_a_dynamic_boundary() {
    let graph = lower("no strict 'refs'; my $v = ${$name};");
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("SymbolicReference"), Some(&1));
}

#[test]
fn typeglob_assignment_is_a_runtime_stash_mutation_boundary() {
    let graph = lower("*alias = $thing;");
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("RuntimeStashMutation"), Some(&1));
}

#[test]
fn autoload_declaration_is_a_dynamic_boundary() {
    let graph = lower("sub AUTOLOAD { }");
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("Autoload"), Some(&1));
}

#[test]
fn dynamic_boundary_nodes_anchor_to_the_boundary_range() {
    let graph = lower(r#"eval "$code";"#);
    let boundary = must_some(
        graph
            .nodes
            .iter()
            .find(|node| matches!(node.operation, PirOperation::DynamicBoundary { .. })),
    );
    assert_eq!(boundary.source_anchor.kind.name(), "DynamicBoundary");
    assert!(boundary.source_anchor.is_anchored());
}

#[test]
fn every_lowered_node_preserves_a_source_anchor() {
    let graph = lower("package Foo; my $x = bar(); $obj->m(); our $y;");
    assert!(graph.nodes.iter().all(|node| node.source_anchor.is_anchored()));
    assert_eq!(graph.receipt.source_anchor_coverage.unanchored, 0);
    assert_eq!(graph.receipt.source_anchor_coverage.anchored, graph.nodes.len());
    assert_eq!(graph.receipt.source_anchor_coverage.total(), graph.nodes.len());
}

#[test]
fn unlowered_constructs_are_counted_not_dropped() {
    let graph = lower("package Foo; use strict; sub f {}");
    // None of these lower to PIR operations in v0...
    assert!(graph.is_empty());
    // ...but they are visible in the receipt rather than silently dropped.
    let unsupported = &graph.receipt.unsupported_construct_counts;
    assert_eq!(unsupported.get("PackageDecl"), Some(&1));
    assert_eq!(unsupported.get("UseDecl"), Some(&1));
    assert_eq!(unsupported.get("SubDecl"), Some(&1));
}

#[test]
fn receipt_counts_are_consistent_with_nodes() {
    let graph = lower("my $x = 1; foo(); $obj->m(); eval '1';");

    let op_total: usize = graph.receipt.operation_counts.values().sum();
    assert_eq!(op_total, graph.nodes.len());
    assert_eq!(graph.receipt.node_count, graph.nodes.len());

    let ctx_total: usize = graph.receipt.context_counts.values().sum();
    assert_eq!(ctx_total, graph.nodes.len());

    assert_eq!(graph.receipt.edge_count, graph.edges.len());
    assert!(!graph.receipt.provider_behavior_changed);
    assert!(graph.receipt.ambient_inputs.is_empty());
}

#[test]
fn consecutive_nodes_in_a_scope_are_linked_by_fallthrough() {
    let graph = lower("foo(); bar(); baz();");
    let fallthroughs =
        graph.edges.iter().filter(|edge| edge.kind == PirEdgeKind::Fallthrough).count();
    // Three sequential calls in the file scope produce two fallthrough edges.
    assert_eq!(fallthroughs, 2);
}

#[test]
fn lowering_is_deterministic() {
    let mut parser = Parser::new("package Foo; my $x = bar(); $obj->m(); eval '1'; our @z;");
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);

    let first = lower_hir(&hir);
    let second = lower_hir(&hir);
    assert_eq!(first, second);
}

#[test]
fn source_identity_is_threaded_into_the_receipt() {
    let mut parser = Parser::new("my $x = 1;");
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);

    let graph = lower_hir_with_identity(&hir, Some("fixture://demo.pl".to_string()));
    assert_eq!(graph.receipt.source_identity.as_deref(), Some("fixture://demo.pl"));

    let anonymous = lower_hir(&hir);
    assert!(anonymous.receipt.source_identity.is_none());
}

#[test]
fn node_lookup_round_trips() {
    let graph = lower("my $x = 1;");
    for node in &graph.nodes {
        assert_eq!(graph.node(node.id), Some(node));
    }
    assert!(graph.node(perl_parser_core::pir::PirId::from_index(9999)).is_none());
}

#[test]
fn multi_variable_declaration_produces_one_write_per_variable() {
    // `my ($a, $b) = (1, 2)` must produce two LexicalWrite nodes (one for each
    // variable) plus one Assign node for the initialiser.
    let graph = lower("my ($a, $b) = (1, 2);");
    let writes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.operation, PirOperation::LexicalWrite { .. }))
        .collect();
    assert_eq!(writes.len(), 2, "expected one LexicalWrite per variable");

    let names: Vec<&str> = writes
        .iter()
        .map(|n| match &n.operation {
            PirOperation::LexicalWrite { name } => name.name.as_str(),
            _ => unreachable!(),
        })
        .collect();
    assert!(names.contains(&"a"), "expected write for $a");
    assert!(names.contains(&"b"), "expected write for $b");

    // Every write is an lvalue; the assignment is void.
    assert!(writes.iter().all(|n| n.context == PirContext::Lvalue));

    let assigns: Vec<_> =
        graph.nodes.iter().filter(|n| matches!(n.operation, PirOperation::Assign)).collect();
    assert_eq!(assigns.len(), 1);
    assert_eq!(assigns[0].context, PirContext::Void);
}

#[test]
fn named_callee_leading_colons_do_not_produce_empty_package() {
    // `::foo` has a leading `::` that `rsplit_once("::")` splits into ("", "foo").
    // The guard `!package.is_empty()` must reject the empty-string package half,
    // so the callee becomes `Named { name: "foo", package: None }` rather than
    // `Named { name: "foo", package: Some("") }`. An empty-string package qualifier
    // would be a confusing artifact; bare-name is the conservative fallback.
    let graph = lower("::foo();");
    let callee = first_op(&graph, |op| match op {
        PirOperation::Call { callee, .. } => Some(callee.clone()),
        _ => None,
    });
    // package must be None (not Some("")) — the empty part is dropped.
    assert_eq!(callee, PirCallee::Named { name: "foo".to_string(), package: None });
}

#[test]
fn two_consecutive_coderef_calls_each_link_to_their_own_boundary() {
    // Two back-to-back coderef calls must each link to their own
    // DynamicBoundary, not share one. The pending_dynamic_callee state machine
    // must be cleared after each linkage.
    let graph = lower("my ($a, $b); $a->(); $b->();");

    let dynamic_calls: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.operation, PirOperation::Call { callee: PirCallee::Dynamic, .. }))
        .collect();
    assert_eq!(dynamic_calls.len(), 2, "expected two coderef calls");

    // Both calls must carry a dynamic_boundary link.
    let b0 = must_some(dynamic_calls[0].dynamic_boundary);
    let b1 = must_some(dynamic_calls[1].dynamic_boundary);

    // The two boundary nodes must be distinct.
    assert_ne!(b0, b1, "each coderef call must link to its own boundary node");

    // Both boundaries must be DynamicCallee kind.
    let node_b0 = must_some(graph.node(b0));
    let node_b1 = must_some(graph.node(b1));
    assert!(matches!(
        node_b0.operation,
        PirOperation::DynamicBoundary { kind: PirDynamicBoundaryKind::DynamicCallee, .. }
    ));
    assert!(matches!(
        node_b1.operation,
        PirOperation::DynamicBoundary { kind: PirDynamicBoundaryKind::DynamicCallee, .. }
    ));
    // Receipt must count two DynamicCallee boundaries.
    assert_eq!(graph.receipt.dynamic_boundary_counts.get("DynamicCallee"), Some(&2));
}

#[test]
fn control_flow_branch_shell_is_now_lowered_to_branch() {
    // Since #8196, BranchShell lowers to PirOperation::Branch instead of being
    // counted as an unsupported construct. The gap is no longer visible in
    // unsupported_construct_counts — it now appears in operation_counts.
    let graph = lower("if (1) { 1 }");
    // BranchShell is now lowered — it must NOT appear in unsupported counts.
    assert_eq!(graph.receipt.unsupported_construct_counts.get("BranchShell"), None);
    // The Branch operation must be counted in operation_counts.
    assert_eq!(graph.receipt.operation_counts.get("Branch"), Some(&1));
    // The graph is no longer empty — a Branch node was produced.
    assert!(!graph.is_empty());
}

#[test]
fn control_flow_loop_shell_lowers_to_loop_operation() {
    // Since #8196, LoopShell lowers to PirOperation::Loop instead of being
    // counted as an unsupported construct.
    let graph = lower("while (1) { last; }");
    // LoopShell is now lowered — not in unsupported counts.
    assert_eq!(graph.receipt.unsupported_construct_counts.get("LoopShell"), None);
    // The Loop operation must be counted.
    assert_eq!(graph.receipt.operation_counts.get("Loop"), Some(&1));
    // The graph is no longer empty — a Loop node was produced.
    assert!(!graph.is_empty());
}

#[test]
fn control_flow_control_transfer_return_lowers_to_return_operation() {
    // Since #8196, ControlTransferKind::Return lowers to PirOperation::Return.
    // Other transfer verbs (next/last/redo/goto) remain unsupported in v0.
    let graph = lower("sub f { return 1; }");
    // The return is now a Return operation, not an unsupported construct.
    assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), None);
    assert_eq!(graph.receipt.operation_counts.get("Return"), Some(&1));
    // The graph is no longer empty — a Return node was produced.
    assert!(!graph.is_empty());
}

#[test]
fn control_flow_statement_modifier_is_counted_not_dropped() {
    // PIR v0 reserves but does not yet lower StatementModifierShell
    // (postfix if/unless/while/etc).
    let graph = lower("$x = 1 if $y;");
    assert_eq!(graph.receipt.unsupported_construct_counts.get("StatementModifierShell"), Some(&1));
    assert!(graph.is_empty());
}

#[test]
fn all_four_control_flow_kinds_in_same_fixture() {
    // Verify that a fixture exercising all four control-flow HIR variants
    // (BranchShell, LoopShell, ControlTransfer, StatementModifierShell)
    // correctly reflects the current lowering state:
    // - BranchShell now lowers to PirOperation::Branch (#8196)
    // - LoopShell now lowers to PirOperation::Loop (#8196)
    // - ControlTransferKind::Return now lowers to PirOperation::Return (#8196);
    //   non-Return transfers (last/next/redo/goto) and StatementModifierShell
    //   remain unsupported in v0
    let graph = lower(
        r#"
if (1) { 1 }                    # BranchShell — now lowers to Branch
while (1) { last; }             # LoopShell (now lowers to Loop) + ControlTransfer (last — unsupported)
sub f { return 1; }             # ControlTransfer (return — now lowers to Return)
$x = 1 if $y;                   # StatementModifierShell
"#,
    );

    // BranchShell now lowers to a Branch operation — NOT in unsupported counts.
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("BranchShell"),
        None,
        "BranchShell must not be unsupported — it now lowers to Branch"
    );
    assert_eq!(
        graph.receipt.operation_counts.get("Branch"),
        Some(&1),
        "Branch operation count mismatch"
    );

    // LoopShell now lowers to a Loop operation — NOT in unsupported counts.
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("LoopShell"),
        None,
        "LoopShell must not be unsupported — it now lowers to Loop"
    );
    assert_eq!(
        graph.receipt.operation_counts.get("Loop"),
        Some(&1),
        "Loop operation count mismatch"
    );

    // Two ControlTransfers in the fixture: `return 1;` in sub f (now a Return
    // operation) and `last;` in the while loop (still unsupported). So Return
    // op count is 1 and the unsupported ControlTransfer count drops to 1.
    assert_eq!(graph.receipt.operation_counts.get("Return"), Some(&1), "Return op count mismatch");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("ControlTransfer"),
        Some(&1),
        "ControlTransfer (non-Return) count mismatch"
    );
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("StatementModifierShell"),
        Some(&1),
        "StatementModifierShell count mismatch"
    );

    // Graph is no longer empty — Branch and Loop nodes were produced.
    assert!(!graph.is_empty(), "graph must have at least the Branch and Loop nodes");
}

#[test]
fn unless_block_lowers_to_branch_not_statement_modifier() {
    // `unless` block form (not postfix) lowers to BranchShell, which since #8196
    // further lowers to PirOperation::Branch. It must not appear as a
    // StatementModifierShell (postfix form).
    let graph = lower("unless (0) { 1 }");
    // BranchShell is now lowered — not in unsupported.
    assert_eq!(graph.receipt.unsupported_construct_counts.get("BranchShell"), None);
    // Branch operation must be present.
    assert_eq!(graph.receipt.operation_counts.get("Branch"), Some(&1));
    assert!(!graph.receipt.unsupported_construct_counts.contains_key("StatementModifierShell"));
}

#[test]
fn foreach_loop_lowers_to_loop_operation() {
    // `for my $x (LIST)` and `foreach` forms lower to LoopShell, which since
    // #8196 further lowers to PirOperation::Loop.
    let graph = lower("for my $x (1..10) { next; }");
    // LoopShell is now lowered — not in unsupported counts.
    assert_eq!(graph.receipt.unsupported_construct_counts.get("LoopShell"), None);
    // The Loop operation must be present.
    assert_eq!(graph.receipt.operation_counts.get("Loop"), Some(&1));
    // `next` is a ControlTransfer — still unsupported.
    assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), Some(&1));
}

#[test]
fn multiple_statement_modifiers_each_counted() {
    // Each postfix modifier is a separate StatementModifierShell node, so they
    // accumulate in the count.
    let graph = lower("foo() if 1; bar() unless 0; baz() while $x;");
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("StatementModifierShell"),
        Some(&3),
        "expected three postfix modifiers"
    );
}
