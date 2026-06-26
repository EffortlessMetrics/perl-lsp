//! HIR lowering completeness / boundary-soundness gate.
//!
//! Ensures that every AST NodeKind is **explicitly classified** in the shared
//! lowering-disposition registry at
//! `crates/perl-parser-core/src/hir/disposition.rs`.  That registry is the
//! **single source of truth** for how every AST kind is treated by `lower.rs`
//! and by the `hir-coverage` metrics tool.
//!
//! The completeness invariant: **no AST kind silently disappears** through the
//! `_ => self.visit_children(node, confidence)` fallthrough arm in `lower.rs`
//! without being explicitly acknowledged in the registry.
//!
//! When a new NodeKind variant is added to `perl_ast::NodeKind`, this test
//! will fail — forcing the author to decide its classification and update the
//! registry before the PR can merge.
//!
//! Refs: issue #2193 (HIR lowering completeness gate), epic #2076.

use perl_parser_core::NodeKind;
use perl_parser_core::hir::disposition;

/// Gate: every NodeKind has an explicit classification in the shared registry.
///
/// This test fails if any `NodeKind` variant is present in `ALL_KIND_NAMES`
/// but absent from `disposition_for()`.  That forces the author of the new
/// variant to decide its lowering classification explicitly.
#[test]
fn hir_lowering_completeness_gate() {
    let missing = disposition::missing_dispositions();

    assert!(
        missing.is_empty(),
        "HIR lowering completeness gate FAILED — the following AST NodeKind variants \
         have no explicit lowering classification.\n\
         \n\
         Every kind must be listed in `disposition_for()` in \
         `crates/perl-parser-core/src/hir/disposition.rs`.\n\
         \n\
         Unclassified kinds ({}):\n  {}\n\
         \n\
         This prevents silent disappearance of Perl constructs through the\n\
         `_ => visit_children` fallthrough arm in `crates/perl-parser-core/src/hir/lower.rs`.\n\
         Refs: issue #2193, epic #2076.",
        missing.len(),
        missing.join("\n  ")
    );
}

/// Verify that the registry covers exactly the current `ALL_KIND_NAMES` set
/// (no stale entries, no duplicates — the registry is a match on &str so
/// phantom entries become dead match arms, caught by the compiler).
#[test]
fn hir_completeness_classification_covers_all_variants() {
    // Every name in ALL_KIND_NAMES must return Some(_) from disposition_for().
    let unclassified: Vec<&str> = NodeKind::ALL_KIND_NAMES
        .iter()
        .copied()
        .filter(|&n| disposition::disposition_for(n).is_none())
        .collect();

    assert!(
        unclassified.is_empty(),
        "Unclassified NodeKind(s) detected: {:?} — update `disposition_for()` in \
         `crates/perl-parser-core/src/hir/disposition.rs`.",
        unclassified
    );

    // Count legacy categories and assert sane floors.
    let mut counts = [0usize; 4];
    for &kind_name in NodeKind::ALL_KIND_NAMES {
        if let Some(d) = disposition::disposition_for(kind_name) {
            match d.legacy_category() {
                disposition::LegacyCategory::Lowered => counts[0] += 1,
                disposition::LegacyCategory::DynamicBoundary => counts[1] += 1,
                disposition::LegacyCategory::IntentionallySkipped => counts[2] += 1,
                disposition::LegacyCategory::NotYetModeled => counts[3] += 1,
            }
        }
    }

    let total = counts.iter().sum::<usize>();
    assert_eq!(
        total,
        NodeKind::ALL_KIND_NAMES.len(),
        "Classification total ({total}) != NodeKind variant count ({}). \
         This is a logic error in the test.",
        NodeKind::ALL_KIND_NAMES.len()
    );

    assert!(
        counts[0] >= 16,
        "Expected at least 16 Lowered kinds (baseline from issue #2124), got {}",
        counts[0]
    );
    assert!(
        counts[1] >= 3,
        "Expected at least 3 DynamicBoundary kinds (baseline from issue #2124), got {}",
        counts[1]
    );
    assert!(counts[2] >= 10, "Expected at least 10 IntentionallySkipped kinds, got {}", counts[2]);
}

/// Verify that `not_yet_modeled` kinds do NOT silently lose their children.
///
/// For each `NotYetModeled` kind (those falling to `_ => visit_children`),
/// the lowerer MUST traverse child nodes. This test verifies that a simple
/// Perl program containing such constructs still produces HIR items for
/// any contained sub-expressions that ARE explicitly modeled.
///
/// Specifically: a `Binary` expression wrapping a FunctionCall must still
/// produce a `CallExpr` HIR item, because `Binary` falls to `visit_children`
/// which recurses into the call.
#[test]
fn hir_not_yet_modeled_kinds_traverse_children() {
    use perl_parser_core::Parser;
    use perl_parser_core::hir::HirKind;
    use perl_parser_core::hir::lower_ast;

    // Binary wraps a FunctionCall — Binary is NotYetModeled, FunctionCall is Lowered.
    // After lowering, we expect to find the CallExpr from the inner FunctionCall.
    let source = "my $x = foo() + bar();";
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let file = lower_ast(&output.ast);

    // Verify Binary is classified as NotYetModeled in the registry.
    let binary_disp =
        disposition::disposition_for("Binary").expect("Binary must have a disposition entry");
    assert_eq!(
        binary_disp.legacy_category(),
        disposition::LegacyCategory::NotYetModeled,
        "Binary should be NotYetModeled"
    );
    assert!(
        binary_disp.traverses_children,
        "Binary (NotYetModeled) must have traverses_children=true in the registry"
    );

    let has_call_expr = file.items.iter().any(|item| matches!(&item.kind, HirKind::CallExpr(_)));
    assert!(
        has_call_expr,
        "Binary (NotYetModeled) should traverse children; expected CallExpr for foo()/bar() \
         inside binary expression, but none found in HIR.\n\
         HIR item count: {}",
        file.items.len()
    );
}

/// Verify `DynamicBoundary` kinds emit the boundary marker rather than silently traversing.
///
/// Expression-form `eval` (non-block) must emit `DynamicBoundary::EvalExpression`.
#[test]
fn hir_dynamic_boundary_kinds_emit_boundary_marker() {
    use perl_parser_core::Parser;
    use perl_parser_core::hir::lower_ast;
    use perl_parser_core::hir::{DynamicBoundaryKind, HirKind};

    // Verify Eval is classified as DynamicBoundary in the registry.
    let eval_disp =
        disposition::disposition_for("Eval").expect("Eval must have a disposition entry");
    assert_eq!(
        eval_disp.legacy_category(),
        disposition::LegacyCategory::DynamicBoundary,
        "Eval should be DynamicBoundary"
    );
    assert!(eval_disp.may_emit_boundary, "Eval must have may_emit_boundary=true in the registry");

    // Expression eval must emit DynamicBoundary
    let source = r#"my $result = eval "1 + 1";"#;
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let file = lower_ast(&output.ast);

    let has_eval_boundary = file.items.iter().any(|item| {
        matches!(&item.kind, HirKind::DynamicBoundary(b) if b.kind == DynamicBoundaryKind::EvalExpression)
    });
    assert!(
        has_eval_boundary,
        "Expression-form `eval` (DynamicBoundary kind) must emit DynamicBoundary::EvalExpression.\n\
         HIR item count: {}",
        file.items.len()
    );
}

/// Verify `intentionally_skipped` kinds still allow child HIR emission.
///
/// `ExpressionStatement` is traversal-only — it should not block HIR emission
/// from its contained expression. A `FunctionCall` inside an `ExpressionStatement`
/// must still produce a `CallExpr`.
#[test]
fn hir_intentionally_skipped_kinds_allow_child_hir_emission() {
    use perl_parser_core::Parser;
    use perl_parser_core::hir::HirKind;
    use perl_parser_core::hir::lower_ast;

    // Verify ExpressionStatement is classified as IntentionallySkipped.
    let es_disp = disposition::disposition_for("ExpressionStatement")
        .expect("ExpressionStatement must have a disposition entry");
    assert_eq!(
        es_disp.legacy_category(),
        disposition::LegacyCategory::IntentionallySkipped,
        "ExpressionStatement should be IntentionallySkipped"
    );
    assert!(
        es_disp.traverses_children,
        "ExpressionStatement must have traverses_children=true in the registry"
    );

    // Top-level function call is wrapped in ExpressionStatement (intentionally skipped)
    let source = "say 'hello';";
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let file = lower_ast(&output.ast);

    let has_call_expr = file.items.iter().any(|item| matches!(&item.kind, HirKind::CallExpr(_)));
    assert!(
        has_call_expr,
        "ExpressionStatement (IntentionallySkipped) must allow child HIR emission. \
         Expected CallExpr for `say` call, but none found.\n\
         HIR item count: {}",
        file.items.len()
    );
}

/// Verify `Unary` (symbolic-ref deref) correctly emits `DynamicBoundary` and
/// is classified as `DynamicBoundary` in the registry — not `NotYetModeled`.
///
/// This was the key disagreement between the old `hir_coverage.rs` (which had
/// `Unary` as `not_yet_modeled`) and the actual lowerer behavior (which emits
/// `DynamicBoundary` for symbolic dereference under no-strict-refs).
#[test]
fn hir_unary_is_dynamic_boundary_not_not_yet_modeled() {
    use perl_parser_core::Parser;
    use perl_parser_core::hir::lower_ast;
    use perl_parser_core::hir::{DynamicBoundaryKind, HirKind};

    // Verify Unary is classified as DynamicBoundary in the registry.
    let unary_disp =
        disposition::disposition_for("Unary").expect("Unary must have a disposition entry");
    assert_eq!(
        unary_disp.legacy_category(),
        disposition::LegacyCategory::DynamicBoundary,
        "Unary should be DynamicBoundary (not NotYetModeled): it emits DynamicBoundary \
         for symbolic-ref deref when strict refs is disabled. \
         This was a disagreement in the old hir_coverage.rs that is now reconciled."
    );
    assert!(
        unary_disp.may_emit_boundary,
        "Unary must have may_emit_boundary=true: the lowerer emits DynamicBoundary \
         for ${{varname}} / @{{varname}} / etc. when strict refs is off."
    );

    // Symbolic dereference without strict refs should emit DynamicBoundary.
    // Note: lower_ast processes a file without `use strict`, so strict_refs is off.
    let source = "my $name = 'foo'; my @arr = @{$name};";
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let file = lower_ast(&output.ast);

    let has_symref_boundary = file.items.iter().any(|item| {
        matches!(
            &item.kind,
            HirKind::DynamicBoundary(b) if b.kind == DynamicBoundaryKind::SymbolicReferenceDeref
        )
    });
    assert!(
        has_symref_boundary,
        "Unary symbolic-ref deref `@{{$name}}` must emit DynamicBoundary::SymbolicReferenceDeref \
         when strict refs is disabled.\n\
         HIR item count: {}\n\
         HIR kinds: {:?}",
        file.items.len(),
        file.items.iter().map(|item| item.anchor.node_kind).collect::<Vec<_>>()
    );
}
