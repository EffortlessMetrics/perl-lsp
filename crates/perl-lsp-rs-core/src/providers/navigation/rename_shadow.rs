//! Rename shadow compare and cutover paths.
//!
//! Provides two entry points for rename:
//!
//! 1. **Shadow mode** ([`rename_shadow`]) — runs both legacy and semantic
//!    paths side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`rename_cutover`]) — uses the semantic path as
//!    the primary source of truth:
//!    - *Exact*: plan has no blockers → apply edits.
//!    - *Blocked*: plan has blockers → present blockers to user.
//!    - Ambiguous / Dynamic / Unavailable → block.
//!
//! # Requirements
//!
//! - **Req 9.6**: Rename calls `SemanticQueries::rename_plan` and applies
//!   edits only when the plan contains no blockers.
//! - **Req 16.4**: When the plan contains blockers, present them to the user
//!   and require confirmation before applying edits.
//! - **Req 22.8**: Rename cutover: Exact → allow; Ambiguous → block;
//!   Dynamic/Unavailable → block.

use perl_semantic_facts::{EntityId, PlanBlocker, PlannedEdit, RenamePlan};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

/// Result of a shadow-compared rename request.
///
/// Contains the legacy rename result (which callers should use during the
/// shadow phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct RenameShadowResult {
    /// Whether the legacy path would allow the rename.
    pub legacy_allowed: bool,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run rename through both legacy and semantic paths, producing a
/// shadow-compare receipt.
///
/// # Arguments
///
/// * `legacy_allowed` — whether the legacy rename path would allow the rename
///   (caller is responsible for running the legacy rename logic).
/// * `semantic_queries` — the new semantic query facade.
/// * `entity_id` — the entity being renamed.
/// * `new_name` — the proposed new name.
///
/// # Returns
///
/// A [`RenameShadowResult`] containing the legacy result and a receipt.
/// The caller should return the legacy result to the LSP client during the
/// shadow phase.
pub fn rename_shadow<Q: SemanticQueries>(
    legacy_allowed: bool,
    semantic_queries: &Q,
    entity_id: EntityId,
    new_name: &str,
) -> RenameShadowResult {
    // ── Legacy path ──
    let old_summary = legacy_rename_to_summary(legacy_allowed);

    // ── New semantic path ──
    let plan = semantic_queries.rename_plan(entity_id, new_name);
    let new_summary = rename_plan_to_summary(&plan);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::RenamePlan,
        ShadowQueryInput { symbol: new_name.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
    );

    tracing::debug!(
        entity_id = entity_id.0,
        new_name = %new_name,
        verdict = ?receipt.verdict,
        blockers = plan.blockers.len(),
        edits = plan.edits.len(),
        "rename shadow compare"
    );

    RenameShadowResult { legacy_allowed, receipt }
}

// ── Cutover types ──

/// Classification of the semantic rename result for cutover decisions.
///
/// Follows the fallback policy table (Req 22.8):
/// - Exact (no blockers) → allow rename, apply edits
/// - Blocked → present blockers to user
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameCutoverResult {
    /// Plan has no blockers — rename is safe to apply.
    Allowed {
        /// The planned edits to apply.
        edits: Vec<PlannedEdit>,
    },
    /// Plan has blockers — present them to the user.
    Blocked {
        /// The blockers preventing the rename.
        blockers: Vec<PlanBlocker>,
        /// The planned edits that would be applied if blockers are overridden.
        edits: Vec<PlannedEdit>,
    },
}

/// Outcome of a cutover rename request.
///
/// Contains the classified result and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct RenameCutoverOutcome {
    /// The classified cutover result.
    pub result: RenameCutoverResult,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run rename with the semantic path as primary.
///
/// # Decision logic
///
/// 1. Call `SemanticQueries::rename_plan` for the entity.
/// 2. Classify the result:
///    - **Allowed**: plan has no blockers → `RenameCutoverResult::Allowed`.
///    - **Blocked**: plan has blockers → `RenameCutoverResult::Blocked`.
/// 3. Emit a shadow-compare receipt regardless of outcome.
///
/// # Arguments
///
/// * `legacy_allowed` — whether the legacy rename path would allow the rename.
/// * `semantic_queries` — the semantic query facade (primary path).
/// * `entity_id` — the entity being renamed.
/// * `new_name` — the proposed new name.
///
/// # Returns
///
/// A [`RenameCutoverOutcome`] with the classified result and receipt.
pub fn rename_cutover<Q: SemanticQueries>(
    legacy_allowed: bool,
    semantic_queries: &Q,
    entity_id: EntityId,
    new_name: &str,
) -> RenameCutoverOutcome {
    // ── Semantic path (primary) ──
    let plan = semantic_queries.rename_plan(entity_id, new_name);
    let new_summary = rename_plan_to_summary(&plan);

    // ── Legacy path (for receipt) ──
    let old_summary = legacy_rename_to_summary(legacy_allowed);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::RenamePlan,
        ShadowQueryInput { symbol: new_name.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
    );

    // ── Classify result ──
    let result = classify_rename_result(plan);

    tracing::debug!(
        entity_id = entity_id.0,
        new_name = %new_name,
        verdict = ?receipt.verdict,
        classification = match &result {
            RenameCutoverResult::Allowed { .. } => "allowed",
            RenameCutoverResult::Blocked { .. } => "blocked",
        },
        "rename cutover"
    );

    RenameCutoverOutcome { result, receipt }
}

/// Classify a rename plan into the cutover result category.
fn classify_rename_result(plan: RenamePlan) -> RenameCutoverResult {
    if plan.blockers.is_empty() {
        RenameCutoverResult::Allowed { edits: plan.edits }
    } else {
        RenameCutoverResult::Blocked { blockers: plan.blockers, edits: plan.edits }
    }
}

/// Convert a legacy rename decision into a [`ShadowResultSummary`].
fn legacy_rename_to_summary(allowed: bool) -> ShadowResultSummary {
    if allowed {
        summarize_identities(Some(vec!["rename:allowed".to_string()]))
    } else {
        summarize_identities(None)
    }
}

/// Convert a semantic [`RenamePlan`] into a [`ShadowResultSummary`].
fn rename_plan_to_summary(plan: &RenamePlan) -> ShadowResultSummary {
    let mut identities: Vec<String> = Vec::new();

    // Edits as identities
    for edit in &plan.edits {
        identities.push(format!("edit:{}:anchor:{}", edit.category.label(), edit.anchor_id.0));
    }

    // Blockers as identities
    for blocker in &plan.blockers {
        identities.push(format!("blocker:{:?}", blocker.reason));
    }

    summarize_identities(Some(identities))
}

/// Extension trait for [`PlannedEditCategory`] display labels.
trait PlannedEditCategoryLabel {
    /// Human-readable label for the edit category.
    fn label(&self) -> &'static str;
}

impl PlannedEditCategoryLabel for perl_semantic_facts::PlannedEditCategory {
    fn label(&self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Reference => "reference",
            Self::ImportList => "import_list",
            Self::ExportList => "export_list",
            // PlannedEditCategory is #[non_exhaustive]; handle future variants.
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact, PlanBlocker,
        PlanBlockerReason, PlannedEdit, PlannedEditCategory, RenamePlan, SafeDeletePlan, ScopeId,
        VisibleSymbol,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    // ── Minimal SemanticQueries stub for testing ──

    struct StubSemanticQueries {
        rename_plan_result: RenamePlan,
    }

    impl SemanticQueries for StubSemanticQueries {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            Vec::new()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            Vec::new()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn rename_plan(&self, _entity_id: EntityId, _new_name: &str) -> RenamePlan {
            self.rename_plan_result.clone()
        }

        fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
            SafeDeletePlan::new(entity_id, String::new(), vec![], vec![])
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            None
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            None
        }
    }

    fn make_edit(anchor_id: u64, category: PlannedEditCategory) -> PlannedEdit {
        PlannedEdit::new(
            AnchorId(anchor_id),
            FileId(1),
            category,
            "old".to_string(),
            "new".to_string(),
        )
    }

    fn make_blocker(reason: PlanBlockerReason) -> PlanBlocker {
        PlanBlocker::new(reason, None, format!("{reason:?} blocker"))
    }

    // ── Shadow mode tests ──

    #[test]
    fn shadow_legacy_allowed_no_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let result = rename_shadow(true, &queries, EntityId(1), "new_name");

        assert!(result.legacy_allowed);
        assert_eq!(result.receipt.query, ShadowQueryName::RenamePlan);
        assert_eq!(result.receipt.input.symbol, "new_name");
        assert!(result.receipt.old_result.available);
        assert!(result.receipt.new_result.available);
        Ok(())
    }

    #[test]
    fn shadow_legacy_disallowed_yields_unavailable_old() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![],
            vec![make_blocker(PlanBlockerReason::DynamicBoundary)],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let result = rename_shadow(false, &queries, EntityId(1), "new_name");

        assert!(!result.legacy_allowed);
        assert!(!result.receipt.old_result.available);
        assert!(result.receipt.new_result.available);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    #[test]
    fn shadow_receipt_uses_rename_plan_query_name() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old".to_string(),
            "new".to_string(),
            vec![],
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let result = rename_shadow(true, &queries, EntityId(1), "new");

        assert_eq!(result.receipt.query, ShadowQueryName::RenamePlan);
        assert_eq!(result.receipt.schema_version, 1);
        Ok(())
    }

    // ── Cutover tests ──

    #[test]
    fn cutover_allowed_when_no_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let edit = make_edit(10, PlannedEditCategory::Definition);
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![edit.clone()],
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        match &outcome.result {
            RenameCutoverResult::Allowed { edits } => {
                assert_eq!(edits.len(), 1);
                assert_eq!(edits[0], edit);
            }
            other => return Err(format!("expected Allowed, got {:?}", other).into()),
        }
        assert_eq!(outcome.receipt.query, ShadowQueryName::RenamePlan);
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_dynamic_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::DynamicBoundary);
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            vec![blocker.clone()],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        match &outcome.result {
            RenameCutoverResult::Blocked { blockers, edits } => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].reason, PlanBlockerReason::DynamicBoundary);
                assert_eq!(edits.len(), 1);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_ambiguous_reference() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::AmbiguousReference);
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![],
            vec![blocker.clone()],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        match &outcome.result {
            RenameCutoverResult::Blocked { blockers, .. } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::AmbiguousReference);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_cross_module_export() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::CrossModuleExport);
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![],
            vec![blocker.clone()],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        match &outcome.result {
            RenameCutoverResult::Blocked { blockers, .. } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::CrossModuleExport);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_generated_member() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::GeneratedMember);
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![],
            vec![blocker.clone()],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        match &outcome.result {
            RenameCutoverResult::Blocked { blockers, .. } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::GeneratedMember);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_allowed_with_multiple_edits() -> Result<(), Box<dyn std::error::Error>> {
        let edits = vec![
            make_edit(10, PlannedEditCategory::Definition),
            make_edit(20, PlannedEditCategory::Reference),
            make_edit(30, PlannedEditCategory::ImportList),
            make_edit(40, PlannedEditCategory::ExportList),
        ];
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            edits.clone(),
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        match &outcome.result {
            RenameCutoverResult::Allowed { edits: result_edits } => {
                assert_eq!(result_edits.len(), 4);
                assert_eq!(*result_edits, edits);
            }
            other => return Err(format!("expected Allowed, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_edits_and_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            vec![make_blocker(PlanBlockerReason::DynamicBoundary)],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");

        // Receipt should reflect both edits and blockers.
        assert_eq!(outcome.receipt.new_result.match_count, 2);
        assert_eq!(outcome.receipt.query, ShadowQueryName::RenamePlan);
        Ok(())
    }

    // ── Summary helper tests ──

    #[test]
    fn legacy_rename_to_summary_allowed() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_rename_to_summary(true);
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["rename:allowed"]);
        Ok(())
    }

    #[test]
    fn legacy_rename_to_summary_disallowed() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_rename_to_summary(false);
        assert!(!summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn rename_plan_to_summary_empty() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old".to_string(),
            "new".to_string(),
            vec![],
            vec![],
            vec![],
        );
        let summary = super::rename_plan_to_summary(&plan);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn rename_plan_to_summary_with_edits_and_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old".to_string(),
            "new".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            vec![make_blocker(PlanBlockerReason::DynamicBoundary)],
            vec![],
        );
        let summary = super::rename_plan_to_summary(&plan);
        assert!(summary.available);
        assert_eq!(summary.match_count, 2);
        Ok(())
    }

    // ── Classify helper tests ──

    #[test]
    fn classify_rename_result_no_blockers_is_allowed() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old".to_string(),
            "new".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            vec![],
            vec![],
        );
        let result = super::classify_rename_result(plan);
        match result {
            RenameCutoverResult::Allowed { edits } => {
                assert_eq!(edits.len(), 1);
            }
            other => return Err(format!("expected Allowed, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn classify_rename_result_with_blockers_is_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old".to_string(),
            "new".to_string(),
            vec![],
            vec![make_blocker(PlanBlockerReason::DynamicBoundary)],
            vec![],
        );
        let result = super::classify_rename_result(plan);
        match result {
            RenameCutoverResult::Blocked { blockers, .. } => {
                assert_eq!(blockers.len(), 1);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }
}
