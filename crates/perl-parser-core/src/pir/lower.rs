//! HIR-to-PIR v0 lowering.
//!
//! Lowering is intentionally conservative. It lowers the data-access, call, and
//! dynamic-boundary operation families that the current HIR substrate can prove
//! from source, anchors every source-derived node, preserves dynamic-boundary
//! links, and records everything it could not lower in the receipt. It never
//! evaluates Perl and never changes provider behavior.

use std::collections::HashMap;

use crate::hir::{CallForm, DynamicBoundaryKind, HirFile, HirItem, HirKind, HirScopeId};

use super::model::{
    LexicalName, PIR_RECEIPT_VERSION, PirAnchorCoverage, PirCallee, PirContext,
    PirDynamicBoundaryKind, PirEdge, PirEdgeKind, PirGraph, PirId, PirLoweringMode, PirMethod,
    PirNode, PirOperation, PirReceipt, PirReceiver, PirSourceAnchor, SymbolName,
};

/// Lower a [`HirFile`] into a PIR v0 graph with no caller-supplied identity.
#[must_use]
pub fn lower_hir(file: &HirFile) -> PirGraph {
    lower_hir_with_identity(file, None)
}

/// Lower a [`HirFile`] into a PIR v0 graph, tagging the receipt with an
/// optional caller-supplied source or fixture identity.
#[must_use]
pub fn lower_hir_with_identity(file: &HirFile, source_identity: Option<String>) -> PirGraph {
    let mut lowerer = Lowerer::new(source_identity);
    for item in &file.items {
        lowerer.lower_item(item);
    }
    lowerer.finish()
}

struct Lowerer {
    nodes: Vec<PirNode>,
    edges: Vec<PirEdge>,
    next_id: u32,
    last_in_scope: HashMap<Option<HirScopeId>, PirId>,
    /// Most recent dynamic-callee boundary HIR emitted, awaiting the coderef
    /// call it belongs to. HIR lowers a coderef invocation as a
    /// `DynamicBoundary(CoderefCall)` item immediately followed by the
    /// `CallExpr { form: Coderef }` item, so PIR links the two rather than
    /// synthesizing a second boundary.
    pending_dynamic_callee: Option<PirId>,
    unsupported: HashMap<&'static str, usize>,
    source_identity: Option<String>,
}

impl Lowerer {
    fn new(source_identity: Option<String>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 0,
            last_in_scope: HashMap::new(),
            pending_dynamic_callee: None,
            unsupported: HashMap::new(),
            source_identity,
        }
    }

    fn lower_item(&mut self, item: &HirItem) {
        // HIR emits a `DynamicBoundary(CoderefCall)` immediately before its
        // `CallExpr { form: Coderef }`, so the pending boundary is consumed by
        // the very next item. Clear it before any other item so a boundary can
        // never mis-link to a later, unrelated coderef call even if HIR's
        // emission order changes.
        let consumes_pending_callee =
            matches!(&item.kind, HirKind::CallExpr(call) if matches!(call.form, CallForm::Coderef));
        if !consumes_pending_callee {
            self.pending_dynamic_callee = None;
        }

        match &item.kind {
            HirKind::VariableDecl(decl) => self.lower_variable_decl(item, decl),
            HirKind::CallExpr(call) => self.lower_call(item, call),
            HirKind::MethodCallExpr(call) => {
                self.lower_method_call(item, &call.method, call.object_kind, call.arg_count)
            }
            HirKind::IndirectCallExpr(call) => {
                self.lower_method_call(item, &call.method, call.object_kind, call.arg_count)
            }
            HirKind::DynamicBoundary(boundary) => {
                self.lower_dynamic_boundary(
                    item,
                    map_boundary_kind(boundary.kind),
                    boundary.reason.clone(),
                );
            }
            // Construct families PIR v0 does not yet lower. They remain visible
            // in the receipt instead of being silently dropped.
            other => {
                *self.unsupported.entry(hir_kind_name(other)).or_insert(0) += 1;
            }
        }
    }

    fn lower_variable_decl(&mut self, item: &HirItem, decl: &crate::hir::VariableDecl) {
        for variable in &decl.variables {
            let anchor = PirSourceAnchor::explicit(variable.range, item.id);
            let operation = if is_stash_declarator(&decl.declarator) {
                PirOperation::StashWrite {
                    symbol: SymbolName {
                        sigil: variable.sigil.clone(),
                        name: variable.name.clone(),
                        package: item.package_context.clone(),
                    },
                }
            } else {
                PirOperation::LexicalWrite {
                    name: LexicalName {
                        sigil: variable.sigil.clone(),
                        name: variable.name.clone(),
                    },
                }
            };
            // The declaration names a write target, which is a known lvalue.
            self.push_node(item, anchor, operation, PirContext::Lvalue, None);
        }

        if decl.has_initializer {
            // A declaration-with-initializer statement evaluates the assignment
            // in void context; the bound lvalues above carry the known context.
            let anchor = PirSourceAnchor::explicit(item.range, item.id);
            self.push_node(item, anchor, PirOperation::Assign, PirContext::Void, None);
        }
    }

    fn lower_call(&mut self, item: &HirItem, call: &crate::hir::CallExpr) {
        match call.form {
            CallForm::NamedFunction => {
                let anchor = PirSourceAnchor::explicit(item.range, item.id);
                let operation = PirOperation::Call {
                    callee: named_callee(&call.name),
                    arg_count: call.arg_count,
                };
                self.push_node(item, anchor, operation, PirContext::Unknown, None);
            }
            CallForm::Coderef => {
                // HIR already emitted a `DynamicBoundary(CoderefCall)` item
                // just before this call; link to it instead of duplicating it.
                let boundary_id = self.pending_dynamic_callee.take();
                let anchor = PirSourceAnchor::explicit(item.range, item.id);
                let operation =
                    PirOperation::Call { callee: PirCallee::Dynamic, arg_count: call.arg_count };
                self.push_node(item, anchor, operation, PirContext::Unknown, boundary_id);
            }
        }
    }

    fn lower_method_call(
        &mut self,
        item: &HirItem,
        method: &str,
        object_kind: &'static str,
        arg_count: usize,
    ) {
        let anchor = PirSourceAnchor::explicit(item.range, item.id);
        let operation = PirOperation::MethodCall {
            receiver: PirReceiver::Expression { kind: object_kind },
            method: PirMethod::Named(method.to_string()),
            arg_count,
        };
        self.push_node(item, anchor, operation, PirContext::Unknown, None);
    }

    fn lower_dynamic_boundary(
        &mut self,
        item: &HirItem,
        kind: PirDynamicBoundaryKind,
        reason: String,
    ) -> PirId {
        let anchor = PirSourceAnchor::dynamic_boundary(item.range, item.id);
        let id = self.push_node(
            item,
            anchor,
            PirOperation::DynamicBoundary { kind, reason },
            PirContext::Unknown,
            None,
        );
        // Control may not return through a dynamic boundary; record the exit
        // edge instead of dropping it.
        self.edges.push(PirEdge { from: id, to: None, kind: PirEdgeKind::DynamicExit });
        if kind == PirDynamicBoundaryKind::DynamicCallee {
            // Hold this boundary for the coderef call HIR emits next.
            self.pending_dynamic_callee = Some(id);
        }
        id
    }

    fn push_node(
        &mut self,
        item: &HirItem,
        source_anchor: PirSourceAnchor,
        operation: PirOperation,
        context: PirContext,
        dynamic_boundary: Option<PirId>,
    ) -> PirId {
        let id = PirId::from_index(self.next_id);
        self.next_id += 1;

        // Conservative intra-region control flow: link consecutive nodes that
        // share a scope with a fallthrough edge.
        let scope = item.scope_context;
        if let Some(previous) = self.last_in_scope.get(&scope).copied() {
            self.edges.push(PirEdge {
                from: previous,
                to: Some(id),
                kind: PirEdgeKind::Fallthrough,
            });
        }
        self.last_in_scope.insert(scope, id);

        self.nodes.push(PirNode {
            id,
            source_anchor,
            operation,
            context,
            dynamic_boundary,
            scope,
            package_context: item.package_context.clone(),
        });
        id
    }

    fn finish(self) -> PirGraph {
        // A DynamicCallee boundary is always consumed by the coderef call HIR
        // emits next, so nothing should be pending here. If a future HIR change
        // emits a boundary without its call, this catches the invariant break in
        // debug builds rather than silently leaving a boundary unlinked.
        debug_assert!(
            self.pending_dynamic_callee.is_none(),
            "pending_dynamic_callee was not consumed: HIR emitted a DynamicCallee \
             boundary without a following coderef CallExpr",
        );
        let receipt =
            build_receipt(&self.nodes, self.edges.len(), self.unsupported, self.source_identity);
        PirGraph { nodes: self.nodes, edges: self.edges, receipt }
    }
}

fn build_receipt(
    nodes: &[PirNode],
    edge_count: usize,
    unsupported: HashMap<&'static str, usize>,
    source_identity: Option<String>,
) -> PirReceipt {
    let mut operation_counts = std::collections::BTreeMap::new();
    let mut context_counts = std::collections::BTreeMap::new();
    let mut dynamic_boundary_counts = std::collections::BTreeMap::new();
    let mut coverage = PirAnchorCoverage::default();

    for node in nodes {
        *operation_counts.entry(node.operation.name()).or_insert(0) += 1;
        *context_counts.entry(node.context.name()).or_insert(0) += 1;
        if node.source_anchor.is_anchored() {
            coverage.anchored += 1;
        } else {
            coverage.unanchored += 1;
        }
        if let PirOperation::DynamicBoundary { kind, .. } = &node.operation {
            *dynamic_boundary_counts.entry(kind.name()).or_insert(0) += 1;
        }
    }

    let unsupported_construct_counts = unsupported.into_iter().collect();

    PirReceipt {
        schema_version: PIR_RECEIPT_VERSION,
        source_identity,
        lowering_mode: PirLoweringMode::HirV0,
        node_count: nodes.len(),
        edge_count,
        operation_counts,
        context_counts,
        source_anchor_coverage: coverage,
        dynamic_boundary_counts,
        unsupported_construct_counts,
        // PIR v0 lowering consumes only HIR; no ambient inputs participate.
        ambient_inputs: Vec::new(),
        // PIR v0 never changes provider behavior.
        provider_behavior_changed: false,
    }
}

fn is_stash_declarator(declarator: &str) -> bool {
    // `our` binds a package/stash symbol; `local` dynamically scopes a
    // package/global slot. Both are stash writes. `my`/`state` are lexical.
    matches!(declarator, "our" | "local")
}

fn named_callee(name: &str) -> PirCallee {
    match name.rsplit_once("::") {
        Some((package, bare)) if !package.is_empty() && !bare.is_empty() => {
            PirCallee::Named { name: bare.to_string(), package: Some(package.to_string()) }
        }
        _ => PirCallee::Named { name: name.to_string(), package: None },
    }
}

fn map_boundary_kind(kind: DynamicBoundaryKind) -> PirDynamicBoundaryKind {
    match kind {
        DynamicBoundaryKind::CoderefCall => PirDynamicBoundaryKind::DynamicCallee,
        DynamicBoundaryKind::EvalExpression => PirDynamicBoundaryKind::EvalExpression,
        DynamicBoundaryKind::DoExpression => PirDynamicBoundaryKind::DoExpression,
        DynamicBoundaryKind::DynamicStashMutation => PirDynamicBoundaryKind::RuntimeStashMutation,
        DynamicBoundaryKind::Autoload => PirDynamicBoundaryKind::Autoload,
        DynamicBoundaryKind::SymbolicReferenceDeref => PirDynamicBoundaryKind::SymbolicReference,
    }
}

fn hir_kind_name(kind: &HirKind) -> &'static str {
    match kind {
        HirKind::PackageDecl(_) => "PackageDecl",
        HirKind::SubDecl(_) => "SubDecl",
        HirKind::MethodDecl(_) => "MethodDecl",
        HirKind::UseDecl(_) => "UseDecl",
        HirKind::RequireDecl(_) => "RequireDecl",
        HirKind::VariableDecl(_) => "VariableDecl",
        HirKind::CallExpr(_) => "CallExpr",
        HirKind::MethodCallExpr(_) => "MethodCallExpr",
        HirKind::IndirectCallExpr(_) => "IndirectCallExpr",
        HirKind::BarewordExpr(_) => "BarewordExpr",
        HirKind::LiteralExpr(_) => "LiteralExpr",
        HirKind::BlockShell(_) => "BlockShell",
        // Control-flow variants added by #1902. PIR v0 does not yet lower
        // branch/loop/return — they fall through to unsupported_construct_counts
        // where the gap is visible in every receipt, consistent with the PR's
        // stated scope ("Branch/Loop/Return reserved but not yet populated").
        HirKind::BranchShell(_) => "BranchShell",
        HirKind::LoopShell(_) => "LoopShell",
        HirKind::ControlTransfer(_) => "ControlTransfer",
        HirKind::StatementModifierShell(_) => "StatementModifierShell",
        HirKind::DynamicBoundary(_) => "DynamicBoundary",
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::PirAnchorKind;
    use super::*;
    use crate::Parser;
    use crate::hir::lower_ast;
    use perl_tdd_support::must_some;

    fn lower(source: &str) -> PirGraph {
        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        let hir = lower_ast(&output.ast);
        lower_hir(&hir)
    }

    #[test]
    fn empty_source_yields_empty_graph() {
        let graph = lower("");
        assert!(graph.is_empty());
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.receipt.node_count, 0);
        assert_eq!(graph.receipt.edge_count, 0);
    }

    #[test]
    fn lexical_declaration_creates_write_and_assign() {
        let graph = lower("my $x = 1;");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].operation.name(), "LexicalWrite");
        assert_eq!(graph.nodes[0].context, PirContext::Lvalue);
        assert_eq!(graph.nodes[1].operation.name(), "Assign");
        assert_eq!(graph.nodes[1].context, PirContext::Void);
    }

    #[test]
    fn our_declaration_is_stash_write() {
        let graph = lower("package Acme; our @items = (1, 2);");
        let stash = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::StashWrite { .. })),
        );
        if let PirOperation::StashWrite { symbol } = &stash.operation {
            assert_eq!(symbol.sigil, "@");
            assert_eq!(symbol.name, "items");
            assert_eq!(symbol.package.as_deref(), Some("Acme"));
        } else {
            panic!("expected StashWrite");
        }
    }

    #[test]
    fn local_declaration_is_stash_write() {
        let graph = lower("local $x;");
        let stash = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::StashWrite { .. })),
        );
        if let PirOperation::StashWrite { symbol } = &stash.operation {
            assert_eq!(symbol.sigil, "$");
            assert_eq!(symbol.name, "x");
        } else {
            panic!("expected StashWrite");
        }
    }

    #[test]
    fn named_call_with_package_qualifier() {
        let graph = lower("Bar::baz();");
        let call = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Call { .. })),
        );
        if let PirOperation::Call { callee, arg_count } = &call.operation {
            assert_eq!(
                *callee,
                PirCallee::Named { name: "baz".to_string(), package: Some("Bar".to_string()) }
            );
            assert_eq!(*arg_count, 0);
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn deep_package_qualified_call() {
        let graph = lower("A::B::foo();");
        let call = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Call { .. })),
        );
        if let PirOperation::Call { callee, .. } = &call.operation {
            assert_eq!(
                *callee,
                PirCallee::Named { name: "foo".to_string(), package: Some("A::B".to_string()) }
            );
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn unqualified_call_has_no_package() {
        let graph = lower("foo(1, 2, 3);");
        let call = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Call { .. })),
        );
        if let PirOperation::Call { callee, arg_count } = &call.operation {
            assert_eq!(*callee, PirCallee::Named { name: "foo".to_string(), package: None });
            assert_eq!(*arg_count, 3);
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn method_call_preserves_method_and_args() {
        let graph = lower("$obj->frobnicate(1, 2);");
        let method = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::MethodCall { .. })),
        );
        if let PirOperation::MethodCall { method, arg_count, .. } = &method.operation {
            assert_eq!(*method, PirMethod::Named("frobnicate".to_string()));
            assert_eq!(*arg_count, 2);
        } else {
            panic!("expected MethodCall");
        }
    }

    #[test]
    fn coderef_call_links_to_dynamic_boundary() {
        let graph = lower("my $cb; $cb->(1);");
        let call = must_some(graph.nodes.iter().find(|n| {
            matches!(n.operation, PirOperation::Call { callee: PirCallee::Dynamic, .. })
        }));
        let boundary_id = must_some(call.dynamic_boundary);
        let boundary = must_some(graph.node(boundary_id));
        if let PirOperation::DynamicBoundary { kind, .. } = &boundary.operation {
            assert_eq!(*kind, PirDynamicBoundaryKind::DynamicCallee);
        } else {
            panic!("expected DynamicBoundary");
        }
    }

    #[test]
    fn multiple_coderef_calls_have_separate_boundaries() {
        let graph = lower("my ($a, $b); $a->(); $b->();");
        let calls: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| {
                matches!(n.operation, PirOperation::Call { callee: PirCallee::Dynamic, .. })
            })
            .collect();
        assert_eq!(calls.len(), 2);
        let b0 = must_some(calls[0].dynamic_boundary);
        let b1 = must_some(calls[1].dynamic_boundary);
        assert_ne!(b0, b1);
    }

    #[test]
    fn eval_creates_dynamic_boundary() {
        let graph = lower(r#"eval "$code";"#);
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("EvalExpression"), Some(&1));
    }

    #[test]
    fn symbolic_reference_creates_boundary() {
        let graph = lower("no strict 'refs'; my $v = ${$name};");
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("SymbolicReference"), Some(&1));
    }

    #[test]
    fn typeglob_creates_dynamic_boundary() {
        let graph = lower("*alias = $thing;");
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("RuntimeStashMutation"), Some(&1));
    }

    #[test]
    fn autoload_creates_dynamic_boundary() {
        let graph = lower("sub AUTOLOAD { }");
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("Autoload"), Some(&1));
    }

    #[test]
    fn dynamic_boundary_has_correct_anchor_kind() {
        let graph = lower(r#"eval "$code";"#);
        let boundary = must_some(
            graph
                .nodes
                .iter()
                .find(|n| matches!(n.operation, PirOperation::DynamicBoundary { .. })),
        );
        assert_eq!(boundary.source_anchor.kind, PirAnchorKind::DynamicBoundary);
    }

    #[test]
    fn all_nodes_have_source_anchors() {
        let graph = lower("package Foo; my $x = bar(); $obj->m(); our $y;");
        for node in &graph.nodes {
            assert!(node.source_anchor.is_anchored());
        }
        assert_eq!(graph.receipt.source_anchor_coverage.unanchored, 0);
    }

    #[test]
    fn branch_shell_counted_in_receipt() {
        let graph = lower("if (1) { 1 }");
        assert_eq!(graph.receipt.unsupported_construct_counts.get("BranchShell"), Some(&1));
    }

    #[test]
    fn loop_shell_counted_in_receipt() {
        let graph = lower("while (1) { last; }");
        assert_eq!(graph.receipt.unsupported_construct_counts.get("LoopShell"), Some(&1));
    }

    #[test]
    fn control_transfer_counted_in_receipt() {
        let graph = lower("sub f { return 1; }");
        assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), Some(&1));
    }

    #[test]
    fn statement_modifier_counted_in_receipt() {
        let graph = lower("$x = 1 if $y;");
        assert_eq!(
            graph.receipt.unsupported_construct_counts.get("StatementModifierShell"),
            Some(&1)
        );
    }

    #[test]
    fn all_four_control_flow_constructs() {
        let graph = lower(
            r#"
if (1) { 1 }
while (1) { last; }
sub f { return 1; }
$x = 1 if $y;
"#,
        );
        assert_eq!(graph.receipt.unsupported_construct_counts.get("BranchShell"), Some(&1));
        assert_eq!(graph.receipt.unsupported_construct_counts.get("LoopShell"), Some(&1));
        assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), Some(&2));
        assert_eq!(
            graph.receipt.unsupported_construct_counts.get("StatementModifierShell"),
            Some(&1)
        );
    }

    #[test]
    fn receipt_operation_counts_match_nodes() {
        let graph = lower("my $x = 1; foo(); $obj->m(); eval '1';");
        let op_total: usize = graph.receipt.operation_counts.values().sum();
        assert_eq!(op_total, graph.nodes.len());
        assert_eq!(graph.receipt.node_count, graph.nodes.len());
    }

    #[test]
    fn receipt_context_counts_match_nodes() {
        let graph = lower("my $x = 1; foo(); $obj->m(); eval '1';");
        let ctx_total: usize = graph.receipt.context_counts.values().sum();
        assert_eq!(ctx_total, graph.nodes.len());
    }

    #[test]
    fn fallthrough_edges_between_statements() {
        let graph = lower("foo(); bar(); baz();");
        let count = graph.edges.iter().filter(|e| e.kind == PirEdgeKind::Fallthrough).count();
        assert_eq!(count, 2);
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
    fn source_identity_threaded_to_receipt() {
        let mut parser = Parser::new("my $x = 1;");
        let output = parser.parse_with_recovery();
        let hir = lower_ast(&output.ast);
        let graph = lower_hir_with_identity(&hir, Some("fixture://demo.pl".to_string()));
        assert_eq!(graph.receipt.source_identity.as_deref(), Some("fixture://demo.pl"));
    }

    #[test]
    fn node_id_lookup_round_trips() {
        let graph = lower("my $x = 1;");
        for node in &graph.nodes {
            let found = must_some(graph.node(node.id));
            assert_eq!(found.id, node.id);
        }
    }

    #[test]
    fn multi_variable_declaration_creates_multiple_writes() {
        let graph = lower("my ($a, $b) = (1, 2);");
        let writes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| matches!(n.operation, PirOperation::LexicalWrite { .. }))
            .collect();
        assert_eq!(writes.len(), 2);
    }

    #[test]
    fn leading_colons_not_empty_package() {
        let graph = lower("::foo();");
        let call = must_some(
            graph.nodes.iter().find(|n| matches!(n.operation, PirOperation::Call { .. })),
        );
        if let PirOperation::Call { callee, .. } = &call.operation {
            assert_eq!(*callee, PirCallee::Named { name: "foo".to_string(), package: None });
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn receipt_edge_count_matches() {
        let graph = lower("my $x = 1; foo(); bar();");
        assert_eq!(graph.receipt.edge_count, graph.edges.len());
    }

    #[test]
    fn lowering_mode_is_hir_v0() {
        let graph = lower("my $x = 1;");
        assert_eq!(graph.receipt.lowering_mode, PirLoweringMode::HirV0);
    }

    #[test]
    fn unsupported_constructs_visible() {
        let graph = lower("package Foo; use strict; sub f {}");
        let unsupported = &graph.receipt.unsupported_construct_counts;
        assert_eq!(unsupported.get("PackageDecl"), Some(&1));
        assert_eq!(unsupported.get("UseDecl"), Some(&1));
        assert_eq!(unsupported.get("SubDecl"), Some(&1));
    }

    #[test]
    fn multiple_dynamic_boundary_types() {
        let graph = lower(
            r#"
eval "$code";
no strict 'refs'; ${$name};
*alias = $thing;
sub AUTOLOAD {}
"#,
        );
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("EvalExpression"), Some(&1));
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("SymbolicReference"), Some(&1));
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("RuntimeStashMutation"), Some(&1));
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("Autoload"), Some(&1));
    }

    #[test]
    fn provider_behavior_not_changed() {
        let graph = lower("my $x = 1;");
        assert!(!graph.receipt.provider_behavior_changed);
    }

    #[test]
    fn ambient_inputs_empty() {
        let graph = lower("my $x = 1;");
        assert!(graph.receipt.ambient_inputs.is_empty());
    }
}
