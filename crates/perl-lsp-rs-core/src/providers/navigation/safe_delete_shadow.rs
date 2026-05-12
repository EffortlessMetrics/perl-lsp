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

use perl_semantic_facts::{
    Confidence, EntityId, PlanBlocker, PlanBlockerReason, Provenance, ProviderFactFreshness,
    ProviderFactSourceKind, ProviderFactTrace, ProviderFallbackState, ProviderSurface,
    SafeDeletePlan,
};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

use super::refactor_receipt_helpers::{blocker_reason_list, blocker_ux_list};

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
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::SafeDeletePlan,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        safe_delete_plan_receipt_notes(legacy_allowed, &plan, "shadow"),
        safe_delete_plan_fact_source_traces(&plan, ProviderFallbackState::Shadow),
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
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::SafeDeletePlan,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        safe_delete_plan_receipt_notes(legacy_allowed, &plan, "cutover"),
        safe_delete_plan_fact_source_traces(&plan, ProviderFallbackState::Primary),
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

fn safe_delete_plan_receipt_notes(
    legacy_allowed: bool,
    plan: &SafeDeletePlan,
    phase: &str,
) -> Vec<String> {
    let blocker_reasons = blocker_reason_list(&plan.blockers);
    let has_dynamic_boundary =
        plan.blockers.iter().any(|blocker| blocker.reason == PlanBlockerReason::DynamicBoundary);
    let has_generated_member =
        plan.blockers.iter().any(|blocker| blocker.reason == PlanBlockerReason::GeneratedMember);
    let has_stale_fact =
        plan.blockers.iter().any(|blocker| blocker.reason == PlanBlockerReason::StaleFact);
    let has_low_confidence = plan.blockers.iter().any(|blocker| {
        matches!(
            blocker.reason,
            PlanBlockerReason::AmbiguousReference | PlanBlockerReason::UnclassifiedOccurrence
        )
    });
    let fallback_state = if plan.blockers.is_empty() { "allowed" } else { "blocked" };

    vec![format!(
        "safe-delete {phase} receipt: legacy_allowed={legacy_allowed}; compiler_plan_safe={}; blocker_count={}; blocker_reasons={blocker_reasons}; dynamic_boundary={has_dynamic_boundary}; generated_member={has_generated_member}; stale_fact={has_stale_fact}; low_confidence={has_low_confidence}; fallback_state={fallback_state}; blocker_ux={}",
        plan.blockers.is_empty(),
        plan.blockers.len(),
        blocker_ux_list(&plan.blockers)
    )]
}

fn safe_delete_plan_fact_source_traces(
    plan: &SafeDeletePlan,
    allowed_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    if plan.blockers.is_empty() {
        return vec![ProviderFactTrace::new(
            ProviderSurface::SafeDelete,
            ProviderFactSourceKind::SemanticFact,
            Provenance::ExactAst,
            Confidence::High,
            ProviderFactFreshness::Fresh,
            allowed_state,
            Some("safe-delete-plan".to_string()),
            None,
            Some(1),
        )];
    }
    plan.blockers.iter().map(blocker_fact_trace).collect()
}

fn blocker_fact_trace(blocker: &PlanBlocker) -> ProviderFactTrace {
    let (source, provenance, confidence, freshness) = match blocker.reason {
        PlanBlockerReason::DynamicBoundary => (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::High,
            ProviderFactFreshness::Fresh,
        ),
        PlanBlockerReason::GeneratedMember => (
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            Confidence::High,
            ProviderFactFreshness::Fresh,
        ),
        PlanBlockerReason::StaleFact => (
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Low,
            ProviderFactFreshness::Stale,
        ),
        PlanBlockerReason::CrossModuleExport
        | PlanBlockerReason::ImportedSymbol
        | PlanBlockerReason::ExportedSymbol => (
            ProviderFactSourceKind::CompilerFact,
            Provenance::ImportExportInference,
            Confidence::High,
            ProviderFactFreshness::Fresh,
        ),
        PlanBlockerReason::AmbiguousReference => (
            ProviderFactSourceKind::SemanticFact,
            Provenance::NameHeuristic,
            Confidence::Low,
            ProviderFactFreshness::Fresh,
        ),
        PlanBlockerReason::ReferencesExist => (
            ProviderFactSourceKind::SemanticFact,
            Provenance::SemanticAnalyzer,
            Confidence::High,
            ProviderFactFreshness::Fresh,
        ),
        PlanBlockerReason::UnclassifiedOccurrence => (
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::Unknown,
        ),
        _ => (
            ProviderFactSourceKind::Unknown,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::Unknown,
        ),
    };
    ProviderFactTrace::new(
        ProviderSurface::SafeDelete,
        source,
        provenance,
        confidence,
        freshness,
        ProviderFallbackState::Blocked,
        Some("safe-delete-plan".to_string()),
        blocker.anchor_id,
        Some(1),
    )
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

    fn first_trace<'a>(
        receipt: &'a SemanticShadowCompareReceipt,
    ) -> Result<&'a ProviderFactTrace, Box<dyn std::error::Error>> {
        match receipt.fact_source_traces.first() {
            Some(trace) => Ok(trace),
            None => Err("missing fact-source trace".into()),
        }
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

    #[test]
    fn safe_delete_compiler_boundaries_exact_static_trace_allows_unreferenced_delete()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(EntityId(1), "my_sub".to_string(), vec![], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let result = safe_delete_shadow(true, &queries, EntityId(1), "my_sub");
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::SafeDelete);
        assert_eq!(trace.source, ProviderFactSourceKind::SemanticFact);
        assert_eq!(trace.provenance, Provenance::ExactAst);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn safe_delete_compiler_boundaries_dynamic_boundary_blocks_delete()
    -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::DynamicBoundary);
        let plan = SafeDeletePlan::new(EntityId(1), "dyn_sub".to_string(), vec![blocker], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "dyn_sub");
        let trace = first_trace(&outcome.receipt)?;

        match outcome.result {
            SafeDeleteCutoverResult::Blocked { blockers } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::DynamicBoundary);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        assert_eq!(trace.surface, ProviderSurface::SafeDelete);
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn safe_delete_runtime_blocker_ux_receipt_labels_dynamic_generated_and_low_confidence()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(
            EntityId(1),
            "accessor".to_string(),
            vec![
                PlanBlocker::new(
                    PlanBlockerReason::DynamicBoundary,
                    None,
                    "AUTOLOAD may dispatch to this symbol".to_string(),
                ),
                PlanBlocker::new(
                    PlanBlockerReason::GeneratedMember,
                    None,
                    "framework-generated member needs a generator-aware delete plan".to_string(),
                ),
                PlanBlocker::new(
                    PlanBlockerReason::AmbiguousReference,
                    None,
                    "low-confidence reference could not be classified".to_string(),
                ),
                PlanBlocker::new(
                    PlanBlockerReason::StaleFact,
                    None,
                    "compiler fact is stale and must be refreshed".to_string(),
                ),
            ],
            vec![],
        );
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "accessor");
        let notes = outcome.receipt.notes.join(" ");

        assert!(notes.contains("blocker_count=4"), "missing blocker count in {}", notes);
        assert!(
            notes.contains(
                "blocker_reasons=dynamic_boundary,generated_member,ambiguous_reference,stale_fact"
            ),
            "missing blocker reasons in {}",
            notes
        );
        assert!(notes.contains("dynamic_boundary=true"), "missing dynamic boundary in {}", notes);
        assert!(notes.contains("generated_member=true"), "missing generated member in {}", notes);
        assert!(notes.contains("stale_fact=true"), "missing stale fact in {}", notes);
        assert!(notes.contains("low_confidence=true"), "missing low confidence in {}", notes);
        assert!(
            notes.contains("AUTOLOAD may dispatch")
                && notes.contains("generator-aware delete plan")
                && notes.contains("low-confidence reference")
                && notes.contains("compiler fact is stale"),
            "missing user-facing blocker descriptions in {}",
            notes
        );
        Ok(())
    }

    #[test]
    fn safe_delete_compiler_boundaries_generated_member_uses_framework_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::GeneratedMember);
        let plan = SafeDeletePlan::new(EntityId(1), "accessor".to_string(), vec![blocker], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "accessor");
        let trace = first_trace(&outcome.receipt)?;

        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn safe_delete_compiler_boundaries_import_export_blockers_use_compiler_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker_with_anchor(PlanBlockerReason::ExportedSymbol, 42);
        let plan = SafeDeletePlan::new(EntityId(1), "exported".to_string(), vec![blocker], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "exported");
        let trace = first_trace(&outcome.receipt)?;

        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::ImportExportInference);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.anchor_id, Some(AnchorId(42)));
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn safe_delete_compiler_boundaries_ambiguous_reference_is_low_confidence_blocker()
    -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::AmbiguousReference);
        let plan = SafeDeletePlan::new(EntityId(1), "ambiguous".to_string(), vec![blocker], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "ambiguous");
        let trace = first_trace(&outcome.receipt)?;

        assert_eq!(trace.source, ProviderFactSourceKind::SemanticFact);
        assert_eq!(trace.provenance, Provenance::NameHeuristic);
        assert_eq!(trace.confidence, Confidence::Low);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn safe_delete_compiler_boundaries_stale_fact_uses_stale_compiler_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let blocker = make_blocker(PlanBlockerReason::StaleFact);
        let plan = SafeDeletePlan::new(EntityId(1), "stale".to_string(), vec![blocker], vec![]);
        let queries = StubSemanticQueries { safe_delete_plan_result: plan };

        let outcome = safe_delete_cutover(true, &queries, EntityId(1), "stale");
        let trace = first_trace(&outcome.receipt)?;
        let notes = outcome.receipt.notes.join(" ");

        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Low);
        assert_eq!(trace.freshness, ProviderFactFreshness::Stale);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        assert!(notes.contains("stale_fact=true"), "missing stale fact in {}", notes);
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
