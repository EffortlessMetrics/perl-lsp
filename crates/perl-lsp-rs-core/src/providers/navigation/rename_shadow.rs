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

use perl_semantic_facts::{
    Confidence, EntityId, PlanBlocker, PlanBlockerReason, PlannedEdit, PlannedEditCategory,
    Provenance, ProviderFactFreshness, ProviderFactSourceKind, ProviderFactTrace,
    ProviderFallbackState, ProviderSurface, RenamePlan,
};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

use super::refactor_receipt_helpers::{blocker_reason_list, blocker_ux_list};

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
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::RenamePlan,
        ShadowQueryInput { symbol: new_name.to_string() },
        old_summary,
        new_summary,
        rename_plan_receipt_notes(legacy_allowed, &plan, "shadow"),
        rename_plan_fact_source_traces(&plan, ProviderFallbackState::Shadow),
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

/// Classification for a package/compiler-backed rename pilot proof.
///
/// This is intentionally narrower than [`RenameCutoverResult`]. It records
/// whether a semantic rename plan is eligible for a future scoped pilot, but it
/// does not broaden live rename behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenamePackagePilotResult {
    /// The plan is eligible for the scoped pilot proof.
    Eligible {
        /// The source-backed definition/reference edits covered by the proof.
        edits: Vec<PlannedEdit>,
    },
    /// The plan is not eligible for the scoped pilot proof.
    Ineligible {
        /// Why the plan is outside the pilot envelope.
        reason: RenamePackagePilotIneligibleReason,
        /// Any planned edits found in the semantic plan.
        edits: Vec<PlannedEdit>,
        /// Any blockers found in the semantic plan.
        blockers: Vec<PlanBlocker>,
    },
}

/// Why a package/compiler-backed rename plan is outside the pilot proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenamePackagePilotIneligibleReason {
    /// No edits were produced, so there is no source-backed action to prove.
    EmptyPlan,
    /// One or more blockers prevent authorizing the edit.
    Blocked,
    /// The plan contains edit categories outside source-backed definition and
    /// reference replacement.
    UnsupportedEditCategory,
}

/// Outcome of a package/compiler-backed rename pilot proof request.
#[derive(Debug)]
#[non_exhaustive]
pub struct RenamePackagePilotOutcome {
    /// The pilot-specific classification.
    pub result: RenamePackagePilotResult,
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
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::RenamePlan,
        ShadowQueryInput { symbol: new_name.to_string() },
        old_summary,
        new_summary,
        rename_plan_receipt_notes(legacy_allowed, &plan, "cutover"),
        rename_plan_fact_source_traces(&plan, ProviderFallbackState::Primary),
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

/// Classify a semantic rename plan for a future package/compiler-backed pilot.
///
/// This reuses the cutover receipt path but applies a narrower policy envelope:
/// source-backed definition/reference edits only, no blockers, and at least one
/// edit. The returned proof is not sufficient on its own for live package
/// rename authorization; live callers must separately prove the materialized
/// edit set, source-backed anchors, and workspace ambiguity guard.
pub fn rename_package_pilot_proof<Q: SemanticQueries>(
    legacy_allowed: bool,
    semantic_queries: &Q,
    entity_id: EntityId,
    new_name: &str,
) -> RenamePackagePilotOutcome {
    let cutover = rename_cutover(legacy_allowed, semantic_queries, entity_id, new_name);
    let result = classify_package_pilot_result(cutover.result);
    let mut receipt = cutover.receipt;
    receipt.notes.push(package_pilot_receipt_note(&result));

    RenamePackagePilotOutcome { result, receipt }
}

/// Classify a rename plan into the cutover result category.
fn classify_rename_result(plan: RenamePlan) -> RenameCutoverResult {
    if plan.blockers.is_empty() {
        RenameCutoverResult::Allowed { edits: plan.edits }
    } else {
        RenameCutoverResult::Blocked { blockers: plan.blockers, edits: plan.edits }
    }
}

fn classify_package_pilot_result(result: RenameCutoverResult) -> RenamePackagePilotResult {
    match result {
        RenameCutoverResult::Blocked { blockers, edits } => RenamePackagePilotResult::Ineligible {
            reason: RenamePackagePilotIneligibleReason::Blocked,
            edits,
            blockers,
        },
        RenameCutoverResult::Allowed { edits } if edits.is_empty() => {
            RenamePackagePilotResult::Ineligible {
                reason: RenamePackagePilotIneligibleReason::EmptyPlan,
                edits,
                blockers: Vec::new(),
            }
        }
        RenameCutoverResult::Allowed { edits }
            if edits.iter().all(is_package_pilot_edit_category) =>
        {
            RenamePackagePilotResult::Eligible { edits }
        }
        RenameCutoverResult::Allowed { edits } => RenamePackagePilotResult::Ineligible {
            reason: RenamePackagePilotIneligibleReason::UnsupportedEditCategory,
            edits,
            blockers: Vec::new(),
        },
    }
}

fn is_package_pilot_edit_category(edit: &PlannedEdit) -> bool {
    matches!(edit.category, PlannedEditCategory::Definition | PlannedEditCategory::Reference)
}

fn package_pilot_receipt_note(result: &RenamePackagePilotResult) -> String {
    let (eligible, reason, edit_count, blocker_count) = match result {
        RenamePackagePilotResult::Eligible { edits } => ("true", "none", edits.len(), 0),
        RenamePackagePilotResult::Ineligible { reason, edits, blockers } => {
            ("false", reason.label(), edits.len(), blockers.len())
        }
    };

    format!(
        "rename package pilot proof: eligible={eligible}; reason={reason}; edit_count={edit_count}; blocker_count={blocker_count}; claim_boundary=receipt-only package/compiler-backed pilot; no_live_rename_cutover=true"
    )
}

impl RenamePackagePilotIneligibleReason {
    fn label(self) -> &'static str {
        match self {
            Self::EmptyPlan => "empty_plan",
            Self::Blocked => "blocked",
            Self::UnsupportedEditCategory => "unsupported_edit_category",
        }
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

fn rename_plan_receipt_notes(legacy_allowed: bool, plan: &RenamePlan, phase: &str) -> Vec<String> {
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
        "rename {phase} receipt: legacy_allowed={legacy_allowed}; compiler_plan_edits={}; blocker_count={}; blocker_reasons={blocker_reasons}; dynamic_boundary={has_dynamic_boundary}; generated_member={has_generated_member}; stale_fact={has_stale_fact}; low_confidence={has_low_confidence}; fallback_state={fallback_state}; blocker_ux={}",
        plan.edits.len(),
        plan.blockers.len(),
        blocker_ux_list(&plan.blockers)
    )]
}

fn rename_plan_fact_source_traces(
    plan: &RenamePlan,
    allowed_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    let edit_state =
        if plan.blockers.is_empty() { allowed_state } else { ProviderFallbackState::Unavailable };
    let mut traces: Vec<ProviderFactTrace> = plan
        .edits
        .iter()
        .map(|edit| edit_fact_trace(edit, edit_state))
        .chain(plan.blockers.iter().map(blocker_fact_trace))
        .collect();
    if traces.is_empty() {
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Rename,
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::NotApplicable,
            ProviderFallbackState::Fallback,
            Some("rename-plan".to_string()),
            None,
            Some(1),
        ));
    }
    traces
}

fn edit_fact_trace(edit: &PlannedEdit, fallback_state: ProviderFallbackState) -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::Rename,
        ProviderFactSourceKind::SemanticFact,
        Provenance::ExactAst,
        Confidence::High,
        ProviderFactFreshness::Fresh,
        fallback_state,
        Some("rename-plan".to_string()),
        Some(edit.anchor_id),
        Some(1),
    )
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
        ProviderSurface::Rename,
        source,
        provenance,
        confidence,
        freshness,
        ProviderFallbackState::Blocked,
        Some("rename-plan".to_string()),
        blocker.anchor_id,
        Some(1),
    )
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

    fn first_trace<'a>(
        receipt: &'a SemanticShadowCompareReceipt,
    ) -> Result<&'a ProviderFactTrace, Box<dyn std::error::Error>> {
        match receipt.fact_source_traces.first() {
            Some(trace) => Ok(trace),
            None => Err("missing fact-source trace".into()),
        }
    }

    fn trace_for_source<'a>(
        receipt: &'a SemanticShadowCompareReceipt,
        source: ProviderFactSourceKind,
    ) -> Result<&'a ProviderFactTrace, Box<dyn std::error::Error>> {
        for trace in &receipt.fact_source_traces {
            if trace.source == source {
                return Ok(trace);
            }
        }
        Err(format!("missing {source:?} trace").into())
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
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    #[test]
    fn rename_compiler_boundaries_exact_static_trace_allows_local_rename()
    -> Result<(), Box<dyn std::error::Error>> {
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
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::Rename);
        assert_eq!(trace.source, ProviderFactSourceKind::SemanticFact);
        assert_eq!(trace.provenance, Provenance::ExactAst);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        assert_eq!(trace.anchor_id, Some(AnchorId(10)));
        Ok(())
    }

    #[test]
    fn rename_compiler_boundaries_dynamic_boundary_blocks_and_marks_edits_unavailable()
    -> Result<(), Box<dyn std::error::Error>> {
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
        let edit_trace = trace_for_source(&outcome.receipt, ProviderFactSourceKind::SemanticFact)?;
        let blocker_trace =
            trace_for_source(&outcome.receipt, ProviderFactSourceKind::DynamicBoundary)?;

        match outcome.result {
            RenameCutoverResult::Blocked { blockers, .. } => {
                assert_eq!(blockers[0].reason, PlanBlockerReason::DynamicBoundary);
            }
            other => return Err(format!("expected Blocked, got {:?}", other).into()),
        }
        assert_eq!(edit_trace.fallback_state, ProviderFallbackState::Unavailable);
        assert_eq!(blocker_trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(blocker_trace.confidence, Confidence::High);
        assert_eq!(blocker_trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn rename_runtime_blocker_ux_receipt_labels_dynamic_generated_and_low_confidence()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            vec![
                PlanBlocker::new(
                    PlanBlockerReason::DynamicBoundary,
                    None,
                    "symbolic reference may target this symbol".to_string(),
                ),
                PlanBlocker::new(
                    PlanBlockerReason::GeneratedMember,
                    None,
                    "generated member needs a framework-aware edit plan".to_string(),
                ),
                PlanBlocker::new(
                    PlanBlockerReason::AmbiguousReference,
                    None,
                    "ambiguous reference has multiple candidates".to_string(),
                ),
                PlanBlocker::new(
                    PlanBlockerReason::StaleFact,
                    None,
                    "compiler fact is stale and must be refreshed".to_string(),
                ),
            ],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");
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
            notes.contains("symbolic reference may target this symbol")
                && notes.contains("framework-aware edit plan")
                && notes.contains("ambiguous reference has multiple candidates")
                && notes.contains("compiler fact is stale"),
            "missing user-facing blocker descriptions in {}",
            notes
        );
        Ok(())
    }

    #[test]
    fn rename_compiler_boundaries_generated_member_uses_framework_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "accessor".to_string(),
            "renamed".to_string(),
            vec![],
            vec![make_blocker(PlanBlockerReason::GeneratedMember)],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "renamed");
        let trace = first_trace(&outcome.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::Rename);
        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn rename_compiler_boundaries_ambiguous_reference_is_low_confidence_blocker()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![],
            vec![make_blocker(PlanBlockerReason::AmbiguousReference)],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");
        let trace = first_trace(&outcome.receipt)?;

        assert_eq!(trace.source, ProviderFactSourceKind::SemanticFact);
        assert_eq!(trace.provenance, Provenance::NameHeuristic);
        assert_eq!(trace.confidence, Confidence::Low);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn rename_compiler_boundaries_stale_fact_uses_stale_compiler_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![],
            vec![make_blocker(PlanBlockerReason::StaleFact)],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_cutover(true, &queries, EntityId(1), "new_name");
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

    #[test]
    fn rename_compiler_boundaries_empty_plan_falls_back() -> Result<(), Box<dyn std::error::Error>>
    {
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
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::Rename);
        assert_eq!(trace.source, ProviderFactSourceKind::Fallback);
        assert_eq!(trace.provenance, Provenance::SearchFallback);
        assert_eq!(trace.freshness, ProviderFactFreshness::NotApplicable);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Fallback);
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
    fn rename_package_pilot_allows_source_backed_definition_and_reference()
    -> Result<(), Box<dyn std::error::Error>> {
        let edits = vec![
            make_edit(10, PlannedEditCategory::Definition),
            make_edit(20, PlannedEditCategory::Reference),
        ];
        let plan = RenamePlan::new(
            EntityId(1),
            "Old::Name".to_string(),
            "New::Name".to_string(),
            edits.clone(),
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_package_pilot_proof(true, &queries, EntityId(1), "New::Name");

        match &outcome.result {
            RenamePackagePilotResult::Eligible { edits: result_edits } => {
                assert_eq!(*result_edits, edits);
            }
            other => return Err(format!("expected Eligible, got {:?}", other).into()),
        }
        let notes = outcome.receipt.notes.join(" ");
        assert!(notes.contains("eligible=true"), "missing eligible note in {}", notes);
        assert!(
            notes.contains("claim_boundary=receipt-only package/compiler-backed pilot"),
            "missing claim boundary in {}",
            notes
        );
        assert!(
            notes.contains("no_live_rename_cutover=true"),
            "missing no-live boundary in {}",
            notes
        );
        for trace in &outcome.receipt.fact_source_traces {
            assert_eq!(trace.surface, ProviderSurface::Rename);
            assert_eq!(trace.source, ProviderFactSourceKind::SemanticFact);
            assert_eq!(trace.provenance, Provenance::ExactAst);
            assert_eq!(trace.confidence, Confidence::High);
            assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
            assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        }
        Ok(())
    }

    #[test]
    fn rename_package_pilot_rejects_empty_plan() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(1),
            "Old::Name".to_string(),
            "New::Name".to_string(),
            vec![],
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_package_pilot_proof(true, &queries, EntityId(1), "New::Name");

        match &outcome.result {
            RenamePackagePilotResult::Ineligible { reason, edits, blockers } => {
                assert_eq!(*reason, RenamePackagePilotIneligibleReason::EmptyPlan);
                assert!(edits.is_empty());
                assert!(blockers.is_empty());
            }
            other => return Err(format!("expected Ineligible, got {:?}", other).into()),
        }
        let notes = outcome.receipt.notes.join(" ");
        assert!(notes.contains("eligible=false"), "missing ineligible note in {}", notes);
        assert!(notes.contains("reason=empty_plan"), "missing reason in {}", notes);
        Ok(())
    }

    #[test]
    fn rename_package_pilot_rejects_import_and_export_edits()
    -> Result<(), Box<dyn std::error::Error>> {
        let edits = vec![
            make_edit(30, PlannedEditCategory::ImportList),
            make_edit(40, PlannedEditCategory::ExportList),
        ];
        let plan = RenamePlan::new(
            EntityId(1),
            "Old::Name".to_string(),
            "New::Name".to_string(),
            edits,
            vec![],
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_package_pilot_proof(true, &queries, EntityId(1), "New::Name");

        match &outcome.result {
            RenamePackagePilotResult::Ineligible { reason, blockers, .. } => {
                assert_eq!(*reason, RenamePackagePilotIneligibleReason::UnsupportedEditCategory);
                assert!(blockers.is_empty());
            }
            other => return Err(format!("expected Ineligible, got {:?}", other).into()),
        }
        let notes = outcome.receipt.notes.join(" ");
        assert!(
            notes.contains("reason=unsupported_edit_category"),
            "missing unsupported category reason in {}",
            notes
        );
        Ok(())
    }

    #[test]
    fn rename_package_pilot_rejects_dynamic_generated_stale_and_low_confidence_blockers()
    -> Result<(), Box<dyn std::error::Error>> {
        let blockers = vec![
            make_blocker(PlanBlockerReason::DynamicBoundary),
            make_blocker(PlanBlockerReason::GeneratedMember),
            make_blocker(PlanBlockerReason::StaleFact),
            make_blocker(PlanBlockerReason::AmbiguousReference),
        ];
        let plan = RenamePlan::new(
            EntityId(1),
            "Old::Name".to_string(),
            "New::Name".to_string(),
            vec![make_edit(10, PlannedEditCategory::Definition)],
            blockers,
            vec![],
        );
        let queries = StubSemanticQueries { rename_plan_result: plan };

        let outcome = rename_package_pilot_proof(true, &queries, EntityId(1), "New::Name");

        match &outcome.result {
            RenamePackagePilotResult::Ineligible { reason, blockers, .. } => {
                assert_eq!(*reason, RenamePackagePilotIneligibleReason::Blocked);
                assert_eq!(blockers.len(), 4);
            }
            other => return Err(format!("expected Ineligible, got {:?}", other).into()),
        }
        let notes = outcome.receipt.notes.join(" ");
        assert!(notes.contains("reason=blocked"), "missing blocked reason in {}", notes);
        assert!(notes.contains("dynamic_boundary=true"), "missing dynamic note in {}", notes);
        assert!(notes.contains("generated_member=true"), "missing generated note in {}", notes);
        assert!(notes.contains("stale_fact=true"), "missing stale note in {}", notes);
        assert!(notes.contains("low_confidence=true"), "missing low-confidence note in {}", notes);
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
