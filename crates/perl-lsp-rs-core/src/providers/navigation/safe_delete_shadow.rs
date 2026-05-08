//! Safe-delete shadow compare and cutover paths.
//!
//! Provides two entry points for safe-delete:
//!
//! 1. **Shadow mode** ([`safe_delete_shadow`]) — runs both legacy and
//!    semantic paths side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`safe_delete_cutover`]) — uses the semantic path
//!    as the primary source of truth:
//!    - *Allowed*: plan has no blockers → allow deletion.
//!    - *Blocked*: plan has blockers → block deletion and present blockers.
//!    - Ambiguous / Dynamic / Unavailable → block.
//!
//! # Requirements
//!
//! - **Req 9.7**: Safe-delete calls `SemanticQueries::safe_delete_plan`
//!   and blocks deletion when the plan contains blockers.
//! - **Req 17.5**: When the plan contains blockers, present them to the
//!   user and block deletion.
//! - **Req 22.9**: Safe-delete cutover: Exact (no refs) → allow;
//!   Ambiguous → block; Dynamic/Unavailable → block.

use perl_semantic_facts::{EntityId, PlanBlocker, SafeDeletePlan};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

/// Result of a shadow-compared safe-delete request.
///
/// Contains the legacy safe-delete result (which callers should use during
/// the shadow phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct SafeDeleteShadowResult {
    /// Whether the legacy path would allow the deletion.
    pub legacy_allowed: bool,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run safe-delete through both legacy and semantic paths, producing a
/// shadow-compare receipt.
///
/// # Arguments
///
/// * `legacy_allowed` — whether the legacy safe-delete path would allow
///   the deletion (caller is responsible for running the legacy logic).
/// * `semantic_queries` — the new semantic query facade.
/// * `entity_id` — the entity being considered for deletion.
/// * `symbol` — the symbol name (for receipt tracking).
///
/// # Returns
///
/// A [`SafeDeleteShadowResult`] containing the legacy result and a receipt.
/// The caller should return the legacy result to the LSP client during the
/// shadow phase.
pub fn safe_delete_shadow<Q: SemanticQueries>(
    legacy_allowed: bool,
    semantic_queries: &Q,
    entity_id: EntityId,
    symbol: &str,
) -> SafeDeleteShadowResult {
    // ── Legacy path ──
    let old_summary = legacy_safe_delete_to_summary(legacy_allowed);

    // ── New semantic path ──
    let plan = semantic_queries.safe_delete_plan(entity_id);
    let new_summary = safe_delete_plan_to_summary(&plan);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::SafeDeletePlan,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
    );

    tracing::debug!(
        entity_id = entity_id.0,
        symbol = %symbol,
        verdict = ?receipt.verdict,
        blockers = plan.blockers.len(),
        "safe-delete shadow compare"
    );

    SafeDeleteShadowResult { legacy_allowed, receipt }
}

// ── Cutover types ──

/// Classification of the semantic safe-delete result for cutover decisions.
///
/// Follows the fallback policy table (Req 22.9):
/// - Allowed (no refs) → allow deletion
/// - Blocked → present blockers to user, block deletion
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafeDeleteCutoverResult {
    /// Plan has no blockers — deletion is safe.
    Allowed,
    /// Plan has blockers — present them to the user and block deletion.
    Blocked {
        /// The blockers preventing the deletion.
        blockers: Vec<PlanBlocker>,
    },
}

/// Outcome of a cutover safe-delete request.
///
/// Contains the classified result and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct SafeDeleteCutoverOutcome {
    /// The classified cutover result.
    pub result: SafeDeleteCutoverResult,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run safe-delete with the semantic path as primary.
///
/// # Decision logic
///
/// 1. Call `SemanticQueries::safe_delete_plan` for the entity.
/// 2. Classify the result:
///    - **Allowed**: plan has no blockers → `SafeDeleteCutoverResult::Allowed`.
///    - **Blocked**: plan has blockers → `SafeDeleteCutoverResult::Blocked`.
/// 3. Emit a shadow-compare receipt regardless of outcome.
///
/// # Arguments
///
/// * `legacy_allowed` — whether the legacy safe-delete path would allow
///   the deletion.
/// * `semantic_queries` — the semantic query facade (primary path).
/// * `entity_id` — the entity being considered for deletion.
/// * `symbol` — the symbol name (for receipt tracking).
///
/// # Returns
///
/// A [`SafeDeleteCutoverOutcome`] with the classified result and receipt.
pub fn safe_delete_cutover<Q: SemanticQueries>(
    legacy_allowed: bool,
    semantic_queries: &Q,
    entity_id: EntityId,
    symbol: &str,
) -> SafeDeleteCutoverOutcome {
    // ── Semantic path (primary) ──
    let plan = semantic_queries.safe_delete_plan(entity_id);
    let new_summary = safe_delete_plan_to_summary(&plan);

    // ── Legacy path (for receipt) ──
    let old_summary = legacy_safe_delete_to_summary(legacy_allowed);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::SafeDeletePlan,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
    );

    // ── Classify result ──
    let result = classify_safe_delete_result(plan);

    tracing::debug!(
        entity_id = entity_id.0,
        symbol = %symbol,
        verdict = ?receipt.verdict,
        classification = match &result {
            SafeDeleteCutoverResult::Allowed => "allowed",
            SafeDeleteCutoverResult::Blocked { .. } => "blocked",
        },
        "safe-delete cutover"
    );

    SafeDeleteCutoverOutcome { result, receipt }
}

/// Classify a safe-delete plan into the cutover result category.
fn classify_safe_delete_result(plan: SafeDeletePlan) -> SafeDeleteCutoverResult {
    if plan.blockers.is_empty() {
        SafeDeleteCutoverResult::Allowed
    } else {
        SafeDeleteCutoverResult::Blocked { blockers: plan.blockers }
    }
}

/// Convert a legacy safe-delete decision into a [`ShadowResultSummary`].
fn legacy_safe_delete_to_summary(allowed: bool) -> ShadowResultSummary {
    if allowed {
        summarize_identities(Some(vec!["safe_delete:allowed".to_string()]))
    } else {
        summarize_identities(None)
    }
}

/// Convert a semantic [`SafeDeletePlan`] into a [`ShadowResultSummary`].
fn safe_delete_plan_to_summary(plan: &SafeDeletePlan) -> ShadowResultSummary {
    let mut identities: Vec<String> = Vec::new();

    // Blockers as identities
    for blocker in &plan.blockers {
        identities.push(format!("blocker:{:?}", blocker.reason));
    }

    // If no blockers, the plan allows deletion
    if plan.blockers.is_empty() {
        identities.push("safe_delete:allowed".to_string());
    }

    summarize_identities(Some(identities))
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact, PlanBlocker,
        PlanBlockerReason, RenamePlan, SafeDeletePlan, ScopeId, VisibleSymbol,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    // ── Minimal SemanticQueries stub for testing ──

    struct StubSemanticQueries {
        safe_delete_plan_result: SafeDeletePlan,
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

        fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
            RenamePlan::new(entity_id, String::new(), new_name.to_string(), vec![], vec![], vec![])
        }

        fn safe_delete_plan(&self, _entity_id: EntityId) -> SafeDeletePlan {
            self.safe_delete_plan_result.clone()
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

    fn make_blocker(reason: PlanBlockerReason) -> PlanBlocker {
        PlanBlocker::new(reason, None, format!("{reason:?} blocker"))
    }

    fn make_blocker_with_anchor(reason: PlanBlockerReason, anchor_id: u64) -> PlanBlocker {
        PlanBlocker::new(reason, Some(AnchorId(anchor_id)), format!("{reason:?} blocker"))
    }

    // ── Shadow mode tests ──

    #[test]
    fn shadow_legacy_allowed_no_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), vec![], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let result = safe_delete_shadow(true, &queries, EntityId(1), "my_sub");

        assert!(result.legacy_allowed);
        assert_eq!(result.receipt.query, ShadowQueryName::SafeDeletePlan);
        assert_eq!(result.receipt.input.symbol, "my_sub");
        assert!(result.receipt.old_result.available);
        assert!(result.receipt.new_result.available);
        Ok(())
    }

    #[test]
    fn shadow_legacy_disallowed_yields_unavailable_old() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(
            EntityId(1),
            "my_sub".to_string(),
            vec![make_blocker(PlanBlockerReason::ReferencesExist)],
            vec![],
        );
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let result = safe_delete_shadow(false, &queries, EntityId(1), "my_sub");

        assert!(!result.legacy_allowed);
        assert!(!result.receipt.old_result.available);
        assert!(result.receipt.new_result.available);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    #[test]
    fn shadow_receipt_uses_safe_delete_plan_query_name() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(EntityId(1), "test".to_string(), vec![], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let result = safe_delete_shadow(true, &queries, EntityId(1), "test");

        assert_eq!(result.receipt.query, ShadowQueryName::SafeDeletePlan);
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    // ── Cutover tests ──

    #[test]
    fn cutover_allowed_when_no_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), vec![], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "my_sub");

        assert_eq!(outcome.result, SafeDeleteCutoverResult::Allowed);
        assert_eq!(outcome.receipt.query, ShadowQueryName::SafeDeletePlan);
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_references_exist() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::ReferencesExist);
        let plan =
            SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), vec![blocker.clone()], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "my_sub");

        match &outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].reason, PlanBlockerReason::ReferencesExist);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_exported_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::ExportedSymbol);
        let plan =
            SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), vec![blocker.clone()], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "my_sub");

        match &outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::ExportedSymbol);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_imported_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::ImportedSymbol);
        let plan =
            SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), vec![blocker.clone()], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "my_sub");

        match &outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::ImportedSymbol);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_generated_member() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::GeneratedMember);
        let plan =
            SafeDeletePlan::new(EntityId(1), "accessor".to_string(), vec![blocker.clone()], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "accessor");

        match &outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::GeneratedMember);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_on_dynamic_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::DynamicBoundary);
        let plan =
            SafeDeletePlan::new(EntityId(1), "dyn_sub".to_string(), vec![blocker.clone()], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "dyn_sub");

        match &outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::DynamicBoundary);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_blocked_multiple_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let blockers = vec![
            make_blocker(PlanBlockerReason::ReferencesExist),
            make_blocker(PlanBlockerReason::ExportedSymbol),
            make_blocker_with_anchor(PlanBlockerReason::ImportedSymbol, 42),
        ];
        let plan = SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), blockers.clone(), vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "my_sub");

        match &outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers: result_blockers } => {
                assert_eq!(result_blockers.len(), 3);
                assert_eq!(result_blockers[0].reason, PlanBlockerReason::ReferencesExist);
                assert_eq!(result_blockers[1].reason, PlanBlockerReason::ExportedSymbol);
                assert_eq!(result_blockers[2].reason, PlanBlockerReason::ImportedSymbol);
                assert_eq!(result_blockers[2].anchor_id, Some(AnchorId(42)));
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(
            EntityId(1),
            "my_sub".to_string(),
            vec![
                make_blocker(PlanBlockerReason::ReferencesExist),
                make_blocker(PlanBlockerReason::ExportedSymbol),
            ],
            vec![],
        );
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "my_sub");

        // Receipt should reflect blockers.
        assert_eq!(outcome.receipt.new_result.match_count, 2);
        assert_eq!(outcome.receipt.query, ShadowQueryName::SafeDeletePlan);
        Ok(())
    }

    // ── Summary helper tests ──

    #[test]
    fn legacy_safe_delete_to_summary_allowed() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_safe_delete_to_summary(true);
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["safe_delete:allowed"]);
        Ok(())
    }

    #[test]
    fn legacy_safe_delete_to_summary_disallowed() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_safe_delete_to_summary(false);
        assert!(!summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn safe_delete_plan_to_summary_no_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(EntityId(1), "test".to_string(), vec![], vec![]);
        let summary = super::safe_delete_plan_to_summary(&plan);
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["safe_delete:allowed"]);
        Ok(())
    }

    #[test]
    fn safe_delete_plan_to_summary_with_blockers() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(
            EntityId(1),
            "test".to_string(),
            vec![make_blocker(PlanBlockerReason::ReferencesExist)],
            vec![],
        );
        let summary = super::safe_delete_plan_to_summary(&plan);
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        Ok(())
    }

    // ── Classify helper tests ──

    #[test]
    fn classify_safe_delete_result_no_blockers_is_allowed() -> Result<(), Box<dyn std::error::Error>>
    {
        let plan = SafeDeletePlan::new(EntityId(1), "test".to_string(), vec![], vec![]);
        let result = super::classify_safe_delete_result(plan);
        assert_eq!(result, SafeDeleteCutoverResult::Allowed);
        Ok(())
    }

    #[test]
    fn classify_safe_delete_result_with_blockers_is_blocked()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(
            EntityId(1),
            "test".to_string(),
            vec![make_blocker(PlanBlockerReason::ReferencesExist)],
            vec![],
        );
        let result = super::classify_safe_delete_result(plan);
        match result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers.len(), 1);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        Ok(())
    }
}
