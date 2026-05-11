//! Diagnostics shadow compare and cutover paths.
//!
//! Provides two entry points for undefined-symbol diagnostics:
//!
//! 1. **Shadow mode** ([`diagnostics_undefined_symbol_shadow`]) -- runs both
//!    legacy diagnostic emission and new `SemanticQueries::definitions` check
//!    side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`diagnostics_undefined_symbol_cutover`]) -- uses the
//!    semantic path as the primary source of truth:
//!    - *Exact*: symbol is confidently undefined -> emit warning diagnostic.
//!    - *Ambiguous*: multiple candidates or low confidence -> suppress or
//!      emit weak warning.
//!    - *Dynamic / Unavailable*: dynamic boundary or symbol found -> suppress.
//!
//! # Requirements
//!
//! - **Req 7.4**: Suppress undefined-symbol diagnostics for references within
//!   dynamic boundary scopes.
//! - **Req 9.4**: Diagnostics calls `SemanticQueries` to verify symbol
//!   definitions before emitting undefined-symbol diagnostics.
//! - **Req 10.1**: Maintain existing query path as fallback during validation.
//! - **Req 10.2**: Shadow-compare runs both old and new paths, producing
//!   deterministic receipts.
//! - **Req 10.9**: Scorecard gate: imported-symbol false positives=0,
//!   dynamic-boundary exact warnings=0.
//! - **Req 22.6**: Exact -> warn; Ambiguous -> suppress / weak warning;
//!   Dynamic/Unavailable -> suppress.

use perl_semantic_facts::{
    Confidence, DefinitionCandidate, FileId, Provenance, ProviderFactFreshness,
    ProviderFactSourceKind, ProviderFactTrace, ProviderFallbackState, ProviderSurface, ScopeId,
};
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

// ── Shadow result ──

/// Result of a shadow-compared undefined-symbol diagnostics check.
///
/// Contains the legacy diagnostic decision (which callers should use during
/// the shadow phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct DiagnosticsShadowResult {
    /// Legacy decision -- whether the legacy path would emit an undefined-symbol
    /// diagnostic. Callers should use this during the shadow phase.
    pub legacy_should_warn: bool,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run undefined-symbol diagnostics through both legacy and semantic paths,
/// producing a shadow-compare receipt.
///
/// # Arguments
///
/// * `legacy_should_warn` -- whether the legacy diagnostics path would emit
///   an undefined-symbol warning for this symbol (passed in because the
///   legacy decision is provider-internal).
/// * `semantic_queries` -- the new semantic query facade.
/// * `symbol` -- the symbol name being checked.
/// * `file_id` -- file containing the symbol reference.
/// * `scope_id` -- scope enclosing the reference, when known.
/// * `byte_offset` -- byte offset of the reference within the file.
///
/// # Returns
///
/// A [`DiagnosticsShadowResult`] containing the legacy decision and a receipt.
/// The caller should use the legacy decision during the shadow phase.
pub fn diagnostics_undefined_symbol_shadow<Q: SemanticQueries>(
    legacy_should_warn: bool,
    semantic_queries: &Q,
    symbol: &str,
    file_id: FileId,
    scope_id: Option<ScopeId>,
    byte_offset: u32,
) -> DiagnosticsShadowResult {
    // Legacy path
    let old_summary = legacy_warn_to_summary(legacy_should_warn);

    // New semantic path
    let context = QueryContext::new(file_id, scope_id, Some(byte_offset));
    let candidates = semantic_queries.definitions(symbol, &context);
    let classification = classify_diagnostic_result(&candidates);
    let new_summary = classification_to_summary(&classification, symbol);

    // Build receipt
    let fact_source_traces =
        diagnostics_fact_source_traces(&classification, &candidates, ProviderFallbackState::Shadow);
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::DiagnosticsCheck,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
        fact_source_traces,
    );

    tracing::debug!(
        symbol = %symbol,
        verdict = ?receipt.verdict,
        legacy_warn = legacy_should_warn,
        classification = ?classification,
        "diagnostics undefined-symbol shadow compare"
    );

    DiagnosticsShadowResult { legacy_should_warn, receipt }
}

// ── Cutover types ──

/// Classification of the semantic definition check for diagnostics decisions.
///
/// Follows the fallback policy table (Req 22.6):
/// - Exact: confident answer that symbol is undefined -> warn
/// - Ambiguous: uncertain (low-confidence candidates only) -> weak warn
/// - DynamicOrUnavailable: dynamic boundary, symbol found, or unavailable -> suppress
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticClassification {
    /// Semantic query gave a confident answer: no definition candidates
    /// found, so the symbol is genuinely undefined. Emit warning.
    Exact,
    /// Semantic query found only low-confidence or mixed-provenance
    /// candidates. Uncertain whether symbol is truly undefined.
    /// Emit weak warning or suppress.
    Ambiguous,
    /// Symbol is within a dynamic boundary, is defined with high
    /// confidence, or no semantic data is available. Suppress.
    DynamicOrUnavailable,
}

/// Action to take for an undefined-symbol diagnostic after cutover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticAction {
    /// Emit the undefined-symbol diagnostic as a warning.
    Warn,
    /// Emit a weak/informational diagnostic (ambiguous case).
    WeakWarn,
    /// Suppress the diagnostic entirely.
    Suppress,
}

/// Outcome of a cutover undefined-symbol diagnostics check.
///
/// Contains the action to take and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct DiagnosticsCutoverOutcome {
    /// The action to take for this undefined-symbol diagnostic.
    pub action: DiagnosticAction,
    /// The classification that led to this action.
    pub classification: DiagnosticClassification,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run undefined-symbol diagnostics with the semantic path as primary.
///
/// # Decision logic
///
/// 1. If the reference is within a dynamic boundary scope, suppress
///    immediately (Req 7.4).
/// 2. Call `SemanticQueries::definitions` for the symbol.
/// 3. Classify the result:
///    - No candidates found -> symbol is undefined -> `Warn`.
///    - All candidates are dynamic-boundary -> `Suppress`.
///    - Only low-confidence candidates -> `WeakWarn`.
///    - Usable candidates found (symbol IS defined) -> `Suppress`
///      (no false-positive undefined-symbol diagnostic).
/// 4. Emit a shadow-compare receipt regardless of outcome.
///
/// # Arguments
///
/// * `legacy_should_warn` -- whether the legacy path would emit a warning.
/// * `semantic_queries` -- the semantic query facade (primary path).
/// * `symbol` -- the symbol name being checked.
/// * `file_id` -- file containing the symbol reference.
/// * `scope_id` -- scope enclosing the reference, when known.
/// * `byte_offset` -- byte offset of the reference within the file.
/// * `is_in_dynamic_scope` -- whether the reference is within a dynamic
///   boundary scope (e.g., inside `eval`, `AUTOLOAD`, symbolic deref).
///
/// # Returns
///
/// A [`DiagnosticsCutoverOutcome`] with the action and receipt.
pub fn diagnostics_undefined_symbol_cutover<Q: SemanticQueries>(
    legacy_should_warn: bool,
    semantic_queries: &Q,
    symbol: &str,
    file_id: FileId,
    scope_id: Option<ScopeId>,
    byte_offset: u32,
    is_in_dynamic_scope: bool,
) -> DiagnosticsCutoverOutcome {
    // Dynamic boundary suppression (Req 7.4)
    if is_in_dynamic_scope {
        let old_summary = legacy_warn_to_summary(legacy_should_warn);
        let new_summary = summarize_identities(Some(Vec::new()));

        let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
            ShadowQueryName::DiagnosticsCheck,
            ShadowQueryInput { symbol: symbol.to_string() },
            old_summary,
            new_summary,
            vec!["suppressed: dynamic boundary scope".to_string()],
            vec![diagnostics_trace(
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFallbackState::Blocked,
                None,
            )],
        );

        return DiagnosticsCutoverOutcome {
            action: DiagnosticAction::Suppress,
            classification: DiagnosticClassification::DynamicOrUnavailable,
            receipt,
        };
    }

    // Semantic path (primary)
    let context = QueryContext::new(file_id, scope_id, Some(byte_offset));
    let candidates = semantic_queries.definitions(symbol, &context);
    let classification = classify_diagnostic_result(&candidates);
    let new_summary = classification_to_summary(&classification, symbol);

    // Legacy path (for receipt)
    let old_summary = legacy_warn_to_summary(legacy_should_warn);

    // Build receipt
    let notes = match classification {
        DiagnosticClassification::Exact => vec![],
        DiagnosticClassification::Ambiguous => {
            vec!["ambiguous: multiple or mixed-confidence candidates".to_string()]
        }
        DiagnosticClassification::DynamicOrUnavailable => {
            vec!["suppressed: dynamic boundary or symbol defined".to_string()]
        }
    };

    let fact_source_traces = diagnostics_fact_source_traces(
        &classification,
        &candidates,
        cutover_fallback_state(&classification, &candidates),
    );
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::DiagnosticsCheck,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        notes,
        fact_source_traces,
    );

    // Map classification to action
    let action = match classification {
        DiagnosticClassification::Exact => DiagnosticAction::Warn,
        DiagnosticClassification::Ambiguous => DiagnosticAction::WeakWarn,
        DiagnosticClassification::DynamicOrUnavailable => DiagnosticAction::Suppress,
    };

    tracing::debug!(
        symbol = %symbol,
        verdict = ?receipt.verdict,
        legacy_warn = legacy_should_warn,
        classification = ?classification,
        action = ?action,
        "diagnostics undefined-symbol cutover"
    );

    DiagnosticsCutoverOutcome { action, classification, receipt }
}

// ── Classification logic ──

/// Classify definition candidates for diagnostics purposes.
///
/// The diagnostics provider needs to decide whether to emit an
/// undefined-symbol warning. The classification reflects the query
/// confidence about the symbol's existence:
///
/// - No candidates at all -> `Exact` -> `Warn` (symbol is undefined)
/// - All dynamic-boundary -> `DynamicOrUnavailable` -> `Suppress`
/// - Only low-confidence candidates -> `Ambiguous` -> `WeakWarn`
/// - Usable candidates found (symbol IS defined) -> `DynamicOrUnavailable`
///   -> `Suppress` (no false-positive undefined-symbol diagnostic)
fn classify_diagnostic_result(
    candidates: &[perl_semantic_facts::DefinitionCandidate],
) -> DiagnosticClassification {
    if candidates.is_empty() {
        // No candidates at all: symbol is undefined -> warn.
        return DiagnosticClassification::Exact;
    }

    // Check if ALL candidates are dynamic-boundary.
    let all_dynamic = candidates.iter().all(|c| c.provenance == Provenance::DynamicBoundary);

    if all_dynamic {
        return DiagnosticClassification::DynamicOrUnavailable;
    }

    // Check for usable (non-dynamic, non-low-confidence) candidates.
    let usable_count = candidates
        .iter()
        .filter(|c| c.provenance != Provenance::DynamicBoundary && c.confidence != Confidence::Low)
        .count();

    if usable_count == 0 {
        // All candidates are low-confidence or dynamic -> ambiguous.
        return DiagnosticClassification::Ambiguous;
    }

    // Usable candidates found: symbol IS defined -> suppress the
    // undefined-symbol diagnostic to avoid false positives.
    DiagnosticClassification::DynamicOrUnavailable
}

// ── Summary helpers ──

/// Convert a legacy "should warn" decision into a [`ShadowResultSummary`].
fn legacy_warn_to_summary(should_warn: bool) -> ShadowResultSummary {
    if should_warn {
        summarize_identities(Some(vec!["warn".to_string()]))
    } else {
        summarize_identities(Some(Vec::new()))
    }
}

/// Convert a diagnostic classification into a [`ShadowResultSummary`].
fn classification_to_summary(
    classification: &DiagnosticClassification,
    symbol: &str,
) -> ShadowResultSummary {
    match classification {
        DiagnosticClassification::Exact => {
            // Symbol is undefined -> would emit warning.
            summarize_identities(Some(vec![format!("warn:{symbol}")]))
        }
        DiagnosticClassification::Ambiguous => {
            // Uncertain -> would emit weak warning.
            summarize_identities(Some(vec![format!("weak_warn:{symbol}")]))
        }
        DiagnosticClassification::DynamicOrUnavailable => {
            // Suppressed -> no diagnostic.
            summarize_identities(Some(Vec::new()))
        }
    }
}

fn diagnostics_fact_source_traces(
    classification: &DiagnosticClassification,
    candidates: &[DefinitionCandidate],
    fallback_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    if let Some(candidate) = select_trace_candidate(candidates, fallback_state) {
        return vec![diagnostics_trace(
            provider_source_for_provenance(candidate.provenance),
            candidate.provenance,
            candidate.confidence,
            fallback_state,
            Some(candidate.anchor_id),
        )];
    }

    let (source, provenance, confidence) = match classification {
        DiagnosticClassification::Exact => {
            (ProviderFactSourceKind::CompilerFact, Provenance::SemanticAnalyzer, Confidence::High)
        }
        DiagnosticClassification::Ambiguous => {
            (ProviderFactSourceKind::SemanticFact, Provenance::NameHeuristic, Confidence::Low)
        }
        DiagnosticClassification::DynamicOrUnavailable => {
            (ProviderFactSourceKind::DynamicBoundary, Provenance::DynamicBoundary, Confidence::High)
        }
    };

    vec![diagnostics_trace(source, provenance, confidence, fallback_state, None)]
}

fn select_trace_candidate(
    candidates: &[DefinitionCandidate],
    fallback_state: ProviderFallbackState,
) -> Option<&DefinitionCandidate> {
    match fallback_state {
        ProviderFallbackState::Blocked => candidates
            .iter()
            .find(|candidate| candidate.provenance == Provenance::DynamicBoundary)
            .or_else(|| candidates.first()),
        ProviderFallbackState::Primary => {
            candidates.iter().find(|candidate| is_primary_cutover_candidate(candidate))
        }
        _ => candidates.first(),
    }
}

fn cutover_fallback_state(
    classification: &DiagnosticClassification,
    candidates: &[DefinitionCandidate],
) -> ProviderFallbackState {
    if candidates.iter().any(|candidate| candidate.provenance == Provenance::DynamicBoundary) {
        return ProviderFallbackState::Blocked;
    }

    match classification {
        DiagnosticClassification::Exact => ProviderFallbackState::Primary,
        DiagnosticClassification::DynamicOrUnavailable
            if candidates.len() == 1 && candidates.iter().any(is_primary_cutover_candidate) =>
        {
            ProviderFallbackState::Primary
        }
        DiagnosticClassification::DynamicOrUnavailable => ProviderFallbackState::Fallback,
        DiagnosticClassification::Ambiguous => ProviderFallbackState::Fallback,
    }
}

fn is_primary_cutover_candidate(candidate: &DefinitionCandidate) -> bool {
    matches!(
        candidate.provenance,
        Provenance::ImportExportInference | Provenance::FrameworkSynthesis
    ) && candidate.confidence == Confidence::High
}

fn diagnostics_trace(
    source: ProviderFactSourceKind,
    provenance: Provenance,
    confidence: Confidence,
    fallback_state: ProviderFallbackState,
    anchor_id: Option<perl_semantic_facts::AnchorId>,
) -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::Diagnostics,
        source,
        provenance,
        confidence,
        ProviderFactFreshness::Fresh,
        fallback_state,
        None,
        anchor_id,
        Some(1),
    )
}

fn provider_source_for_provenance(provenance: Provenance) -> ProviderFactSourceKind {
    match provenance {
        Provenance::ImportExportInference
        | Provenance::FrameworkSynthesis
        | Provenance::PragmaInference => ProviderFactSourceKind::CompilerFact,
        Provenance::DynamicBoundary => ProviderFactSourceKind::DynamicBoundary,
        Provenance::SearchFallback | Provenance::NameHeuristic => ProviderFactSourceKind::Fallback,
        Provenance::ExactAst
        | Provenance::DesugaredAst
        | Provenance::SemanticAnalyzer
        | Provenance::LiteralRequireImport => ProviderFactSourceKind::SemanticFact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason,
        EntityFact, EntityId, EntityKind, FileId, OccurrenceFact, Provenance, RenamePlan,
        SafeDeletePlan, ScopeId, VisibleSymbol,
    };
    use perl_workspace::semantic::queries::{DynamicCallableEvidence, SemanticQueries};
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    // ── Minimal SemanticQueries stub for testing ──

    struct StubSemanticQueries {
        definitions_result: Vec<DefinitionCandidate>,
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
            self.definitions_result.clone()
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

    fn make_candidate(
        name: &str,
        anchor_id: u64,
        entity_id: u64,
        provenance: Provenance,
        confidence: Confidence,
    ) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(entity_id),
            AnchorId(anchor_id),
            name.to_string(),
            name.to_string(),
            None,
            EntityKind::Subroutine,
            provenance,
            confidence,
            DefinitionRank::ExactQualified,
            DefinitionRankReason::ExactQualifiedName,
        )
    }

    fn make_exact_candidate(name: &str, anchor_id: u64, entity_id: u64) -> DefinitionCandidate {
        make_candidate(name, anchor_id, entity_id, Provenance::ExactAst, Confidence::High)
    }

    // ── Shadow mode tests ──

    #[test]
    fn shadow_legacy_warn_no_semantic_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let result = diagnostics_undefined_symbol_shadow(
            true,
            &queries,
            "undefined_sub",
            FileId(1),
            None,
            42,
        );

        assert!(result.legacy_should_warn);
        assert_eq!(result.receipt.query, ShadowQueryName::DiagnosticsCheck);
        assert_eq!(result.receipt.input.symbol, "undefined_sub");
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        // Legacy warns (1 identity), new also warns (1 identity) -> Same.
        assert_eq!(result.receipt.old_result.available, true);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.new_result.match_count, 1);
        Ok(())
    }

    #[test]
    fn shadow_legacy_no_warn_semantic_has_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let candidate = make_exact_candidate("Foo::bar", 10, 20);
        let queries = StubSemanticQueries { definitions_result: vec![candidate] };

        let result =
            diagnostics_undefined_symbol_shadow(false, &queries, "Foo::bar", FileId(1), None, 10);

        assert!(!result.legacy_should_warn);
        assert_eq!(result.receipt.query, ShadowQueryName::DiagnosticsCheck);
        // Legacy does not warn (0 identities), new suppresses (0 identities) -> Same.
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    #[test]
    fn shadow_legacy_warn_semantic_suppresses() -> Result<(), Box<dyn std::error::Error>> {
        // Legacy would warn (false positive), but semantic finds the symbol.
        let candidate = make_candidate(
            "imported_sub",
            10,
            20,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let queries = StubSemanticQueries { definitions_result: vec![candidate] };

        let result =
            diagnostics_undefined_symbol_shadow(true, &queries, "imported_sub", FileId(1), None, 5);

        assert!(result.legacy_should_warn);
        assert_eq!(result.receipt.old_result.match_count, 1); // legacy warns
        assert_eq!(result.receipt.new_result.match_count, 0); // semantic suppresses
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Regression);
        assert_eq!(result.receipt.fact_source_traces.len(), 1);
        let trace = &result.receipt.fact_source_traces[0];
        assert_eq!(trace.surface, ProviderSurface::Diagnostics);
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::ImportExportInference);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn diagnostics_compiler_shadow_exact_warning_trace_is_shadow_only()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let result = diagnostics_undefined_symbol_shadow(
            true,
            &queries,
            "genuinely_missing",
            FileId(1),
            None,
            42,
        );

        assert!(result.legacy_should_warn, "shadow mode must preserve legacy behavior");
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 1);
        let trace = &result.receipt.fact_source_traces[0];
        assert_eq!(trace.surface, ProviderSurface::Diagnostics);
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn diagnostics_compiler_shadow_dynamic_boundary_trace_is_labeled()
    -> Result<(), Box<dyn std::error::Error>> {
        let dynamic =
            make_candidate("maybe_dynamic", 10, 20, Provenance::DynamicBoundary, Confidence::Low);
        let queries = StubSemanticQueries { definitions_result: vec![dynamic] };

        let result = diagnostics_undefined_symbol_shadow(
            true,
            &queries,
            "maybe_dynamic",
            FileId(1),
            None,
            5,
        );

        assert!(result.legacy_should_warn, "shadow mode must preserve legacy behavior");
        assert_eq!(result.receipt.new_result.match_count, 0);
        let trace = &result.receipt.fact_source_traces[0];
        assert_eq!(trace.surface, ProviderSurface::Diagnostics);
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(trace.confidence, Confidence::Low);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn shadow_receipt_uses_diagnostics_check_query_name() -> Result<(), Box<dyn std::error::Error>>
    {
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let result =
            diagnostics_undefined_symbol_shadow(false, &queries, "test", FileId(1), None, 0);

        assert_eq!(result.receipt.query, ShadowQueryName::DiagnosticsCheck);
        assert_eq!(result.receipt.input.symbol, "test");
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    // ── Classification tests ──

    #[test]
    fn classify_empty_candidates_is_exact() -> Result<(), Box<dyn std::error::Error>> {
        let result = classify_diagnostic_result(&[]);
        assert_eq!(result, DiagnosticClassification::Exact);
        Ok(())
    }

    #[test]
    fn classify_all_dynamic_is_dynamic_or_unavailable() -> Result<(), Box<dyn std::error::Error>> {
        let candidates =
            vec![make_candidate("dyn_sym", 1, 1, Provenance::DynamicBoundary, Confidence::Low)];
        let result = classify_diagnostic_result(&candidates);
        assert_eq!(result, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn classify_only_low_confidence_is_ambiguous() -> Result<(), Box<dyn std::error::Error>> {
        let candidates =
            vec![make_candidate("low_sym", 1, 1, Provenance::NameHeuristic, Confidence::Low)];
        let result = classify_diagnostic_result(&candidates);
        assert_eq!(result, DiagnosticClassification::Ambiguous);
        Ok(())
    }

    #[test]
    fn classify_usable_candidates_suppresses() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = vec![make_exact_candidate("Foo::bar", 1, 1)];
        let result = classify_diagnostic_result(&candidates);
        // Symbol IS defined -> suppress (DynamicOrUnavailable maps to Suppress).
        assert_eq!(result, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn classify_mixed_dynamic_and_usable_suppresses() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = vec![
            make_exact_candidate("Foo::bar", 1, 1),
            make_candidate("Foo::bar", 2, 2, Provenance::DynamicBoundary, Confidence::Low),
        ];
        let result = classify_diagnostic_result(&candidates);
        // Has usable candidates -> suppress.
        assert_eq!(result, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn classify_mixed_low_and_dynamic_is_ambiguous() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = vec![
            make_candidate("sym", 1, 1, Provenance::NameHeuristic, Confidence::Low),
            make_candidate("sym", 2, 2, Provenance::DynamicBoundary, Confidence::Low),
        ];
        let result = classify_diagnostic_result(&candidates);
        // Not all dynamic (one is NameHeuristic), but no usable -> ambiguous.
        assert_eq!(result, DiagnosticClassification::Ambiguous);
        Ok(())
    }

    // ── Cutover tests ──

    #[test]
    fn cutover_warn_when_symbol_undefined() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "undefined_sub",
            FileId(1),
            None,
            42,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::Warn);
        assert_eq!(outcome.classification, DiagnosticClassification::Exact);
        assert_eq!(outcome.receipt.query, ShadowQueryName::DiagnosticsCheck);
        Ok(())
    }

    #[test]
    fn cutover_suppress_when_symbol_defined() -> Result<(), Box<dyn std::error::Error>> {
        let candidate = make_exact_candidate("Foo::bar", 10, 20);
        let queries = StubSemanticQueries { definitions_result: vec![candidate] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "Foo::bar",
            FileId(1),
            None,
            10,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(outcome.classification, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn cutover_suppress_in_dynamic_scope() -> Result<(), Box<dyn std::error::Error>> {
        // Even if semantic would warn, dynamic scope suppresses.
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "eval_sym",
            FileId(1),
            None,
            50,
            true, // is_in_dynamic_scope
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(outcome.classification, DiagnosticClassification::DynamicOrUnavailable);
        assert!(outcome.receipt.notes.iter().any(|n| n.contains("dynamic boundary")));
        Ok(())
    }

    #[test]
    fn cutover_suppress_when_all_dynamic_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let dynamic = make_candidate("dyn_sym", 1, 1, Provenance::DynamicBoundary, Confidence::Low);
        let queries = StubSemanticQueries { definitions_result: vec![dynamic] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "dyn_sym",
            FileId(1),
            None,
            10,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(outcome.classification, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn cutover_weak_warn_when_only_low_confidence() -> Result<(), Box<dyn std::error::Error>> {
        let low = make_candidate("maybe_sym", 1, 1, Provenance::NameHeuristic, Confidence::Low);
        let queries = StubSemanticQueries { definitions_result: vec![low] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "maybe_sym",
            FileId(1),
            None,
            10,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::WeakWarn);
        assert_eq!(outcome.classification, DiagnosticClassification::Ambiguous);
        Ok(())
    }

    #[test]
    fn cutover_suppress_imported_symbol_no_false_positive() -> Result<(), Box<dyn std::error::Error>>
    {
        // Imported symbol should be found by semantic queries -> suppress.
        // This validates the scorecard gate: imported-symbol false positives=0.
        let imported = make_candidate(
            "imported_func",
            10,
            20,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let queries = StubSemanticQueries { definitions_result: vec![imported] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true, // legacy would falsely warn
            &queries,
            "imported_func",
            FileId(1),
            None,
            5,
            false,
        );

        // Semantic path finds the import -> suppress (no false positive).
        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(
            outcome.receipt.fact_source_traces[0].fallback_state,
            ProviderFallbackState::Primary
        );
        assert_eq!(
            outcome.receipt.fact_source_traces[0].provenance,
            Provenance::ImportExportInference
        );
        Ok(())
    }

    #[test]
    fn cutover_suppress_generated_symbol_with_primary_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let generated = make_candidate(
            "generated_accessor",
            10,
            20,
            Provenance::FrameworkSynthesis,
            Confidence::High,
        );
        let queries = StubSemanticQueries { definitions_result: vec![generated] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "generated_accessor",
            FileId(1),
            None,
            5,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        let trace = &outcome.receipt.fact_source_traces[0];
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn cutover_dynamic_boundary_no_exact_warning() -> Result<(), Box<dyn std::error::Error>> {
        // Dynamic boundary scope should never produce an exact warning.
        // This validates the scorecard gate: dynamic-boundary exact warnings=0.
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "autoloaded_method",
            FileId(1),
            None,
            100,
            true, // is_in_dynamic_scope
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        // Must NOT be Exact classification in dynamic scope.
        assert_eq!(outcome.classification, DiagnosticClassification::DynamicOrUnavailable);
        assert_eq!(
            outcome.receipt.fact_source_traces[0].fallback_state,
            ProviderFallbackState::Blocked
        );
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_classification() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { definitions_result: vec![] };

        let outcome = diagnostics_undefined_symbol_cutover(
            false,
            &queries,
            "test_sym",
            FileId(1),
            None,
            0,
            false,
        );

        assert_eq!(outcome.receipt.query, ShadowQueryName::DiagnosticsCheck);
        assert_eq!(outcome.receipt.input.symbol, "test_sym");
        // Legacy does not warn (0), new warns (1) -> Improved.
        assert_eq!(outcome.receipt.old_result.match_count, 0);
        assert_eq!(outcome.receipt.new_result.match_count, 1);
        Ok(())
    }

    #[test]
    fn cutover_medium_confidence_suppresses() -> Result<(), Box<dyn std::error::Error>> {
        let medium =
            make_candidate("Foo::bar", 10, 20, Provenance::SemanticAnalyzer, Confidence::Medium);
        let queries = StubSemanticQueries { definitions_result: vec![medium] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "Foo::bar",
            FileId(1),
            None,
            10,
            false,
        );

        // Medium confidence is usable -> symbol is defined -> suppress.
        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(
            outcome.receipt.fact_source_traces[0].fallback_state,
            ProviderFallbackState::Fallback
        );
        Ok(())
    }

    #[test]
    fn cutover_low_confidence_trace_uses_fallback_state() -> Result<(), Box<dyn std::error::Error>>
    {
        let low = make_candidate(
            "maybe_imported",
            1,
            1,
            Provenance::ImportExportInference,
            Confidence::Low,
        );
        let queries = StubSemanticQueries { definitions_result: vec![low] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "maybe_imported",
            FileId(1),
            None,
            10,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::WeakWarn);
        assert_eq!(
            outcome.receipt.fact_source_traces[0].fallback_state,
            ProviderFallbackState::Fallback
        );
        Ok(())
    }

    #[test]
    fn cutover_ambiguous_compiler_fact_trace_uses_fallback_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let first = make_candidate(
            "ambiguous_import",
            1,
            1,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let second = make_candidate(
            "ambiguous_import",
            2,
            2,
            Provenance::FrameworkSynthesis,
            Confidence::High,
        );
        let queries = StubSemanticQueries { definitions_result: vec![first, second] };

        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &queries,
            "ambiguous_import",
            FileId(1),
            None,
            10,
            false,
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(
            outcome.receipt.fact_source_traces[0].fallback_state,
            ProviderFallbackState::Fallback
        );
        Ok(())
    }

    // ── Summary helper tests ──

    #[test]
    fn legacy_warn_to_summary_true() -> Result<(), Box<dyn std::error::Error>> {
        let summary = legacy_warn_to_summary(true);
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["warn"]);
        Ok(())
    }

    #[test]
    fn legacy_warn_to_summary_false() -> Result<(), Box<dyn std::error::Error>> {
        let summary = legacy_warn_to_summary(false);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        assert!(summary.identities.is_empty());
        Ok(())
    }

    #[test]
    fn classification_to_summary_exact() -> Result<(), Box<dyn std::error::Error>> {
        let summary = classification_to_summary(&DiagnosticClassification::Exact, "test_sym");
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["warn:test_sym"]);
        Ok(())
    }

    #[test]
    fn classification_to_summary_ambiguous() -> Result<(), Box<dyn std::error::Error>> {
        let summary = classification_to_summary(&DiagnosticClassification::Ambiguous, "test_sym");
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["weak_warn:test_sym"]);
        Ok(())
    }

    #[test]
    fn classification_to_summary_dynamic() -> Result<(), Box<dyn std::error::Error>> {
        let summary =
            classification_to_summary(&DiagnosticClassification::DynamicOrUnavailable, "test_sym");
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        assert!(summary.identities.is_empty());
        Ok(())
    }
}
