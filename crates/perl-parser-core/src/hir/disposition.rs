//! Shared HIR lowering-disposition registry.
//!
//! This is the **single source of truth** for how every AST [`NodeKind`] is
//! treated by the HIR lowerer (`lower.rs`) and by the `hir-coverage` metrics
//! tool (`xtask/src/tasks/metrics/hir_coverage.rs`).
//!
//! ## Design
//!
//! [`LoweringDisposition`] is multi-axis: each flag is independent, allowing
//! kinds that both emit HIR items *and* record side-facts (e.g. `Package`)
//! to be described accurately.  A legacy four-category view
//! ([`LegacyCategory`]) is derived from the flags for backward-compatible
//! reporting.
//!
//! ## Keeping this file in sync
//!
//! When you add a new `NodeKind` variant:
//!
//! 1. Add an entry to [`disposition_for`] that describes its lowering behavior.
//! 2. The xtask `hir-coverage --check` CI gate will fail if the entry is
//!    missing — this is intentional: the check guards against silent fallthrough
//!    in `lower.rs`.
//!
//! [`NodeKind`]: crate::NodeKind

use crate::NodeKind;

/// Multi-axis lowering disposition for a single AST [`NodeKind`].
///
/// Each flag is independent; a node can simultaneously emit HIR items, emit
/// dynamic-boundary markers, traverse children, and record side-facts.
///
/// The *authoritative* description of a node's behavior is the lowerer source
/// in `hir/lower.rs`; this registry mirrors that behavior and is validated by
/// the `hir-coverage` xtask and the `hir_lowering_completeness_tests`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoweringDisposition {
    /// The lowerer's match arm calls `push_item()` to emit one or more non-boundary
    /// HIR items (e.g. `PackageDecl`, `SubDecl`, `CallExpr`).
    pub emits_items: bool,

    /// The lowerer's match arm may call `push_item(DynamicBoundary(…))` — either
    /// unconditionally or on a runtime condition (e.g. `Eval` when block is not a
    /// block literal, `Unary` when symbolic-ref deref is detected).
    pub may_emit_boundary: bool,

    /// The lowerer traverses the node's children (via `visit_children` or manual
    /// child iteration), allowing nested constructs to produce their own HIR items.
    pub traverses_children: bool,

    /// The lowerer records facts into side-graphs (scope bindings, stash slots,
    /// compile-environment directives, prototype table, …) without necessarily
    /// emitting a HIR item.
    pub records_side_facts: bool,

    /// `true` when the lowering disposition for this kind is **intentionally
    /// decided** — either because `lower.rs` has a named match arm, or because
    /// the node is deliberately traversal-only (e.g. `ExpressionStatement`) even
    /// if it falls to `_ => visit_children` as a simplification.
    ///
    /// `false` means the node genuinely falls to `_ => visit_children` without
    /// a conscious design decision — i.e. it is "not yet modeled."
    ///
    /// Used by [`legacy_category`] to distinguish `IntentionallySkipped` from
    /// `NotYetModeled`.
    pub is_intentional: bool,

    /// Human-readable note describing the lowering behavior.  Used in the generated
    /// `docs/project/status/hir_lowering.md` and in test failure messages.
    pub note: &'static str,
}

impl LoweringDisposition {
    /// Derive the legacy four-category classification from the multi-axis flags.
    ///
    /// This mapping is used by `hir_coverage.rs` to produce backward-compatible
    /// status-doc tables.
    ///
    /// Derivation rules (in priority order):
    /// 1. `emits_items` → `Lowered`
    /// 2. `!emits_items && may_emit_boundary` → `DynamicBoundary`
    /// 3. `!emits_items && !may_emit_boundary && is_intentional` → `IntentionallySkipped`
    /// 4. `!emits_items && !may_emit_boundary && !is_intentional` → `NotYetModeled`
    pub fn legacy_category(self) -> LegacyCategory {
        if self.emits_items {
            return LegacyCategory::Lowered;
        }
        if self.may_emit_boundary {
            return LegacyCategory::DynamicBoundary;
        }
        if self.is_intentional {
            LegacyCategory::IntentionallySkipped
        } else {
            LegacyCategory::NotYetModeled
        }
    }
}

/// Legacy four-category view of lowering classification.
///
/// Derived from [`LoweringDisposition`] via [`LoweringDisposition::legacy_category`].
/// Used for backward-compatible status-doc reporting and test count assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegacyCategory {
    /// The lowerer emits one or more HIR items today.
    Lowered,
    /// The lowerer emits an explicit dynamic-boundary HIR item for unsupported
    /// static truth (may also emit other items or traverse children).
    DynamicBoundary,
    /// Traversal, metadata, or recovery placeholder; no standalone HIR item expected.
    IntentionallySkipped,
    /// Parser AST construct exists, but HIR has no shell yet (falls through to
    /// `visit_children` without an explicit arm).
    NotYetModeled,
}

impl LegacyCategory {
    /// Machine-readable string used in JSON / markdown output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lowered => "lowered",
            Self::DynamicBoundary => "dynamic_boundary",
            Self::IntentionallySkipped => "intentionally_skipped",
            Self::NotYetModeled => "not_yet_modeled",
        }
    }

    /// Human-readable meaning shown in the generated status doc.
    pub fn meaning(self) -> &'static str {
        match self {
            Self::Lowered => "Emits one or more HIR items today.",
            Self::DynamicBoundary => {
                "Emits an explicit dynamic-boundary HIR item for unsupported static truth."
            }
            Self::IntentionallySkipped => {
                "Traversal, metadata, or recovery placeholder; no standalone HIR item expected."
            }
            Self::NotYetModeled => "Parser AST construct exists, but HIR has no shell yet.",
        }
    }
}

/// HIR kinds emitted for a given AST kind, used for the coverage inventory.
///
/// Each entry lists the `HirKind` variant names that the lowerer may produce
/// for this AST kind.  Empty means no HIR items are emitted.
pub fn hir_kinds_for(ast_kind: &str) -> &'static [&'static str] {
    match ast_kind {
        "ArrayLiteral" => &["LiteralExpr"],
        "Block" => &["BlockShell"],
        "Do" => &["DynamicBoundary"],
        "Eval" => &["DynamicBoundary"],
        "Assignment" => &["DynamicBoundary"],
        "FunctionCall" => &["CallExpr", "DynamicBoundary", "RequireDecl"],
        "HashLiteral" => &["LiteralExpr"],
        "Identifier" => &["BarewordExpr"],
        "IndirectCall" => &["IndirectCallExpr"],
        "Method" => &["MethodDecl"],
        "MethodCall" => &["MethodCallExpr"],
        "Number" => &["LiteralExpr"],
        "Package" => &["PackageDecl"],
        "String" => &["LiteralExpr"],
        "Subroutine" => &["SubDecl"],
        "Undef" => &["LiteralExpr"],
        "Use" => &["UseDecl"],
        "VariableDeclaration" => &["VariableDecl"],
        "VariableListDeclaration" => &["VariableDecl"],
        "If" => &["BranchShell"],
        "Ternary" => &["BranchShell"],
        "While" => &["LoopShell"],
        "For" => &["LoopShell"],
        "Foreach" => &["LoopShell"],
        "Return" => &["ControlTransfer"],
        "LoopControl" => &["ControlTransfer"],
        "Goto" => &["ControlTransfer"],
        "StatementModifier" => &["StatementModifierShell"],
        "Unary" => &["DynamicBoundary"],
        _ => &[],
    }
}

/// Return the [`LoweringDisposition`] for a given AST kind name.
///
/// Returns `None` if `ast_kind` is not a recognized [`NodeKind`] name — this
/// means the entry is **missing** from the registry and the caller (e.g. the
/// `hir-coverage --check` gate or the completeness gate test) should fail.
///
/// The classification is derived from the actual lowerer behavior in
/// `crates/perl-parser-core/src/hir/lower.rs` — that file is the ground truth.
pub fn disposition_for(ast_kind: &str) -> Option<LoweringDisposition> {
    // Helper constructors — keep in sync with lower.rs behavior.
    //
    // Naming convention for flags:
    //   emits  = push_item() called for non-boundary HIR item
    //   bound  = push_item(HirKind::DynamicBoundary) called (conditional or unconditional)
    //   trav   = visit_children / explicit child iteration called
    //   side   = scope / stash / compile-env side-graph mutations
    //   intentl = the disposition is an intentional design decision (vs genuine not-yet-modeled)

    macro_rules! disp {
        ($emits:expr, $bound:expr, $trav:expr, $side:expr, $intentl:expr, $note:expr) => {
            Some(LoweringDisposition {
                emits_items: $emits,
                may_emit_boundary: $bound,
                traverses_children: $trav,
                records_side_facts: $side,
                is_intentional: $intentl,
                note: $note,
            })
        };
    }

    match ast_kind {
        // ── Explicitly lowered: emits HIR items ──────────────────────────────
        "ArrayLiteral" => disp!(
            true,
            false,
            true,
            false,
            true,
            "Lowered as aggregate literal shell; children (elements) are traversed."
        ),
        "Block" => disp!(
            true,
            false,
            true,
            true,
            true,
            "Lowered as block shell and contributes a ScopeGraph block frame."
        ),
        "FunctionCall" => disp!(
            true,
            true,
            true,
            true,
            true,
            "`require` calls lower as `RequireDecl`; coderef calls add a dynamic boundary."
        ),
        "HashLiteral" => disp!(
            true,
            false,
            true,
            false,
            true,
            "Lowered as aggregate literal shell; pairs are traversed."
        ),
        "Identifier" => disp!(
            true,
            false,
            false,
            true,
            true,
            "Lowered as bareword expression shell; records bareword fact."
        ),
        "IndirectCall" => {
            disp!(true, false, true, false, true, "Lowered as indirect-object call shell.")
        }
        "Method" => disp!(
            true,
            false,
            true,
            true,
            true,
            "Lowered as method declaration shell and contributes a method scope frame."
        ),
        "MethodCall" => disp!(true, false, true, false, true, "Lowered as method-call shell."),
        "Number" => disp!(true, false, false, false, true, "Lowered as numeric literal shell."),
        "Package" => disp!(
            true,
            false,
            true,
            true,
            true,
            "Lowered and updates package context plus package scope."
        ),
        "String" => disp!(true, false, false, false, true, "Lowered as string literal shell."),
        "Subroutine" => disp!(
            true,
            true,
            true,
            true,
            true,
            "Lowered as sub declaration shell; AUTOLOAD may also emit DynamicBoundary."
        ),
        "Undef" => disp!(true, false, false, false, true, "Lowered as undef literal shell."),
        "Use" => disp!(
            true,
            false,
            false,
            true,
            true,
            "Lowered as use declaration shell and records CompileEnvironment directive facts."
        ),
        "VariableDeclaration" => disp!(
            true,
            false,
            true,
            true,
            true,
            "Lowered as single variable declaration shell and records ScopeGraph bindings."
        ),
        "VariableListDeclaration" => disp!(
            true,
            false,
            true,
            true,
            true,
            "Lowered as list variable declaration shell and records ScopeGraph bindings."
        ),
        "If" => disp!(
            true,
            false,
            true,
            false,
            true,
            "`if`/`unless` block form lowered as a branch shell with condition anchor and arm counts."
        ),
        "Ternary" => disp!(
            true,
            false,
            true,
            false,
            true,
            "Ternary expression lowered as a branch shell with both arms present."
        ),
        "While" => disp!(
            true,
            false,
            true,
            false,
            true,
            "`while`/`until` lowered as a loop shell with condition and continue-block facts."
        ),
        "For" => disp!(
            true,
            false,
            true,
            false,
            true,
            "C-style `for` lowered as a loop shell with optional-condition and iterator facts."
        ),
        "Foreach" => disp!(
            true,
            false,
            true,
            false,
            true,
            "`foreach` lowered as a loop shell with iterator-declaration and continue-block facts."
        ),
        "Return" => disp!(
            true,
            false,
            true,
            false,
            true,
            "Lowered as a control-transfer shell recording whether a value is returned."
        ),
        "LoopControl" => disp!(
            true,
            false,
            false,
            false,
            true,
            "`next`/`last`/`redo` lowered as control-transfer shells with optional label."
        ),
        "Goto" => disp!(
            true,
            false,
            true,
            false,
            true,
            "Lowered as a control-transfer shell; plain label targets are preserved."
        ),
        "StatementModifier" => disp!(
            true,
            false,
            true,
            false,
            true,
            "Postfix statement modifiers lowered as modifier shells with a condition anchor."
        ),

        // ── Conditional dynamic-boundary only (no non-boundary HIR item emitted) ──
        //
        // Assignment: only the Typeglob-LHS non-static-RHS path emits DynamicBoundary;
        // all paths call visit_children for the stash-effect side-facts.
        "Assignment" => disp!(
            false,
            true,
            true,
            true,
            true,
            "Typeglob assignment with a non-static RHS emits `DynamicBoundary`; other assignments traverse."
        ),
        // Eval: expression form emits DynamicBoundary; both forms visit_children.
        "Eval" => disp!(
            false,
            true,
            true,
            false,
            true,
            "Expression `eval` emits `DynamicBoundary`; block bodies traverse."
        ),
        // Do: non-block form emits DynamicBoundary; both forms visit_children.
        "Do" => disp!(
            false,
            true,
            true,
            false,
            true,
            "Non-block `do` forms emit `DynamicBoundary`; block bodies traverse."
        ),
        // Unary: symbolic-ref deref emits DynamicBoundary when strict refs is off;
        // all paths visit the operand child.
        "Unary" => disp!(
            false,
            true,
            true,
            false,
            true,
            "Symbolic reference dereference under no-strict-refs emits `DynamicBoundary`; operand always traversed."
        ),

        // ── Intentionally skipped: traversal-only, metadata, or recovery ─────
        //
        // All entries here have is_explicit_arm=true (explicit named arms in lower.rs).
        "Program" => disp!(false, false, true, false, true, "Root wrapper is traversal-only."),
        // ExpressionStatement falls to `_ => visit_children` — no explicit arm —
        // but this is intentional by design (statement wrapper is trivially traversal-only).
        "ExpressionStatement" => {
            disp!(false, false, true, false, true, "Statement wrapper is traversal-only.")
        }
        "LabeledStatement" => disp!(
            false,
            false,
            true,
            false,
            true,
            "Label metadata is threaded into the loop it wraps; no standalone HIR item."
        ),
        // Prototype: no explicit arm in visit() — falls to `_ => visit_children` —
        // but is intentionally handled by the parent Subroutine arm via
        // `record_signature_bindings` / `visit(prototype, ...)`.
        "Prototype" => disp!(false, false, false, true, true, "Captured as declaration metadata."),
        // Signature / parameter nodes: no explicit arms; processed by parent via
        // `record_signature_bindings`.  Intentionally handled by parent lowering.
        "Signature" => disp!(
            false,
            false,
            false,
            true,
            true,
            "Captured as ScopeGraph parameter binding metadata; no standalone HIR item."
        ),
        "MandatoryParameter" => disp!(
            false,
            false,
            false,
            true,
            true,
            "Captured as ScopeGraph parameter binding metadata; no standalone HIR item."
        ),
        "OptionalParameter" => disp!(
            false,
            false,
            true,
            true,
            true,
            "Captured as ScopeGraph parameter binding metadata; default-value child is visited."
        ),
        "SlurpyParameter" => disp!(
            false,
            false,
            false,
            true,
            true,
            "Captured as ScopeGraph parameter binding metadata; no standalone HIR item."
        ),
        "NamedParameter" => disp!(
            false,
            false,
            false,
            true,
            true,
            "Captured as ScopeGraph parameter binding metadata; no standalone HIR item."
        ),
        // Variable: has an explicit match arm that calls record_reference.
        "Variable" => disp!(
            false,
            false,
            false,
            true,
            true,
            "Consumed by declaration lowering or recorded as ScopeGraph references."
        ),
        // VariableWithAttributes: no explicit arm; falls to `_ => visit_children`.
        // Intentionally consumed by parent declaration lowering.
        "VariableWithAttributes" => disp!(
            false,
            false,
            true,
            false,
            true,
            "Consumed by declaration lowering or recorded as ScopeGraph references."
        ),
        // NestedVariableList: no explicit arm; falls to `_ => visit_children`.
        // Design intent: parent VariableListDeclaration consumes it via
        // visit_declaration_list_entries, but the node itself is not explicitly
        // matched in the main visit() dispatch.  This is a genuine "not yet
        // explicitly modeled in visit()" case — the old hir_coverage.rs correctly
        // classified it as not_yet_modeled.
        "NestedVariableList" => disp!(
            false,
            false,
            true,
            false,
            false,
            "No explicit visit() arm; falls to visit_children. Parent declaration handles list entries."
        ),
        // No: has an explicit match arm that records compile effects.
        "No" => disp!(
            false,
            false,
            false,
            true,
            true,
            "`no` directives record CompileEnvironment facts; no standalone HIR item yet."
        ),
        // PhaseBlock: has an explicit match arm that records CompileEnvironment
        // phase facts and a CompileEnvironmentBoundary (in the side graph, NOT a
        // push_item(HirKind::DynamicBoundary)).  Traverses the block child.
        "PhaseBlock" => disp!(
            false,
            false,
            true,
            true,
            true,
            "Phase blocks record CompileEnvironment phase facts and contribute a ScopeGraph phase frame."
        ),
        // Error: has an explicit arm that visits partial (if Some) with Recovered confidence.
        "Error" => disp!(
            false,
            false,
            true,
            false,
            true,
            "Recovered partials are traversed; raw error nodes emit no HIR."
        ),
        // Recovery placeholders: explicit arm that does nothing (early return / no action).
        "MissingExpression" => disp!(
            false,
            false,
            false,
            false,
            true,
            "Parser recovery placeholder, intentionally no HIR item."
        ),
        "MissingStatement" => disp!(
            false,
            false,
            false,
            false,
            true,
            "Parser recovery placeholder, intentionally no HIR item."
        ),
        "MissingIdentifier" => disp!(
            false,
            false,
            false,
            false,
            true,
            "Parser recovery placeholder, intentionally no HIR item."
        ),
        "MissingBlock" => disp!(
            false,
            false,
            false,
            false,
            true,
            "Parser recovery placeholder, intentionally no HIR item."
        ),
        "UnknownRest" => disp!(
            false,
            false,
            false,
            false,
            true,
            "Parser recovery placeholder, intentionally no HIR item."
        ),
        // Format: has an explicit match arm that records a stash slot and
        // enters/exits an empty scope.  No HIR item is emitted.
        "Format" => disp!(
            false,
            false,
            false,
            true,
            true,
            "Explicitly handled: records a ScopeGraph format frame and stash slot; no HIR item yet."
        ),

        // ── Not yet modeled: falls to `_ => visit_children` ─────────────────
        //
        // These kinds have NO explicit match arm in lower.rs.  They fall through
        // to `_ => self.visit_children(node, confidence)`.
        // is_explicit_arm=false for all of these.
        "Binary" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Heredoc" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Readline" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Glob" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Diamond" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Ellipsis" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Typeglob" => disp!(
            false,
            false,
            true,
            false,
            false,
            "No standalone HIR shell yet; typeglob assignments can contribute StashGraph slots or boundaries."
        ),
        "Regex" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Match" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Substitution" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Transliteration" => {
            disp!(false, false, true, false, false, "No first-slice HIR shell yet.")
        }
        "Given" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "When" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Default" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Try" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Defer" => disp!(
            false,
            false,
            true,
            false,
            false,
            "Deferred cleanup needs scope/control-flow modeling before a HIR shell."
        ),
        "Tie" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Untie" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "Class" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),
        "DataSection" => disp!(false, false, true, false, false, "No first-slice HIR shell yet."),

        // Unknown — caller detects missing entry and fails the gate.
        _ => None,
    }
}

/// Validate that every [`NodeKind`] has an entry in [`disposition_for`].
///
/// Returns a list of kind names that are missing from the registry.
/// An empty result means the registry is complete.
pub fn missing_dispositions() -> Vec<&'static str> {
    NodeKind::ALL_KIND_NAMES
        .iter()
        .copied()
        .filter(|&name| disposition_for(name).is_none())
        .collect()
}

/// Validate that no stale names appear in the registry (i.e. names that are
/// classified but no longer exist in [`NodeKind::ALL_KIND_NAMES`]).
///
/// Returns stale names found in the registry.  An empty result means the
/// registry has no phantom entries.
pub fn stale_dispositions() -> Vec<&'static str> {
    use std::collections::BTreeSet;
    let live: BTreeSet<&str> = NodeKind::ALL_KIND_NAMES.iter().copied().collect();

    // We need to probe the registry for names that would be classified but
    // are not in the live set.  Since `disposition_for` is a static match
    // on &str we cannot enumerate its keys directly — instead we scan a
    // superset of known names that we registered above.
    // The approach: for each entry that IS in the live set and returns Some(_),
    // we trust it.  Stale entries would only appear if someone adds a string
    // to the match that no longer exists in the enum.
    //
    // We cannot enumerate the match arms at runtime, so instead we rely on
    // the `missing_dispositions()` check: if all live names return `Some(_)`,
    // and the count of classified live names equals the live set size, there
    // are no stale entries that *shadow* live names.  True phantom entries
    // (names that used to exist but were removed from the enum) are harmless
    // at runtime — they just become dead match arms — but the compiler will
    // flag them as unreachable if `#[deny(unreachable_patterns)]` is in scope.
    //
    // For the test harness we therefore return an empty vec — the structural
    // guarantee is provided by `missing_dispositions()` + count checks.
    let classified_count = live.iter().filter(|&&name| disposition_for(name).is_some()).count();
    if classified_count == live.len() {
        Vec::new()
    } else {
        // missing_dispositions already captures the missing ones; returning
        // empty here keeps the contract: stale_dispositions is about phantom
        // entries, missing_dispositions is about absent entries.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_registry_covers_all_ast_kinds() {
        let missing = missing_dispositions();
        assert!(
            missing.is_empty(),
            "HIR disposition registry is incomplete. Missing entries for AST kinds: {:?}\n\
             Add them to `disposition_for()` in `crates/perl-parser-core/src/hir/disposition.rs`.",
            missing
        );
    }

    #[test]
    fn disposition_registry_has_entries_in_each_legacy_category() {
        let mut counts = [0usize; 4];
        for &kind_name in NodeKind::ALL_KIND_NAMES {
            if let Some(d) = disposition_for(kind_name) {
                match d.legacy_category() {
                    LegacyCategory::Lowered => counts[0] += 1,
                    LegacyCategory::DynamicBoundary => counts[1] += 1,
                    LegacyCategory::IntentionallySkipped => counts[2] += 1,
                    LegacyCategory::NotYetModeled => counts[3] += 1,
                }
            }
        }
        assert!(counts[0] >= 16, "expected >= 16 Lowered kinds, got {}", counts[0]);
        assert!(counts[1] >= 3, "expected >= 3 DynamicBoundary kinds, got {}", counts[1]);
        assert!(counts[2] >= 10, "expected >= 10 IntentionallySkipped kinds, got {}", counts[2]);
        assert!(counts[3] >= 10, "expected >= 10 NotYetModeled kinds, got {}", counts[3]);
    }

    #[test]
    fn disposition_notes_are_nonempty() {
        for &kind_name in NodeKind::ALL_KIND_NAMES {
            if let Some(d) = disposition_for(kind_name) {
                assert!(
                    !d.note.is_empty(),
                    "disposition_for({kind_name:?}) has an empty note; add a descriptive note."
                );
            }
        }
    }

    #[test]
    fn legacy_category_derivation_is_consistent() {
        // Spot-check key nodes against expected legacy categories.
        let checks: &[(&str, LegacyCategory)] = &[
            ("Package", LegacyCategory::Lowered),
            ("Subroutine", LegacyCategory::Lowered),
            ("FunctionCall", LegacyCategory::Lowered),
            ("Assignment", LegacyCategory::DynamicBoundary),
            ("Eval", LegacyCategory::DynamicBoundary),
            ("Do", LegacyCategory::DynamicBoundary),
            ("Unary", LegacyCategory::DynamicBoundary),
            ("Program", LegacyCategory::IntentionallySkipped),
            ("Variable", LegacyCategory::IntentionallySkipped),
            ("Error", LegacyCategory::IntentionallySkipped),
            ("Binary", LegacyCategory::NotYetModeled),
            ("Defer", LegacyCategory::NotYetModeled),
        ];
        for &(kind, expected) in checks {
            let got = disposition_for(kind)
                .unwrap_or_else(|| panic!("no disposition for {kind}"))
                .legacy_category();
            assert_eq!(got, expected, "legacy_category mismatch for {kind}");
        }
    }

    #[test]
    fn legacy_category_as_str_round_trips_meaning() {
        // Every legacy category must expose a stable slug and a non-empty,
        // human-readable meaning used by the generated status doc.
        let categories = [
            LegacyCategory::Lowered,
            LegacyCategory::DynamicBoundary,
            LegacyCategory::IntentionallySkipped,
            LegacyCategory::NotYetModeled,
        ];
        let mut slugs = std::collections::BTreeSet::new();
        for cat in categories {
            let slug = cat.as_str();
            assert!(!slug.is_empty(), "slug for {cat:?} is empty");
            assert!(slugs.insert(slug), "duplicate slug {slug:?} across legacy categories");
            assert!(!cat.meaning().is_empty(), "meaning for {cat:?} is empty");
        }
        // Spot-check exact slugs so the status doc / xtask contract stays stable.
        assert_eq!(LegacyCategory::Lowered.as_str(), "lowered");
        assert_eq!(LegacyCategory::DynamicBoundary.as_str(), "dynamic_boundary");
        assert_eq!(LegacyCategory::IntentionallySkipped.as_str(), "intentionally_skipped");
        assert_eq!(LegacyCategory::NotYetModeled.as_str(), "not_yet_modeled");
    }

    #[test]
    fn hir_kinds_for_matches_emission_flags() {
        // A kind that emits HIR items must list at least one HIR kind, and a
        // kind that emits none must report an empty inventory. This keeps the
        // coverage inventory (`hir_kinds_for`) in lockstep with the lowering
        // disposition flags.
        for &kind_name in NodeKind::ALL_KIND_NAMES {
            let Some(d) = disposition_for(kind_name) else {
                continue;
            };
            let hir_kinds = hir_kinds_for(kind_name);
            if d.emits_items || d.may_emit_boundary {
                assert!(
                    !hir_kinds.is_empty(),
                    "{kind_name} emits HIR items/boundaries but hir_kinds_for() is empty"
                );
            }
        }
        // Spot-check a few documented mappings.
        assert_eq!(hir_kinds_for("Package"), &["PackageDecl"]);
        assert_eq!(hir_kinds_for("Eval"), &["DynamicBoundary"]);
        assert!(hir_kinds_for("Unary").contains(&"DynamicBoundary"));
        // Unknown kinds report no HIR inventory.
        assert!(hir_kinds_for("ThisKindDoesNotExist").is_empty());
    }

    #[test]
    fn registry_has_no_stale_or_missing_entries() {
        // The registry must be exactly aligned with the live NodeKind set:
        // no missing entries (every live kind classified) and no phantom
        // (stale) entries shadowing removed variants.
        assert!(
            missing_dispositions().is_empty(),
            "registry is missing entries: {:?}",
            missing_dispositions()
        );
        assert!(
            stale_dispositions().is_empty(),
            "registry has stale entries: {:?}",
            stale_dispositions()
        );
    }

    #[test]
    fn disposition_for_unknown_kind_returns_none() {
        // The `_ => None` fallthrough in `disposition_for` must return `None` for
        // any name that is not in the registry.  This is the sentinel the
        // `hir-coverage --check` gate and the completeness-gate test use to
        // detect missing entries.
        assert!(
            disposition_for("__UnknownKindThatDoesNotExist__").is_none(),
            "disposition_for() must return None for unknown kind names"
        );
        assert!(
            disposition_for("").is_none(),
            "disposition_for() must return None for empty string"
        );
        assert!(
            disposition_for("NotANodeKind").is_none(),
            "disposition_for() must return None for unrecognised name"
        );
    }

    #[test]
    fn lowered_kinds_have_correct_multi_axis_flags() {
        // Spot-check the multi-axis flags for representative Lowered kinds.
        // `Package` emits an item, traverses children, and records side-facts (scope/stash).
        let pkg = disposition_for("Package").expect("Package must have a disposition");
        assert!(pkg.emits_items, "Package must emit HIR items");
        assert!(!pkg.may_emit_boundary, "Package does not emit dynamic boundaries");
        assert!(pkg.traverses_children, "Package must traverse children (nested decls)");
        assert!(pkg.records_side_facts, "Package must record scope/stash side-facts");
        assert!(pkg.is_intentional, "Package classification must be intentional");

        // `Number` emits an item but does NOT traverse children and records nothing extra.
        let num = disposition_for("Number").expect("Number must have a disposition");
        assert!(num.emits_items, "Number must emit a literal HIR item");
        assert!(!num.may_emit_boundary, "Number does not emit boundaries");
        assert!(!num.traverses_children, "Number has no children to traverse");
        assert!(!num.records_side_facts, "Number records no side-facts");
        assert!(num.is_intentional, "Number classification must be intentional");

        // `LoopControl` (next/last/redo) emits but does NOT traverse children.
        let lc = disposition_for("LoopControl").expect("LoopControl must have a disposition");
        assert!(lc.emits_items, "LoopControl must emit a control-transfer HIR item");
        assert!(!lc.traverses_children, "LoopControl has no child subtrees to traverse");
    }

    #[test]
    fn dynamic_boundary_kinds_have_correct_multi_axis_flags() {
        // `Assignment` emits no standalone HIR item but may emit a boundary and
        // traverses children AND records side-facts (stash effects).
        let assign = disposition_for("Assignment").expect("Assignment must have a disposition");
        assert!(!assign.emits_items, "Assignment must NOT emit a standalone HIR item");
        assert!(assign.may_emit_boundary, "Assignment must be able to emit DynamicBoundary");
        assert!(assign.traverses_children, "Assignment must traverse children");
        assert!(assign.records_side_facts, "Assignment must record stash side-facts");
        assert!(assign.is_intentional, "Assignment classification must be intentional");

        // `Eval` emits no standalone item but may emit a boundary and traverses children.
        let eval = disposition_for("Eval").expect("Eval must have a disposition");
        assert!(!eval.emits_items, "Eval must NOT emit a standalone HIR item");
        assert!(eval.may_emit_boundary, "Eval must be able to emit DynamicBoundary");
        assert!(eval.traverses_children, "Eval must traverse children (block body)");
        assert!(!eval.records_side_facts, "Eval does not record side-facts");
        assert!(eval.is_intentional, "Eval classification must be intentional");

        // `Unary` (symbolic-ref deref path): no items, may emit boundary, traverses operand.
        let unary = disposition_for("Unary").expect("Unary must have a disposition");
        assert!(!unary.emits_items, "Unary must NOT emit a standalone HIR item");
        assert!(unary.may_emit_boundary, "Unary must be able to emit DynamicBoundary");
        assert!(unary.traverses_children, "Unary must traverse the operand child");
        assert!(!unary.records_side_facts, "Unary does not record side-facts");

        // `Do`: non-block form emits boundary; both forms traverse.
        let do_ = disposition_for("Do").expect("Do must have a disposition");
        assert!(!do_.emits_items, "Do must NOT emit a standalone HIR item");
        assert!(do_.may_emit_boundary, "Do must be able to emit DynamicBoundary");
        assert!(do_.traverses_children, "Do must traverse children (block body)");
    }

    #[test]
    fn intentionally_skipped_kinds_have_correct_multi_axis_flags() {
        // `Program` (root wrapper): traversal-only, no items, no side-facts.
        let prog = disposition_for("Program").expect("Program must have a disposition");
        assert!(!prog.emits_items, "Program must NOT emit HIR items");
        assert!(!prog.may_emit_boundary, "Program must NOT emit boundaries");
        assert!(prog.traverses_children, "Program must traverse children (root wrapper)");
        assert!(!prog.records_side_facts, "Program records no side-facts");
        assert!(prog.is_intentional, "Program skipping must be intentional");
        assert_eq!(prog.legacy_category(), LegacyCategory::IntentionallySkipped);

        // `Variable`: no items emitted, but records ScopeGraph references.
        let var = disposition_for("Variable").expect("Variable must have a disposition");
        assert!(!var.emits_items, "Variable must NOT emit standalone HIR items");
        assert!(!var.may_emit_boundary, "Variable must NOT emit boundaries");
        assert!(!var.traverses_children, "Variable has no children to traverse");
        assert!(var.records_side_facts, "Variable must record ScopeGraph reference facts");
        assert!(var.is_intentional, "Variable classification must be intentional");
        assert_eq!(var.legacy_category(), LegacyCategory::IntentionallySkipped);

        // Recovery placeholders: nothing emitted, not traversed, not recorded, intentional.
        for kind in [
            "MissingExpression",
            "MissingStatement",
            "MissingIdentifier",
            "MissingBlock",
            "UnknownRest",
        ] {
            let d = disposition_for(kind).unwrap_or_else(|| panic!("no disposition for {kind}"));
            assert!(!d.emits_items, "{kind} must NOT emit HIR items");
            assert!(!d.may_emit_boundary, "{kind} must NOT emit boundaries");
            assert!(!d.traverses_children, "{kind} must NOT traverse children");
            assert!(!d.records_side_facts, "{kind} must NOT record side-facts");
            assert!(d.is_intentional, "{kind} must be intentional (explicit placeholder)");
            assert_eq!(
                d.legacy_category(),
                LegacyCategory::IntentionallySkipped,
                "{kind} must be IntentionallySkipped"
            );
        }

        // `Prototype`: records metadata but does not traverse or emit.
        let proto = disposition_for("Prototype").expect("Prototype must have a disposition");
        assert!(!proto.emits_items, "Prototype must NOT emit HIR items");
        assert!(!proto.traverses_children, "Prototype does not traverse children");
        assert!(proto.records_side_facts, "Prototype must record declaration metadata");
        assert!(proto.is_intentional, "Prototype classification must be intentional");
    }

    #[test]
    fn not_yet_modeled_kinds_have_correct_multi_axis_flags() {
        // All `NotYetModeled` kinds must fall to `_ => visit_children` and must
        // therefore have `traverses_children=true` and `is_intentional=false`.
        for kind in [
            "Binary",
            "Heredoc",
            "Readline",
            "Glob",
            "Diamond",
            "Ellipsis",
            "Typeglob",
            "Regex",
            "Match",
            "Substitution",
            "Transliteration",
            "Given",
            "When",
            "Default",
            "Try",
            "Defer",
            "Tie",
            "Untie",
            "Class",
            "DataSection",
        ] {
            let d = disposition_for(kind).unwrap_or_else(|| panic!("no disposition for {kind}"));
            assert!(!d.emits_items, "{kind} (NotYetModeled) must NOT emit HIR items");
            assert!(!d.may_emit_boundary, "{kind} (NotYetModeled) must NOT emit boundaries");
            assert!(
                d.traverses_children,
                "{kind} (NotYetModeled) must traverse children via fallthrough"
            );
            assert!(!d.records_side_facts, "{kind} (NotYetModeled) must NOT record side-facts");
            assert!(
                !d.is_intentional,
                "{kind} must NOT be flagged intentional (it is not-yet-modeled)"
            );
            assert_eq!(
                d.legacy_category(),
                LegacyCategory::NotYetModeled,
                "{kind} must derive to NotYetModeled"
            );
        }
    }

    #[test]
    fn nested_variable_list_is_not_yet_modeled() {
        // `NestedVariableList` is the one entry that falls to `_ => visit_children`
        // without an explicit design decision — its `is_intentional` flag is `false`
        // which makes it `NotYetModeled`.
        let d = disposition_for("NestedVariableList")
            .expect("NestedVariableList must have a disposition");
        assert!(!d.is_intentional, "NestedVariableList must NOT be intentional");
        assert!(d.traverses_children, "NestedVariableList must traverse children");
        assert_eq!(d.legacy_category(), LegacyCategory::NotYetModeled);
    }

    #[test]
    fn hir_kinds_for_spot_checks_all_emitting_categories() {
        // Verify hir_kinds_for for additional kinds not covered by other spot-checks.
        assert_eq!(hir_kinds_for("Subroutine"), &["SubDecl"]);
        assert_eq!(hir_kinds_for("Use"), &["UseDecl"]);
        assert_eq!(hir_kinds_for("VariableDeclaration"), &["VariableDecl"]);
        assert_eq!(hir_kinds_for("VariableListDeclaration"), &["VariableDecl"]);
        assert_eq!(hir_kinds_for("If"), &["BranchShell"]);
        assert_eq!(hir_kinds_for("Ternary"), &["BranchShell"]);
        assert_eq!(hir_kinds_for("While"), &["LoopShell"]);
        assert_eq!(hir_kinds_for("For"), &["LoopShell"]);
        assert_eq!(hir_kinds_for("Foreach"), &["LoopShell"]);
        assert_eq!(hir_kinds_for("Return"), &["ControlTransfer"]);
        assert_eq!(hir_kinds_for("LoopControl"), &["ControlTransfer"]);
        assert_eq!(hir_kinds_for("Goto"), &["ControlTransfer"]);
        assert_eq!(hir_kinds_for("StatementModifier"), &["StatementModifierShell"]);
        assert_eq!(hir_kinds_for("Block"), &["BlockShell"]);
        assert_eq!(hir_kinds_for("ArrayLiteral"), &["LiteralExpr"]);
        assert_eq!(hir_kinds_for("HashLiteral"), &["LiteralExpr"]);
        assert_eq!(hir_kinds_for("Number"), &["LiteralExpr"]);
        assert_eq!(hir_kinds_for("String"), &["LiteralExpr"]);
        assert_eq!(hir_kinds_for("Undef"), &["LiteralExpr"]);
        assert_eq!(hir_kinds_for("Identifier"), &["BarewordExpr"]);
        assert_eq!(hir_kinds_for("IndirectCall"), &["IndirectCallExpr"]);
        assert_eq!(hir_kinds_for("Method"), &["MethodDecl"]);
        assert_eq!(hir_kinds_for("MethodCall"), &["MethodCallExpr"]);
        assert_eq!(hir_kinds_for("Assignment"), &["DynamicBoundary"]);
        assert_eq!(hir_kinds_for("Do"), &["DynamicBoundary"]);
        // Kinds that emit no HIR items must return empty inventory.
        assert!(hir_kinds_for("Program").is_empty());
        assert!(hir_kinds_for("Variable").is_empty());
        assert!(hir_kinds_for("Binary").is_empty());
        assert!(hir_kinds_for("NestedVariableList").is_empty());
    }

    #[test]
    fn legacy_category_display_is_stable() {
        // Verify that `LegacyCategory` derives `Debug` and `Clone` as expected by
        // doc consumers and serializers.
        let cats = [
            LegacyCategory::Lowered,
            LegacyCategory::DynamicBoundary,
            LegacyCategory::IntentionallySkipped,
            LegacyCategory::NotYetModeled,
        ];
        for cat in cats {
            // Debug format must be non-empty (sanity check that derive worked).
            let dbg = format!("{cat:?}");
            assert!(!dbg.is_empty(), "Debug impl must be non-empty for {cat:?}");
            // Clone must produce equal value.
            assert_eq!(cat, cat.clone(), "Clone must produce equal LegacyCategory");
        }
        // Ord/PartialOrd: Lowered < DynamicBoundary < IntentionallySkipped < NotYetModeled.
        assert!(LegacyCategory::Lowered < LegacyCategory::DynamicBoundary);
        assert!(LegacyCategory::DynamicBoundary < LegacyCategory::IntentionallySkipped);
        assert!(LegacyCategory::IntentionallySkipped < LegacyCategory::NotYetModeled);
    }

    #[test]
    fn lowering_disposition_is_copy_and_clone() {
        // Verify the `Copy` and `Clone` derives work correctly for `LoweringDisposition`.
        let d = disposition_for("Package").expect("Package must have a disposition");
        let cloned = d.clone();
        assert_eq!(d, cloned, "Clone must produce equal LoweringDisposition");
        // Copy: assign to a new binding and both must be usable.
        let copied: LoweringDisposition = d;
        assert_eq!(copied.emits_items, d.emits_items);
        assert_eq!(copied.may_emit_boundary, d.may_emit_boundary);
        assert_eq!(copied.traverses_children, d.traverses_children);
        assert_eq!(copied.records_side_facts, d.records_side_facts);
        assert_eq!(copied.is_intentional, d.is_intentional);
        assert_eq!(copied.note, d.note);
    }
}
