//! HIR-to-PIR v0 lowering.
//!
//! Lowering is intentionally conservative. It lowers the data-access, call, and
//! dynamic-boundary operation families that the current HIR substrate can prove
//! from source, anchors every source-derived node, preserves dynamic-boundary
//! links, and records everything it could not lower in the receipt. It never
//! evaluates Perl and never changes provider behavior.

use std::collections::HashMap;

use crate::hir::{
    AccessMode, AssignMode, BranchShell, CallForm, ControlTransferKind, DeclStorageClass,
    DynamicBoundaryKind, HIR_BODY_MODEL_VERSION, HirBody, HirBodyId, HirExpr, HirExprId, HirFile,
    HirItem, HirKind, HirScopeId, HirStmt, LoopShell, Sigil, UnaryMode, VariableKind,
};

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
            HirKind::BranchShell(branch) => self.lower_branch(item, branch),
            HirKind::LoopShell(loop_shell) => self.lower_loop(item, loop_shell),
            // Only the `return` verb lowers to PirOperation::Return. The other
            // ControlTransferKind verbs (next/last/redo/goto) are loop-control /
            // goto transfers, not subroutine returns; they fall through to the
            // `other` arm below and stay visible in unsupported_construct_counts
            // under the canonical `hir_kind_name` key — never mislabeled as a
            // return or dropped. Future #[non_exhaustive] verbs default to the
            // same safe, receipt-visible fallback.
            HirKind::ControlTransfer(transfer) if transfer.kind == ControlTransferKind::Return => {
                self.lower_return(item);
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

    fn lower_branch(&mut self, item: &HirItem, _branch: &BranchShell) {
        // Source anchor: explicit, backed by the BranchShell HIR item's range.
        let anchor = PirSourceAnchor::explicit(item.range, item.id);

        // `condition` is None: PIR v0 does not yet lower the condition
        // expression to a separate PIR node. Condition-expression lowering is a
        // named follow-up (see PLSP-SPEC-0025 §Control-Flow Model).
        let operation = PirOperation::Branch { condition: None };

        // Void context: an `if`/`unless` statement is a control-flow fork that
        // yields no value at statement level. Using Unknown here would over-
        // approximate; Scalar/List would misrepresent the node's role. The
        // condition sub-expression evaluates in scalar context, but that is the
        // condition's context — not the BranchShell node's context, which is the
        // whole statement.
        //
        // Arm-edge modeling (PirEdgeKind::Branch for then/else arms) is deferred
        // to a follow-up pass; this slice records the branch node and its
        // fallthrough without silently dropping it.
        self.push_node(item, anchor, operation, PirContext::Void, None);
    }

    fn lower_loop(&mut self, item: &HirItem, _loop_shell: &LoopShell) {
        // Source anchor: explicit, backed by the LoopShell HIR item's range.
        let anchor = PirSourceAnchor::explicit(item.range, item.id);

        // `condition` is None: PIR v0 does not yet lower the condition
        // expression to a separate PIR node. Condition-expression lowering is a
        // named follow-up (see PLSP-SPEC-0025 §Control-Flow Model), mirroring
        // the same deferral in lower_branch.
        let operation = PirOperation::Loop { condition: None };

        // Void context: a while/until/for/foreach statement is a control-flow
        // construct that yields no value at statement level. All LoopShell
        // surface forms (While, Until, CStyleFor, Foreach) are statements —
        // unlike BranchShell (which can cover value-producing ternaries), loops
        // are never expressions in Perl, so Void is correct for all of them.
        //
        // Loop back-edges (PirEdgeKind::Loop) are deferred to a follow-up pass;
        // this slice records the loop node and its conservative fallthrough
        // without silently dropping it.
        self.push_node(item, anchor, operation, PirContext::Void, None);
    }

    fn lower_return(&mut self, item: &HirItem) {
        // Source anchor: explicit, backed by the ControlTransfer HIR item's
        // range.
        let anchor = PirSourceAnchor::explicit(item.range, item.id);

        // `PirOperation::Return` is fieldless in PIR v0: the returned expression
        // (`return $x`) is not lowered to a separate PIR node, mirroring the
        // deferred condition lowering in lower_branch/lower_loop. The HIR
        // `has_value`/`label` fields are intentionally not consumed yet;
        // returned-value lowering is a named follow-up (see PLSP-SPEC-0025
        // §Control-Flow Model).
        let operation = PirOperation::Return;

        // Void context: a `return` statement yields no value at the statement
        // level — it transfers control out of the enclosing subroutine. The
        // returned expression carries its own context, not the Return node's.
        let id = self.push_node(item, anchor, operation, PirContext::Void, None);

        // A `return` is terminal: control leaves the enclosing subroutine and
        // does NOT fall through to the next statement. Record the Return exit
        // edge (mirroring the DynamicExit shape — `to: None` leaves the modeled
        // graph) and clear this scope's fallthrough source so later items in the
        // same scope are not linked by a spurious `Fallthrough` edge *from* the
        // return. This matters in two cases the conservative push_node linking
        // would otherwise get wrong: (1) `return foo();`, where HIR emits the
        // ControlTransfer item *before* the returned `CallExpr` sibling, and
        // (2) any statement following a `return` in the same scope. Modeling the
        // returned expression as a reachable operand (rather than an unlinked
        // sibling) is part of the deferred returned-expression lowering.
        self.edges.push(PirEdge { from: id, to: None, kind: PirEdgeKind::Return });
        self.last_in_scope.remove(&item.scope_context);
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
        HirKind::DerefExpr(_) => "DerefExpr",
        HirKind::BlockShell(_) => "BlockShell",
        // Control-flow variants: BranchShell lowered by #8196 (Branch op),
        // LoopShell lowered by #8196 (Loop op), ControlTransfer::Return lowered
        // by #8196 (Return op). Non-Return ControlTransfer verbs
        // (next/last/redo/goto) and StatementModifierShell are not lowered and
        // reach the `other =>` fallback above, which keys unsupported counts on
        // hir_kind_name — so this "ControlTransfer" arm is the single source of
        // that key (no duplicated literal). BranchShell/LoopShell/return do not
        // reach this fallback; their arms are retained for completeness
        // (hir_kind_name is also used by the BodyLowerer unsupported path).
        HirKind::BranchShell(_) => "BranchShell",
        HirKind::LoopShell(_) => "LoopShell",
        HirKind::ControlTransfer(_) => "ControlTransfer",
        HirKind::StatementModifierShell(_) => "StatementModifierShell",
        HirKind::DynamicBoundary(_) => "DynamicBoundary",
    }
}

/// Lower a single [`HirBody`] to PIR nodes, preserving body identity.
///
/// This is the engine for the lexical extractor: it processes one HirBody at a time,
/// yielding all PIR nodes emitted from that body without merging into a flat graph.
/// Body boundaries are preserved, enabling per-body analysis like scope isolation.
///
/// A fresh [`BodyLowerer`] is created for each call so state (fallthrough tracking,
/// node IDs) is isolated to the single body. The returned nodes are in lowering order.
///
/// Returns a `Vec` of [`PirNode`], in lowering order. Each node carries its source anchor.
#[must_use]
pub fn lower_single_body(body: &HirBody, body_id: HirBodyId, file: &HirFile) -> Vec<PirNode> {
    let mut lowerer = BodyLowerer::new(None);
    lowerer.lower_body(body, body_id, file);
    lowerer.nodes
}

// ── PIR-A: lower from canonical HirFile::bodies ───────────────────────────────
//
// This is the new canonical lowering path introduced in PR 2 (#2578). It lowers
// Read/Write/Modify operations directly from the body arenas attached to
// `HirFile::bodies` by `lower_ast()` (PR 1, #2575/#2602).
//
// The old `lower_hir` (flat-items path above) lowers from `HirFile::items` and
// is now dormant relative to body-based facts. It is retained for backward
// compatibility until its callers are migrated; once fully superseded it should
// be retired (#2578 follow-up).

/// Lower a [`HirFile`]'s canonical body arenas into a PIR-A graph.
///
/// Requires that `lower_ast()` has run — i.e. `file.body_model_version ==
/// HIR_BODY_MODEL_VERSION`. If the version check fails the returned graph is
/// empty and the receipt records the mismatch in `ambient_inputs`.
#[must_use]
pub fn lower_hir_bodies(file: &HirFile) -> PirGraph {
    lower_hir_bodies_with_identity(file, None)
}

/// Returns `true` iff `version` matches the current HIR body-model version.
///
/// Extracted as a pure predicate so the schema-version equality boundary is
/// independently unit-testable with *literal* version arguments (below / equal /
/// above) — a fixture-construction test that assigns the constant to a field
/// cannot expose this equality to static analysis.
#[inline]
#[must_use]
fn body_model_version_matches(version: u32) -> bool {
    version == HIR_BODY_MODEL_VERSION
}

/// Lower a [`HirFile`]'s canonical body arenas into a PIR-A graph, tagging the
/// receipt with an optional caller-supplied source or fixture identity.
#[must_use]
pub fn lower_hir_bodies_with_identity(file: &HirFile, source_identity: Option<String>) -> PirGraph {
    // Verifier rule: schema-version mismatch → empty graph.
    if !body_model_version_matches(file.body_model_version) {
        let receipt = PirReceipt {
            schema_version: PIR_RECEIPT_VERSION,
            source_identity,
            lowering_mode: PirLoweringMode::HirV0,
            node_count: 0,
            edge_count: 0,
            operation_counts: Default::default(),
            context_counts: Default::default(),
            source_anchor_coverage: Default::default(),
            dynamic_boundary_counts: Default::default(),
            unsupported_construct_counts: Default::default(),
            ambient_inputs: vec![format!(
                "body_model_version mismatch: expected {HIR_BODY_MODEL_VERSION}, got {}",
                file.body_model_version
            )],
            provider_behavior_changed: false,
        };
        return PirGraph { nodes: vec![], edges: vec![], receipt };
    }

    let mut lowerer = BodyLowerer::new(source_identity);
    for (body_idx, body) in file.bodies.iter().enumerate() {
        lowerer.lower_body(body, HirBodyId(body_idx as u32), file);
    }
    lowerer.finish()
}

/// Body-arena lowerer for PIR-A.
///
/// Walks `HirBody` arenas and emits `Read`/`Write`/`Modify` PIR operations.
/// Verifier rules are applied inline — any node that would emit a wrong fact
/// instead produces nothing (fail-closed).
struct BodyLowerer {
    nodes: Vec<PirNode>,
    edges: Vec<PirEdge>,
    next_id: u32,
    last_in_scope: HashMap<Option<HirScopeId>, PirId>,
    unsupported: HashMap<&'static str, usize>,
    source_identity: Option<String>,
}

impl BodyLowerer {
    fn new(source_identity: Option<String>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 0,
            last_in_scope: HashMap::new(),
            unsupported: HashMap::new(),
            source_identity,
        }
    }

    fn lower_body(&mut self, body: &HirBody, _body_id: HirBodyId, file: &HirFile) {
        // Clear the intra-scope fallthrough state between bodies. Without this,
        // the last node of body N would be connected by a spurious Fallthrough
        // edge to the first node of body N+1 — incorrect because bodies are
        // independent control-flow regions (sub bodies do not fall through into
        // the program root body or into each other).
        self.last_in_scope.clear();

        // Walk the root block's statements.
        if let Some(root_block) = body.block(body.root_block) {
            for stmt_id in &root_block.stmts {
                self.lower_stmt(body, *stmt_id, file);
            }
        }
    }

    fn lower_stmt(&mut self, body: &HirBody, stmt_id: crate::hir::HirStmtId, file: &HirFile) {
        let stmt = match body.stmt(stmt_id) {
            Some(s) => s,
            None => return,
        };
        match stmt {
            HirStmt::Let { name, sigil, storage, init } => {
                // Emit exactly ONE Write op for the declaration target.
                // `storage` determines whether this is a lexical (my/state) or
                // package (our) slot. Ignoring `storage` was the root cause of
                // BUG 1 (double Write), BUG 2 (our→wrong LexicalWrite), and
                // BUG 3 (spurious LexicalWrite alongside correct StashWrite).
                //
                // We emit the Write here from the declaration metadata, then
                // lower ONLY the RHS of the initialiser (not the HirExpr::Assign
                // wrapper, which would re-emit the LHS variable as a second Write).
                // Anchor the declaration write at the VARIABLE token (`$x`) to match
                // the legacy find-references / LSP anchoring, NOT the whole
                // `my $x = ...` statement span (issue #2640, PR3 range parity). The
                // variable's range is the LHS of the initialiser's `Assign`; fall back
                // to the statement range for declarations without an initialiser.
                // Resolve the initialiser to its `Assign` so its LHS variable range
                // can anchor the write. `body.expr` is folded into the `and_then` so
                // the `None` arm is reached by declarations WITHOUT an initialiser
                // (`my $x;` / `our $x;` → `init` is `None`), not just the unreachable
                // non-`Assign` case — keeping the arm exercised by tests.
                let var_range = match init.as_ref().and_then(|init_id| body.expr(*init_id)) {
                    Some(HirExpr::Assign { lhs, .. }) => {
                        body.source_map.expr_ranges.get(lhs.0 as usize).copied()
                    }
                    _ => None,
                };
                let range = var_range
                    .or_else(|| body.source_map.stmt_ranges.get(stmt_id.0 as usize).copied());
                if let Some(range) = range {
                    let anchor = self.make_body_anchor(range);
                    let op = match storage {
                        // `our` binds a package/stash symbol; `local` dynamically
                        // scopes a package/global slot. Both are stash writes.
                        DeclStorageClass::Our | DeclStorageClass::Local => {
                            PirOperation::StashWrite {
                                symbol: SymbolName {
                                    sigil: sigil_str(sigil),
                                    name: name.clone(),
                                    package: None, // package context not yet threaded into body arena
                                },
                            }
                        }
                        // my / state / any other declarator → lexical write
                        _ => PirOperation::LexicalWrite {
                            name: LexicalName { sigil: sigil_str(sigil), name: name.clone() },
                        },
                    };
                    self.push_body_node(anchor, op, PirContext::Lvalue, None, file);
                }
                // Lower the RHS of the initialiser. The init expr in the HIR body
                // is an HirExpr::Assign { lhs: Variable(Write), rhs, mode: Simple }.
                // We skip the Assign wrapper and lower only the rhs to avoid
                // re-emitting the LHS Variable as a second Write.
                if let Some(init_id) = init {
                    if let Some(HirExpr::Assign { rhs, .. }) = body.expr(*init_id) {
                        self.lower_expr(body, *rhs, file);
                    } else {
                        // Not an Assign node (shouldn't happen in well-formed HIR,
                        // but handle defensively — lower the init as-is).
                        self.lower_expr(body, *init_id, file);
                    }
                }
            }
            HirStmt::Expr(expr_id) => {
                self.lower_expr(body, *expr_id, file);
            }
        }
    }

    fn lower_expr(&mut self, body: &HirBody, expr_id: HirExprId, file: &HirFile) {
        let expr = match body.expr(expr_id) {
            Some(e) => e,
            None => return,
        };
        let range = body.source_map.expr_ranges.get(expr_id.0 as usize).copied();
        let range = match range {
            Some(r) => r,
            None => return, // no source range → fail-closed, emit nothing
        };

        match expr {
            HirExpr::Variable(v) => {
                self.lower_variable_expr(v, range, file);
            }

            HirExpr::Assign { lhs, rhs, mode } => {
                match mode {
                    AssignMode::Simple => {
                        // Lower LHS as a Write place, then RHS as a Read.
                        self.lower_expr(body, *lhs, file);
                        self.lower_expr(body, *rhs, file);
                        // Emit an Assign node spanning the whole expression.
                        let anchor = self.make_body_anchor(range);
                        self.push_body_node(
                            anchor,
                            PirOperation::Assign,
                            PirContext::Void,
                            None,
                            file,
                        );
                    }
                    AssignMode::ReadModifyWrite => {
                        // Compound assign: emit a single Modify node — place evaluated once.
                        // The LHS expr must be a Variable; if it isn't, fall through to unsupported.
                        if let Some(HirExpr::Variable(v)) = body.expr(*lhs) {
                            let op_text = compound_op_for_rmw_assign(body, *lhs);
                            self.lower_variable_modify(v, op_text, range, file);
                            // Lower RHS as a Read operand.
                            self.lower_expr(body, *rhs, file);
                        } else {
                            // Non-variable LHS for compound assign → unsupported (fail-closed).
                            *self.unsupported.entry("CompoundAssignNonVarLhs").or_insert(0) += 1;
                        }
                    }
                }
            }

            HirExpr::Unary { operand, mode, op } => {
                match mode {
                    UnaryMode::ReadModifyWrite => {
                        // `++`/`--` on a variable → Modify.
                        if let Some(HirExpr::Variable(v)) = body.expr(*operand) {
                            self.lower_variable_modify(v, op.clone(), range, file);
                        } else {
                            *self.unsupported.entry("UnaryRmwNonVar").or_insert(0) += 1;
                        }
                    }
                    UnaryMode::Read => {
                        self.lower_expr(body, *operand, file);
                    }
                }
            }

            HirExpr::Binary { lhs, rhs, op: _ } => {
                // Lower both operands as reads; the binary op itself is not modeled
                // in PIR-A (no CFG, no value tracking).
                self.lower_expr(body, *lhs, file);
                self.lower_expr(body, *rhs, file);
            }

            HirExpr::Opaque { ast_kind } => {
                // Fail-closed: opaque nodes never emit exact facts.
                *self.unsupported.entry(ast_kind_to_static(ast_kind)).or_insert(0) += 1;
            }

            HirExpr::Call { args, ast_kind: _ } => {
                // Record the call itself as unsupported — PIR-A does not yet model
                // calls from body arenas as named PIR nodes. However, we DO walk
                // the argument expressions so that variable reads in call-arg position
                // (e.g. `print $x`, `return $x`) correctly produce LexicalRead nodes.
                *self.unsupported.entry("Call").or_insert(0) += 1;
                for arg_id in args {
                    self.lower_expr(body, *arg_id, file);
                }
            }
        }
    }

    /// Emit a Read or Write PIR node for a `HirVariable`, respecting its `VariableKind`.
    ///
    /// # Why `AccessMode::ReadModifyWrite` is not handled here
    ///
    /// `lower_variable_expr` is only called from `lower_expr` when it encounters a
    /// standalone `HirExpr::Variable` node. In the current HIR design, a `Variable`
    /// node with `access == ReadModifyWrite` only ever appears as:
    ///
    /// - The LHS of `HirExpr::Assign { mode: ReadModifyWrite }` — handled by extracting
    ///   the variable and calling `lower_variable_modify` directly; `lower_expr(lhs)` is
    ///   never called, so the Variable node itself never reaches `lower_expr`.
    /// - The operand of `HirExpr::Unary { mode: ReadModifyWrite }` — same pattern:
    ///   `lower_variable_modify` is called directly.
    ///
    /// Therefore this function only needs to handle `Read` and `Write` access. If a
    /// future HIR change routes an RMW Variable here, the early-return below ensures
    /// fail-closed behaviour (no wrong fact emitted, gap recorded in the receipt).
    fn lower_variable_expr(
        &mut self,
        v: &crate::hir::HirVariable,
        range: crate::SourceLocation,
        file: &HirFile,
    ) {
        // Fail-closed guard: RMW variables are resolved through lower_variable_modify,
        // not through this function. If HIR ever routes one here, emit nothing.
        if v.access == AccessMode::ReadModifyWrite {
            *self.unsupported.entry("RmwVariableFallthrough").or_insert(0) += 1;
            return;
        }

        let anchor = self.make_body_anchor(range);
        let sigil = sigil_str(&v.sigil);
        // At this point access is Read or Write (RMwW filtered above).
        let op = match &v.kind {
            VariableKind::Lexical if v.access == AccessMode::Read => {
                PirOperation::LexicalRead { name: LexicalName { sigil, name: v.name.clone() } }
            }
            VariableKind::Lexical => {
                PirOperation::LexicalWrite { name: LexicalName { sigil, name: v.name.clone() } }
            }
            VariableKind::Package if v.access == AccessMode::Read => PirOperation::StashRead {
                symbol: SymbolName {
                    sigil,
                    name: v.name.clone(),
                    package: package_from_name(&v.name),
                },
            },
            VariableKind::Package => PirOperation::StashWrite {
                symbol: SymbolName {
                    sigil,
                    name: v.name.clone(),
                    package: package_from_name(&v.name),
                },
            },
        };
        self.push_body_node(anchor, op, PirContext::Unknown, None, file);
    }

    /// Emit a Modify (or StashModify) PIR node for a compound-assign or `++`/`--`.
    fn lower_variable_modify(
        &mut self,
        v: &crate::hir::HirVariable,
        op_text: String,
        range: crate::SourceLocation,
        file: &HirFile,
    ) {
        let anchor = self.make_body_anchor(range);
        let sigil = sigil_str(&v.sigil);
        let op = match &v.kind {
            VariableKind::Lexical => PirOperation::Modify {
                name: LexicalName { sigil, name: v.name.clone() },
                op: op_text,
            },
            VariableKind::Package => PirOperation::StashModify {
                symbol: SymbolName {
                    sigil,
                    name: v.name.clone(),
                    package: package_from_name(&v.name),
                },
                op: op_text,
            },
        };
        self.push_body_node(anchor, op, PirContext::Unknown, None, file);
    }

    fn make_body_anchor(&self, range: crate::SourceLocation) -> PirSourceAnchor {
        // Body nodes don't have a HirId — they come from the body arena, not the
        // flat items list. Use a synthetic HirId(0) as a placeholder; the range
        // and anchor_id carry the meaningful identity.
        use crate::hir::HirId;
        PirSourceAnchor {
            kind: super::model::PirAnchorKind::ExplicitSource,
            range: Some(range),
            anchor_id: Some(perl_semantic_facts::AnchorId(range.start as u64)),
            hir_item: Some(HirId::from_index(0)),
        }
    }

    fn push_body_node(
        &mut self,
        source_anchor: PirSourceAnchor,
        operation: PirOperation,
        context: PirContext,
        dynamic_boundary: Option<PirId>,
        file: &HirFile,
    ) -> PirId {
        let id = PirId::from_index(self.next_id);
        self.next_id += 1;

        // Conservative intra-body fallthrough edges (scope = None for body nodes,
        // since body arenas don't carry HirScopeId per-node in this slice).
        let scope: Option<HirScopeId> = None;
        if let Some(previous) = self.last_in_scope.get(&scope).copied() {
            self.edges.push(PirEdge {
                from: previous,
                to: Some(id),
                kind: PirEdgeKind::Fallthrough,
            });
        }
        self.last_in_scope.insert(scope, id);

        let _ = file; // reserved for future scope/package lookup from ScopeGraph
        self.nodes.push(PirNode {
            id,
            source_anchor,
            operation,
            context,
            dynamic_boundary,
            scope,
            package_context: None, // deferred: body arenas don't carry package_context yet
        });
        id
    }

    fn finish(self) -> PirGraph {
        let receipt =
            build_receipt(&self.nodes, self.edges.len(), self.unsupported, self.source_identity);
        PirGraph { nodes: self.nodes, edges: self.edges, receipt }
    }
}

// ── Helpers for PIR-A body lowering ──────────────────────────────────────────

fn sigil_str(sigil: &Sigil) -> String {
    match sigil {
        Sigil::Scalar => "$".to_string(),
        Sigil::Array => "@".to_string(),
        Sigil::Hash => "%".to_string(),
        Sigil::Code => "&".to_string(),
        Sigil::Glob => "*".to_string(),
    }
}

/// Extract package prefix from a qualified name (`Foo::x` → `Some("Foo")`).
///
/// Leading-`::` names (e.g. `::foo`) split into `("", "foo")`. The empty-string
/// package half is filtered out so the result is `None` rather than `Some("")`,
/// which would be a confusing artifact. Bare-name is the conservative fallback.
fn package_from_name(name: &str) -> Option<String> {
    name.rsplit_once("::").and_then(
        |(pkg, _)| {
            if pkg.is_empty() { None } else { Some(pkg.to_string()) }
        },
    )
}

/// Map a known-bad `ast_kind` string to a static str for the unsupported counter.
fn ast_kind_to_static(kind: &str) -> &'static str {
    // We can't return the dynamic string as a static ref; map to a small set
    // of known opaque kinds. Everything else maps to "OpaqueExpr".
    match kind {
        "ExpressionStatement" => "OpaqueExpressionStatement",
        "FunctionCall" | "Call" => "OpaqueCall",
        "MethodCall" => "OpaqueMethodCall",
        _ => "OpaqueExpr",
    }
}

/// Return the compound operator text for the LHS variable node of an
/// `AssignMode::ReadModifyWrite` assignment.
///
/// HIR stores the access mode on the variable node but not the original
/// operator string at the Assign level (the body lowerer peels the AST `op`
/// into `AssignMode` only). For PIR-A receipts we emit a reasonable default;
/// the exact text is available in the HIR body source map if needed later.
fn compound_op_for_rmw_assign(_body: &HirBody, _lhs_id: HirExprId) -> String {
    // The operator string was recorded in the AST but is not yet threaded
    // through the HirExpr::Assign variant. Use a conservative placeholder.
    // A future HIR body revision can add `op: Option<String>` to Assign to
    // forward the exact text; until then receipts show "compound" for the op.
    "compound".to_string()
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
    fn body_model_version_matches_pins_equality_boundary() {
        // Literal-argument boundary test for the schema-version predicate that
        // guards `lower_hir_bodies_with_identity`. Passing the version directly
        // pins below / equal / above so a mutation removing or flipping the
        // equality is caught — and the discriminator value is statically visible.
        assert!(body_model_version_matches(HIR_BODY_MODEL_VERSION), "exact version must match");
        assert!(
            !body_model_version_matches(HIR_BODY_MODEL_VERSION - 1),
            "below-threshold version must not match"
        );
        assert!(
            !body_model_version_matches(HIR_BODY_MODEL_VERSION + 1),
            "above-threshold version must not match"
        );
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
    fn decl_var_range_match_covers_init_and_no_init_arms() {
        // Drive lower_single_body (the declaration write path) over an initialised
        // AND a bare declaration, so the var-range match in lower_stmt is fully
        // exercised by --lib coverage: `my $x = 1;` hits the `Some(Assign)` arm
        // (LHS range lookup), `my $x;` hits the `None` arm (statement fallback).
        use crate::hir::{HirBodyId, lower_ast};
        for src in ["my $x = 1;", "my $x;"] {
            let mut parser = crate::Parser::new(src);
            let output = parser.parse_with_recovery();
            let file = lower_ast(&output.ast);
            let mut saw_write = false;
            for (idx, body) in file.bodies.iter().enumerate() {
                for node in super::lower_single_body(body, HirBodyId(idx as u32), &file) {
                    if matches!(node.operation, PirOperation::LexicalWrite { .. }) {
                        saw_write = true;
                    }
                }
            }
            assert!(saw_write, "expected a LexicalWrite for `{src}`");
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
    fn symbolic_string_reference_creates_boundary() {
        let graph = lower("no strict 'refs'; my $v = ${\"name\"};");
        assert_eq!(graph.receipt.dynamic_boundary_counts.get("SymbolicReference"), Some(&1));
    }

    #[test]
    fn ordinary_runtime_reference_does_not_create_boundary() {
        let graph = lower("no strict 'refs'; my $v = ${$name};");
        assert!(graph.receipt.dynamic_boundary_counts.get("SymbolicReference").is_none());
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
    fn branch_shell_lowers_to_branch_operation() {
        // Since #8196, BranchShell lowers to PirOperation::Branch.
        let graph = lower("if (1) { 1 }");
        // BranchShell is now lowered — not in unsupported_construct_counts.
        assert_eq!(graph.receipt.unsupported_construct_counts.get("BranchShell"), None);
        // The Branch operation must appear in operation_counts.
        assert_eq!(graph.receipt.operation_counts.get("Branch"), Some(&1));
    }

    #[test]
    fn loop_shell_lowers_to_loop_operation() {
        // Since #8196, LoopShell lowers to PirOperation::Loop.
        let graph = lower("while (1) { last; }");
        // LoopShell is now lowered — not in unsupported_construct_counts.
        assert_eq!(graph.receipt.unsupported_construct_counts.get("LoopShell"), None);
        // The Loop operation must appear in operation_counts.
        assert_eq!(graph.receipt.operation_counts.get("Loop"), Some(&1));
    }

    #[test]
    fn control_transfer_return_lowers_to_return_operation() {
        // Since #8196, ControlTransferKind::Return lowers to PirOperation::Return.
        let graph = lower("sub f { return 1; }");
        // The return is no longer an unsupported construct.
        assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), None);
        // The Return operation must appear in operation_counts.
        assert_eq!(graph.receipt.operation_counts.get("Return"), Some(&1));
    }

    #[test]
    fn non_return_control_transfer_stays_unsupported() {
        // `last`/`next`/`redo`/`goto` are not subroutine returns; they remain
        // visible in unsupported_construct_counts rather than lowering to Return.
        let graph = lower("while (1) { last; }");
        assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), Some(&1));
        assert_eq!(graph.receipt.operation_counts.get("Return"), None);
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
        // BranchShell now lowers to Branch (#8196) — not in unsupported.
        assert_eq!(graph.receipt.unsupported_construct_counts.get("BranchShell"), None);
        assert_eq!(graph.receipt.operation_counts.get("Branch"), Some(&1));
        // LoopShell now lowers to Loop (#8196) — not in unsupported.
        assert_eq!(graph.receipt.unsupported_construct_counts.get("LoopShell"), None);
        assert_eq!(graph.receipt.operation_counts.get("Loop"), Some(&1));
        // ControlTransferKind::Return now lowers to Return (#8196). The fixture
        // has one `return` (in sub f) and one `last` (in the while loop): the
        // return becomes a Return op, the last stays an unsupported transfer.
        assert_eq!(graph.receipt.operation_counts.get("Return"), Some(&1));
        assert_eq!(graph.receipt.unsupported_construct_counts.get("ControlTransfer"), Some(&1));
        // StatementModifierShell is still unsupported.
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
no strict 'refs'; @{'Symbolic::values'};
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
