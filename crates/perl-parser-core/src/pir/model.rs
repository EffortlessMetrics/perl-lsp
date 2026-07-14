//! PIR v0 data model.
//!
//! PIR ("Perl intermediate representation") is a source-anchored tooling IR
//! lowered from [`HirFile`](crate::hir::HirFile). It is oriented around editor
//! tooling and static analysis, not bytecode execution: it preserves source
//! anchors, dynamic-boundary links, and expression context so later analyses
//! (control flow, dead code, safe delete, rename safety) have an honest base.
//!
//! PIR v0 is a compiler-substrate data layer only. It never evaluates Perl,
//! never runs `perldoc`/DAP/application code, never replaces HIR as the
//! canonical syntax tree, and never changes LSP provider behavior. The
//! authoritative contract is [`PLSP-SPEC-0025`](../../../../docs/specs/PLSP-SPEC-0025-pir-v0.md).

use crate::SourceLocation;
use crate::hir::{DerefAggregateKind, DerefOperandKind, HirId, HirScopeId};
use perl_semantic_facts::AnchorId;
use std::collections::BTreeMap;

/// Current PIR lowering-receipt schema version.
pub const PIR_RECEIPT_VERSION: u32 = 1;

/// Stable identifier for a PIR node within one lowering receipt.
///
/// IDs are internally deterministic for the same source, compiler environment,
/// and configuration. They are not guaranteed stable across versions or
/// unrelated workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct PirId {
    index: u32,
}

impl PirId {
    /// Create an identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Provenance class for a PIR node's source anchor.
///
/// Every source-derived node anchors to the workspace range that caused it.
/// Generated, framework, or ambient nodes only exist when their provenance is
/// explicit, so a provider can never mistake a modeled fact for source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PirAnchorKind {
    /// Node anchors directly to source text.
    ExplicitSource,
    /// Node anchors to a framework declaration, not a fabricated method body.
    SourceBackedGenerated,
    /// Generated node with no source backing; receipt-only.
    GeneratedNoSource,
    /// Node anchors to a dynamic boundary range when available.
    DynamicBoundary,
    /// Node reports an ambient-input class rather than source text.
    AmbientInput,
    /// Fallback node whose anchor could not be classified.
    Unknown,
}

impl PirAnchorKind {
    /// Stable name used in receipts and snapshots.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ExplicitSource => "ExplicitSource",
            Self::SourceBackedGenerated => "SourceBackedGenerated",
            Self::GeneratedNoSource => "GeneratedNoSource",
            Self::DynamicBoundary => "DynamicBoundary",
            Self::AmbientInput => "AmbientInput",
            Self::Unknown => "Unknown",
        }
    }

    /// Whether a node with this anchor kind is expected to carry a source range.
    #[must_use]
    pub const fn is_source_backed(self) -> bool {
        matches!(self, Self::ExplicitSource | Self::SourceBackedGenerated | Self::DynamicBoundary)
    }
}

/// Source anchor for a PIR node.
///
/// A source anchor records why a node exists and where it came from. Nodes
/// without a concrete range (generated-no-source, ambient, unknown) keep
/// `range` and `anchor_id` absent and explain themselves through `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirSourceAnchor {
    /// Provenance class for this anchor.
    pub kind: PirAnchorKind,
    /// Workspace source range that caused the node, when source-backed.
    pub range: Option<SourceLocation>,
    /// Stable anchor id derived from the source range, when source-backed.
    pub anchor_id: Option<AnchorId>,
    /// HIR item this node lowered from, when available.
    pub hir_item: Option<HirId>,
}

impl PirSourceAnchor {
    /// Build a source-backed anchor from a HIR item and its range.
    #[must_use]
    pub fn explicit(range: SourceLocation, hir_item: HirId) -> Self {
        Self {
            kind: PirAnchorKind::ExplicitSource,
            range: Some(range),
            anchor_id: Some(AnchorId(range.start as u64)),
            hir_item: Some(hir_item),
        }
    }

    /// Build a dynamic-boundary anchor pointing at the boundary range.
    #[must_use]
    pub fn dynamic_boundary(range: SourceLocation, hir_item: HirId) -> Self {
        Self {
            kind: PirAnchorKind::DynamicBoundary,
            range: Some(range),
            anchor_id: Some(AnchorId(range.start as u64)),
            hir_item: Some(hir_item),
        }
    }

    /// Return true when this anchor preserves a concrete source range.
    #[must_use]
    pub fn is_anchored(&self) -> bool {
        self.range.is_some()
    }
}

/// Expression context modeled by PIR v0.
///
/// Unknown context is allowed when the compiler substrate cannot prove context
/// without executing Perl. Unknown context is visible in receipts and is never
/// silently promoted to scalar or list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PirContext {
    /// Scalar context.
    Scalar,
    /// List context.
    List,
    /// Void context.
    Void,
    /// Lvalue (assignment-target) context.
    Lvalue,
    /// Context that cannot be proven statically.
    Unknown,
}

impl PirContext {
    /// Stable name used in receipts and snapshots.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Scalar => "Scalar",
            Self::List => "List",
            Self::Void => "Void",
            Self::Lvalue => "Lvalue",
            Self::Unknown => "Unknown",
        }
    }
}

/// A lexical (`my`/`state`) variable named by a PIR operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LexicalName {
    /// Variable sigil (`$`, `@`, `%`).
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
}

/// A package/stash symbol named by a PIR operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SymbolName {
    /// Symbol sigil when known.
    pub sigil: String,
    /// Symbol name without sigil.
    pub name: String,
    /// Package context active where the symbol is used, when known.
    pub package: Option<String>,
}

/// Callee of a PIR call operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PirCallee {
    /// A statically named callee, optionally package-qualified.
    Named {
        /// Callee name without the package qualifier.
        name: String,
        /// Package qualifier when the callee was written `Pkg::name`.
        package: Option<String>,
    },
    /// A coderef or otherwise dynamic callee; see the node's dynamic boundary.
    Dynamic,
}

/// Receiver of a PIR method-call operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PirReceiver {
    /// A bareword class receiver such as `Foo->new`.
    Class(String),
    /// An expression receiver; the field records the parser AST kind.
    Expression {
        /// Parser AST kind name for the receiver expression.
        kind: &'static str,
    },
    /// A dynamic receiver; see the node's dynamic boundary.
    Dynamic,
}

/// Method named by a PIR method-call operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PirMethod {
    /// A statically named method.
    Named(String),
    /// A dynamic method name; see the node's dynamic boundary.
    Dynamic,
}

/// Dynamic-boundary category preserved by PIR instead of guessing behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PirDynamicBoundaryKind {
    /// Coderef / dynamic callee call.
    DynamicCallee,
    /// Dynamic method receiver.
    DynamicReceiver,
    /// Dynamic (computed) method name.
    DynamicMethodName,
    /// Symbolic-reference dereference under disabled `strict refs`.
    SymbolicReference,
    /// Non-literal typeglob access or mutation.
    TypeglobAccess,
    /// Dynamic dereference boundary.
    DynamicDereference,
    /// Runtime stash/package-name mutation.
    RuntimeStashMutation,
    /// `eval` whose body is not a statically parsed block.
    EvalExpression,
    /// `do` whose body is not a statically parsed block.
    DoExpression,
    /// `AUTOLOAD`-driven dynamic dispatch.
    Autoload,
    /// Unclassified dynamic boundary.
    Unknown,
}

impl PirDynamicBoundaryKind {
    /// Stable name used in receipts and snapshots.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DynamicCallee => "DynamicCallee",
            Self::DynamicReceiver => "DynamicReceiver",
            Self::DynamicMethodName => "DynamicMethodName",
            Self::SymbolicReference => "SymbolicReference",
            Self::TypeglobAccess => "TypeglobAccess",
            Self::DynamicDereference => "DynamicDereference",
            Self::RuntimeStashMutation => "RuntimeStashMutation",
            Self::EvalExpression => "EvalExpression",
            Self::DoExpression => "DoExpression",
            Self::Autoload => "Autoload",
            Self::Unknown => "Unknown",
        }
    }
}

/// A modeled PIR operation.
///
/// Operations model data access, calls, and control flow without executing
/// Perl. Families that the current HIR substrate cannot prove (for example
/// branch and loop conditions, or explicit returns) are part of the contract
/// but are populated by later lowering passes; receipts make the gap visible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PirOperation {
    /// Read a lexical variable.
    LexicalRead {
        /// The lexical being read.
        name: LexicalName,
    },
    /// Write (declare or assign) a lexical variable.
    LexicalWrite {
        /// The lexical being written.
        name: LexicalName,
    },
    /// Read a package/stash symbol.
    StashRead {
        /// The symbol being read.
        symbol: SymbolName,
    },
    /// Write a package/stash symbol.
    StashWrite {
        /// The symbol being written.
        symbol: SymbolName,
    },
    /// Compound read-modify-write on a lexical variable (`+=`, `-=`, `++`, etc.).
    ///
    /// The place is evaluated exactly once. The compound operator is preserved in
    /// `op` so downstream analyses can distinguish `+=` from `++` without re-parsing.
    Modify {
        /// The lexical variable being modified.
        name: LexicalName,
        /// The compound operator text (`"+="`, `"-="`, `"*="`, `"++"`, `"--"`, etc.).
        op: String,
    },
    /// Compound read-modify-write on a package/stash symbol.
    ///
    /// Mirrors [`Modify`](Self::Modify) for package slots.
    StashModify {
        /// The package symbol being modified.
        symbol: SymbolName,
        /// The compound operator text.
        op: String,
    },
    /// An assignment expression.
    Assign,
    /// A subroutine or function call.
    Call {
        /// The callee.
        callee: PirCallee,
        /// Number of parsed arguments.
        arg_count: usize,
    },
    /// A method call.
    MethodCall {
        /// The receiver.
        receiver: PirReceiver,
        /// The method.
        method: PirMethod,
        /// Number of parsed arguments.
        arg_count: usize,
    },
    /// An aggregate or slot dereference whose target expression is preserved
    /// as a runtime-shaped operand rather than evaluated by PIR.
    Deref {
        /// Aggregate or slot selected by the dereference.
        aggregate_kind: DerefAggregateKind,
        /// Syntactic shape supplying the runtime target.
        operand_kind: DerefOperandKind,
    },
    /// A branch (condition is populated by later control-flow lowering).
    Branch {
        /// PIR node computing the branch condition, when modeled.
        condition: Option<PirId>,
    },
    /// A loop (condition is populated by later control-flow lowering).
    Loop {
        /// PIR node computing the loop condition, when modeled.
        condition: Option<PirId>,
    },
    /// A return from the enclosing subroutine.
    Return,
    /// A preserved dynamic boundary.
    DynamicBoundary {
        /// Boundary category.
        kind: PirDynamicBoundaryKind,
        /// Short human-readable reason for the boundary.
        reason: String,
    },
}

impl PirOperation {
    /// Stable operation-family name used in receipts and snapshots.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::LexicalRead { .. } => "LexicalRead",
            Self::LexicalWrite { .. } => "LexicalWrite",
            Self::StashRead { .. } => "StashRead",
            Self::StashWrite { .. } => "StashWrite",
            Self::Modify { .. } => "Modify",
            Self::StashModify { .. } => "StashModify",
            Self::Assign => "Assign",
            Self::Call { .. } => "Call",
            Self::MethodCall { .. } => "MethodCall",
            Self::Deref { .. } => "Deref",
            Self::Branch { .. } => "Branch",
            Self::Loop { .. } => "Loop",
            Self::Return => "Return",
            Self::DynamicBoundary { .. } => "DynamicBoundary",
        }
    }

    /// All operation-family names PIR v0 models.
    ///
    /// Receipts and status generators should use this list instead of keeping a
    /// separate copy of the current PIR operation surface.
    pub const ALL_OPERATION_NAMES: &[&'static str] = &[
        "Assign",
        "Branch",
        "Call",
        "Deref",
        "DynamicBoundary",
        "LexicalRead",
        "LexicalWrite",
        "Loop",
        "MethodCall",
        "Modify",
        "Return",
        "StashModify",
        "StashRead",
        "StashWrite",
    ];
}

/// One PIR node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirNode {
    /// Stable node id within the lowering receipt.
    pub id: PirId,
    /// Source anchor explaining why and where this node exists.
    pub source_anchor: PirSourceAnchor,
    /// Modeled operation.
    pub operation: PirOperation,
    /// Expression context, possibly `Unknown`.
    pub context: PirContext,
    /// Link to a dynamic-boundary node this operation defers to, when any.
    pub dynamic_boundary: Option<PirId>,
    /// HIR scope this node belongs to, when known.
    pub scope: Option<HirScopeId>,
    /// Package context active at this node, when known.
    pub package_context: Option<String>,
}

/// Control-flow edge category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PirEdgeKind {
    /// Straight-line fallthrough to the next node in the same region.
    Fallthrough,
    /// Edge taken by a branch arm.
    Branch,
    /// Edge taken by a loop back-edge or entry.
    Loop,
    /// Edge taken by a return.
    Return,
    /// Edge leaving the modeled graph through a dynamic boundary.
    DynamicExit,
    /// Conservative unknown edge that must not be dropped silently.
    Unknown,
}

impl PirEdgeKind {
    /// Stable name used in receipts and snapshots.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fallthrough => "Fallthrough",
            Self::Branch => "Branch",
            Self::Loop => "Loop",
            Self::Return => "Return",
            Self::DynamicExit => "DynamicExit",
            Self::Unknown => "Unknown",
        }
    }
}

/// One control-flow edge between PIR nodes.
///
/// A `to` of `None` represents an exit, unknown, or dynamic continuation that
/// must remain visible rather than be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirEdge {
    /// Source node.
    pub from: PirId,
    /// Destination node, or `None` for an exit/unknown continuation.
    pub to: Option<PirId>,
    /// Edge category.
    pub kind: PirEdgeKind,
}

/// Lowering pass that produced a PIR graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PirLoweringMode {
    /// First HIR-to-PIR v0 lowering pass.
    HirV0,
}

impl PirLoweringMode {
    /// Stable name used in receipts and snapshots.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::HirV0 => "HirV0",
        }
    }
}

/// Source-anchor coverage summary for a lowering receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PirAnchorCoverage {
    /// Nodes that preserved a concrete source range.
    pub anchored: usize,
    /// Nodes without a concrete source range (generated, ambient, unknown).
    pub unanchored: usize,
}

impl PirAnchorCoverage {
    /// Total nodes counted.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.anchored + self.unanchored
    }
}

/// A PIR lowering receipt.
///
/// Receipts explain what lowered, what fell back, and what was blocked. They
/// are the proof surface for PIR work and assert that provider behavior did not
/// change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirReceipt {
    /// Receipt schema version.
    pub schema_version: u32,
    /// Caller-supplied source file or workspace fixture identity.
    pub source_identity: Option<String>,
    /// Lowering mode that produced the graph.
    pub lowering_mode: PirLoweringMode,
    /// Number of PIR nodes.
    pub node_count: usize,
    /// Number of control-flow edges.
    pub edge_count: usize,
    /// Lowered operation counts, keyed by operation-family name.
    pub operation_counts: BTreeMap<&'static str, usize>,
    /// Context counts, keyed by context name.
    pub context_counts: BTreeMap<&'static str, usize>,
    /// Source-anchor coverage summary.
    pub source_anchor_coverage: PirAnchorCoverage,
    /// Dynamic-boundary counts, keyed by boundary-kind name.
    pub dynamic_boundary_counts: BTreeMap<&'static str, usize>,
    /// HIR constructs PIR v0 did not lower, keyed by HIR kind name.
    pub unsupported_construct_counts: BTreeMap<&'static str, usize>,
    /// Stale or ambient inputs that affected lowering.
    pub ambient_inputs: Vec<String>,
    /// Whether provider behavior changed. Always `false` under PIR v0.
    pub provider_behavior_changed: bool,
}

/// A lowered PIR graph plus its receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirGraph {
    /// PIR nodes in stable lowering order.
    pub nodes: Vec<PirNode>,
    /// Control-flow edges.
    pub edges: Vec<PirEdge>,
    /// Lowering receipt describing this graph.
    pub receipt: PirReceipt,
}

impl PirGraph {
    /// Return true when no PIR nodes were lowered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Look up a node by id.
    ///
    /// This is O(1): lowering pushes nodes in stable index order, so `nodes[k]`
    /// always has `id.index() == k`. Lowerings must preserve that invariant —
    /// do not store nodes out of order or assign non-sequential ids.
    #[must_use]
    pub fn node(&self, id: PirId) -> Option<&PirNode> {
        self.nodes.get(id.index() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pir_context_has_stable_names() {
        assert_eq!(PirContext::Scalar.name(), "Scalar");
        assert_eq!(PirContext::List.name(), "List");
        assert_eq!(PirContext::Void.name(), "Void");
        assert_eq!(PirContext::Lvalue.name(), "Lvalue");
        assert_eq!(PirContext::Unknown.name(), "Unknown");
    }

    #[test]
    fn pir_anchor_kind_has_stable_names() {
        assert_eq!(PirAnchorKind::ExplicitSource.name(), "ExplicitSource");
        assert_eq!(PirAnchorKind::SourceBackedGenerated.name(), "SourceBackedGenerated");
        assert_eq!(PirAnchorKind::GeneratedNoSource.name(), "GeneratedNoSource");
        assert_eq!(PirAnchorKind::DynamicBoundary.name(), "DynamicBoundary");
        assert_eq!(PirAnchorKind::AmbientInput.name(), "AmbientInput");
        assert_eq!(PirAnchorKind::Unknown.name(), "Unknown");
    }

    #[test]
    fn pir_anchor_kind_is_source_backed() {
        assert!(PirAnchorKind::ExplicitSource.is_source_backed());
        assert!(PirAnchorKind::SourceBackedGenerated.is_source_backed());
        assert!(!PirAnchorKind::GeneratedNoSource.is_source_backed());
        assert!(PirAnchorKind::DynamicBoundary.is_source_backed());
        assert!(!PirAnchorKind::AmbientInput.is_source_backed());
        assert!(!PirAnchorKind::Unknown.is_source_backed());
    }

    #[test]
    fn pir_source_anchor_explicit_creates_anchored() {
        let loc = SourceLocation { start: 0, end: 5 };
        let anchor = PirSourceAnchor::explicit(loc, HirId::from_index(1));
        assert!(anchor.is_anchored());
        assert_eq!(anchor.kind, PirAnchorKind::ExplicitSource);
        assert_eq!(anchor.range, Some(loc));
    }

    #[test]
    fn pir_source_anchor_dynamic_boundary_creates_anchored() {
        let loc = SourceLocation { start: 10, end: 20 };
        let anchor = PirSourceAnchor::dynamic_boundary(loc, HirId::from_index(2));
        assert!(anchor.is_anchored());
        assert_eq!(anchor.kind, PirAnchorKind::DynamicBoundary);
        assert_eq!(anchor.range, Some(loc));
    }

    #[test]
    fn pir_callee_named_equality() {
        let callee1 =
            PirCallee::Named { name: "foo".to_string(), package: Some("Bar".to_string()) };
        let callee2 =
            PirCallee::Named { name: "foo".to_string(), package: Some("Bar".to_string()) };
        assert_eq!(callee1, callee2);
    }

    #[test]
    fn pir_callee_dynamic_equality() {
        assert_eq!(PirCallee::Dynamic, PirCallee::Dynamic);
    }

    #[test]
    fn pir_method_named_equality() {
        assert_eq!(PirMethod::Named("foo".to_string()), PirMethod::Named("foo".to_string()));
    }

    #[test]
    fn pir_operation_has_all_names() {
        let expected = vec![
            "Assign",
            "Branch",
            "Call",
            "Deref",
            "DynamicBoundary",
            "LexicalRead",
            "LexicalWrite",
            "Loop",
            "MethodCall",
            "Modify",
            "Return",
            "StashModify",
            "StashRead",
            "StashWrite",
        ];
        let actual: Vec<_> = PirOperation::ALL_OPERATION_NAMES.to_vec();
        assert_eq!(actual, expected);
    }

    #[test]
    fn pir_operation_lexical_read_name() {
        let op = PirOperation::LexicalRead {
            name: LexicalName { sigil: "$".to_string(), name: "x".to_string() },
        };
        assert_eq!(op.name(), "LexicalRead");
    }

    #[test]
    fn pir_operation_lexical_write_name() {
        let op = PirOperation::LexicalWrite {
            name: LexicalName { sigil: "$".to_string(), name: "x".to_string() },
        };
        assert_eq!(op.name(), "LexicalWrite");
    }

    #[test]
    fn pir_operation_stash_read_name() {
        let op = PirOperation::StashRead {
            symbol: SymbolName { sigil: "$".to_string(), name: "x".to_string(), package: None },
        };
        assert_eq!(op.name(), "StashRead");
    }

    #[test]
    fn pir_operation_stash_write_name() {
        let op = PirOperation::StashWrite {
            symbol: SymbolName {
                sigil: "@".to_string(),
                name: "items".to_string(),
                package: Some("Acme".to_string()),
            },
        };
        assert_eq!(op.name(), "StashWrite");
    }

    #[test]
    fn pir_operation_assign_name() {
        let op = PirOperation::Assign;
        assert_eq!(op.name(), "Assign");
    }

    #[test]
    fn pir_operation_call_name() {
        let op = PirOperation::Call {
            callee: PirCallee::Named { name: "foo".to_string(), package: None },
            arg_count: 2,
        };
        assert_eq!(op.name(), "Call");
    }

    #[test]
    fn pir_operation_method_call_name() {
        let op = PirOperation::MethodCall {
            receiver: PirReceiver::Expression { kind: "Variable" },
            method: PirMethod::Named("foo".to_string()),
            arg_count: 1,
        };
        assert_eq!(op.name(), "MethodCall");
    }

    #[test]
    fn pir_operation_deref_name() {
        let op = PirOperation::Deref {
            aggregate_kind: DerefAggregateKind::Array,
            operand_kind: DerefOperandKind::Variable,
        };
        assert_eq!(op.name(), "Deref");
    }

    #[test]
    fn pir_operation_branch_name() {
        let op = PirOperation::Branch { condition: None };
        assert_eq!(op.name(), "Branch");
    }

    #[test]
    fn pir_operation_loop_name() {
        let op = PirOperation::Loop { condition: None };
        assert_eq!(op.name(), "Loop");
    }

    #[test]
    fn pir_operation_return_name() {
        let op = PirOperation::Return;
        assert_eq!(op.name(), "Return");
    }

    #[test]
    fn pir_operation_dynamic_boundary_name() {
        let op = PirOperation::DynamicBoundary {
            kind: PirDynamicBoundaryKind::DynamicCallee,
            reason: "test".to_string(),
        };
        assert_eq!(op.name(), "DynamicBoundary");
    }

    #[test]
    fn pir_dynamic_boundary_kind_has_stable_names() {
        assert_eq!(PirDynamicBoundaryKind::DynamicCallee.name(), "DynamicCallee");
        assert_eq!(PirDynamicBoundaryKind::DynamicReceiver.name(), "DynamicReceiver");
        assert_eq!(PirDynamicBoundaryKind::DynamicMethodName.name(), "DynamicMethodName");
        assert_eq!(PirDynamicBoundaryKind::SymbolicReference.name(), "SymbolicReference");
        assert_eq!(PirDynamicBoundaryKind::TypeglobAccess.name(), "TypeglobAccess");
        assert_eq!(PirDynamicBoundaryKind::DynamicDereference.name(), "DynamicDereference");
        assert_eq!(PirDynamicBoundaryKind::RuntimeStashMutation.name(), "RuntimeStashMutation");
        assert_eq!(PirDynamicBoundaryKind::EvalExpression.name(), "EvalExpression");
        assert_eq!(PirDynamicBoundaryKind::DoExpression.name(), "DoExpression");
        assert_eq!(PirDynamicBoundaryKind::Autoload.name(), "Autoload");
        assert_eq!(PirDynamicBoundaryKind::Unknown.name(), "Unknown");
    }

    #[test]
    fn pir_edge_kind_has_stable_names() {
        assert_eq!(PirEdgeKind::Fallthrough.name(), "Fallthrough");
        assert_eq!(PirEdgeKind::Branch.name(), "Branch");
        assert_eq!(PirEdgeKind::Loop.name(), "Loop");
        assert_eq!(PirEdgeKind::Return.name(), "Return");
        assert_eq!(PirEdgeKind::DynamicExit.name(), "DynamicExit");
        assert_eq!(PirEdgeKind::Unknown.name(), "Unknown");
    }

    #[test]
    fn pir_lowering_mode_has_stable_name() {
        assert_eq!(PirLoweringMode::HirV0.name(), "HirV0");
    }

    #[test]
    fn pir_anchor_coverage_total() {
        let coverage = PirAnchorCoverage { anchored: 5, unanchored: 3 };
        assert_eq!(coverage.total(), 8);
    }

    #[test]
    fn pir_anchor_coverage_default() {
        let coverage = PirAnchorCoverage::default();
        assert_eq!(coverage.anchored, 0);
        assert_eq!(coverage.unanchored, 0);
        assert_eq!(coverage.total(), 0);
    }

    #[test]
    fn pir_id_from_index_round_trip() {
        let id = PirId::from_index(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn pir_graph_empty_returns_true_for_no_nodes() {
        let graph = PirGraph {
            nodes: vec![],
            edges: vec![],
            receipt: PirReceipt {
                schema_version: 1,
                source_identity: None,
                lowering_mode: PirLoweringMode::HirV0,
                node_count: 0,
                edge_count: 0,
                operation_counts: Default::default(),
                context_counts: Default::default(),
                source_anchor_coverage: Default::default(),
                dynamic_boundary_counts: Default::default(),
                unsupported_construct_counts: Default::default(),
                ambient_inputs: vec![],
                provider_behavior_changed: false,
            },
        };
        assert!(graph.is_empty());
    }

    #[test]
    fn pir_graph_empty_returns_false_for_nodes() {
        let loc = SourceLocation { start: 0, end: 1 };
        let node = PirNode {
            id: PirId::from_index(0),
            source_anchor: PirSourceAnchor::explicit(loc, HirId::from_index(1)),
            operation: PirOperation::Assign,
            context: PirContext::Void,
            dynamic_boundary: None,
            scope: None,
            package_context: None,
        };
        let graph = PirGraph {
            nodes: vec![node],
            edges: vec![],
            receipt: PirReceipt {
                schema_version: 1,
                source_identity: None,
                lowering_mode: PirLoweringMode::HirV0,
                node_count: 1,
                edge_count: 0,
                operation_counts: Default::default(),
                context_counts: Default::default(),
                source_anchor_coverage: Default::default(),
                dynamic_boundary_counts: Default::default(),
                unsupported_construct_counts: Default::default(),
                ambient_inputs: vec![],
                provider_behavior_changed: false,
            },
        };
        assert!(!graph.is_empty());
    }

    #[test]
    fn pir_graph_node_lookup() {
        let loc = SourceLocation { start: 0, end: 1 };
        let node = PirNode {
            id: PirId::from_index(0),
            source_anchor: PirSourceAnchor::explicit(loc, HirId::from_index(1)),
            operation: PirOperation::Assign,
            context: PirContext::Void,
            dynamic_boundary: None,
            scope: None,
            package_context: None,
        };
        let graph = PirGraph {
            nodes: vec![node.clone()],
            edges: vec![],
            receipt: PirReceipt {
                schema_version: 1,
                source_identity: None,
                lowering_mode: PirLoweringMode::HirV0,
                node_count: 1,
                edge_count: 0,
                operation_counts: Default::default(),
                context_counts: Default::default(),
                source_anchor_coverage: Default::default(),
                dynamic_boundary_counts: Default::default(),
                unsupported_construct_counts: Default::default(),
                ambient_inputs: vec![],
                provider_behavior_changed: false,
            },
        };
        let found = graph.node(PirId::from_index(0));
        assert_eq!(found, Some(&node));
    }

    #[test]
    fn pir_graph_node_lookup_invalid_id() {
        let graph = PirGraph {
            nodes: vec![],
            edges: vec![],
            receipt: PirReceipt {
                schema_version: 1,
                source_identity: None,
                lowering_mode: PirLoweringMode::HirV0,
                node_count: 0,
                edge_count: 0,
                operation_counts: Default::default(),
                context_counts: Default::default(),
                source_anchor_coverage: Default::default(),
                dynamic_boundary_counts: Default::default(),
                unsupported_construct_counts: Default::default(),
                ambient_inputs: vec![],
                provider_behavior_changed: false,
            },
        };
        let found = graph.node(PirId::from_index(42));
        assert_eq!(found, None);
    }

    #[test]
    fn lexical_name_structure() {
        let name = LexicalName { sigil: "$".to_string(), name: "x".to_string() };
        assert_eq!(name.sigil, "$");
        assert_eq!(name.name, "x");
    }

    #[test]
    fn symbol_name_with_package() {
        let symbol = SymbolName {
            sigil: "@".to_string(),
            name: "items".to_string(),
            package: Some("Acme".to_string()),
        };
        assert_eq!(symbol.sigil, "@");
        assert_eq!(symbol.name, "items");
        assert_eq!(symbol.package.as_deref(), Some("Acme"));
    }

    #[test]
    fn pir_receiver_class() {
        let receiver = PirReceiver::Class("Foo".to_string());
        assert_eq!(format!("{:?}", receiver), "Class(\"Foo\")");
    }

    #[test]
    fn pir_receiver_expression() {
        let receiver = PirReceiver::Expression { kind: "Variable" };
        assert_eq!(format!("{:?}", receiver), "Expression { kind: \"Variable\" }");
    }

    #[test]
    fn pir_receiver_dynamic() {
        assert_eq!(format!("{:?}", PirReceiver::Dynamic), "Dynamic");
    }
}
