//! Goto-definition shadow compare and cutover paths.
//!
//! Provides two entry points for goto-definition:
//!
//! 1. **Shadow mode** ([`goto_definition_shadow`]) — runs both legacy and
//!    semantic paths side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`goto_definition_cutover`]) — uses the semantic
//!    path as the primary source of truth with fallback to legacy:
//!    - *Exact*: single high-confidence candidate → jump to definition.
//!    - *Ambiguous*: multiple candidates → show candidate list.
//!    - *Dynamic / Unavailable*: no usable candidates → fall back to legacy.
//!
//! # Requirements
//!
//! - **Req 9.1**: Goto-definition calls `SemanticQueries::definitions`.
//! - **Req 10.1**: Maintain existing query path as fallback during validation.
//! - **Req 10.2**: Shadow-compare runs both old and new paths, producing
//!   deterministic receipts.
//! - **Req 10.6**: Scorecard gate: regressions=0, ambiguous classified,
//!   unavailable falls back.
//! - **Req 22.1**: Goto-definition shadow mode emits receipts before cutover.
//! - **Req 22.3**: Exact → jump; Ambiguous → show candidates;
//!   Dynamic/Unavailable → fall back to legacy.

use perl_semantic_facts::{
    Confidence, DefinitionCandidate, EntityKind, Provenance, ProviderFactFreshness,
    ProviderFactSourceKind, ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};
use perl_workspace::workspace_index::{Location, WorkspaceIndex};

/// Result of a shadow-compared goto-definition request.
///
/// Contains the legacy result (which callers should use during the shadow
/// phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct DefinitionShadowResult {
    /// Legacy result — the locations returned by `WorkspaceIndex::find_definition`.
    /// Callers should use this during the shadow phase.
    pub legacy_result: Option<Location>,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run goto-definition through both legacy and semantic paths, producing a
/// shadow-compare receipt.
///
/// # Arguments
///
/// * `workspace_index` — the legacy workspace index for `find_definition`.
/// * `semantic_queries` — the new semantic query facade.
/// * `symbol` — the symbol name to look up (qualified or bare).
/// * `context` — query context for the semantic path (file, scope, offset).
///
/// # Returns
///
/// A [`DefinitionShadowResult`] containing the legacy result and a receipt.
/// The caller should return the legacy result to the LSP client during the
/// shadow phase.
pub fn goto_definition_shadow<Q: SemanticQueries>(
    workspace_index: &WorkspaceIndex,
    semantic_queries: &Q,
    symbol: &str,
    context: &QueryContext,
) -> DefinitionShadowResult {
    // ── Legacy path ──
    let legacy_location = workspace_index.find_definition(symbol);
    let old_summary = legacy_location_to_summary(legacy_location.as_ref());

    // ── New semantic path ──
    let new_candidates = semantic_queries.definitions(symbol, context);
    let new_summary = semantic_candidates_to_summary(&new_candidates);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::FindDefinition,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        vec![definition_shadow_quality_note(legacy_location.as_ref(), &new_candidates)],
        definition_fact_source_traces(&new_candidates, ProviderFallbackState::Shadow),
    );

    tracing::debug!(
        symbol = %symbol,
        verdict = ?receipt.verdict,
        old_count = receipt.old_result.match_count,
        new_count = receipt.new_result.match_count,
        "goto-definition shadow compare"
    );

    DefinitionShadowResult { legacy_result: legacy_location, receipt }
}

// ── Cutover types ──

/// Classification of the semantic definition result for cutover decisions.
///
/// Follows the fallback policy table (Req 22.3):
/// - Exact → jump to definition
/// - Ambiguous → show candidate list
/// - LegacyFallback → semantic path unavailable or dynamic; use legacy result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionCutoverResult {
    /// Exactly one high-confidence candidate — jump directly to it.
    Exact(DefinitionCandidate),
    /// Multiple candidates — present the list to the user.
    Ambiguous(Vec<DefinitionCandidate>),
    /// Semantic path produced no usable result — fall back to legacy.
    LegacyFallback(Option<Location>),
}

/// Outcome of a cutover goto-definition request.
///
/// Contains the classified result and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct DefinitionCutoverOutcome {
    /// The classified cutover result.
    pub result: DefinitionCutoverResult,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run goto-definition with the semantic path as primary, falling back to
/// legacy when the semantic result is unavailable or dynamic.
///
/// # Decision logic
///
/// 1. Call `SemanticQueries::definitions` for the symbol.
/// 2. Filter out candidates that are purely dynamic-boundary
///    (`Provenance::DynamicBoundary`) or have `Confidence::Low`.
/// 3. Classify the filtered result:
///    - **Exact**: exactly one candidate → `DefinitionCutoverResult::Exact`.
///    - **Ambiguous**: two or more candidates → `DefinitionCutoverResult::Ambiguous`.
///    - **Unavailable**: zero usable candidates → fall back to legacy
///      `WorkspaceIndex::find_definition`.
/// 4. Emit a shadow-compare receipt regardless of outcome.
///
/// # Arguments
///
/// * `workspace_index` — legacy workspace index for fallback.
/// * `semantic_queries` — the semantic query facade (primary path).
/// * `symbol` — the symbol name to look up (qualified or bare).
/// * `context` — query context for the semantic path.
///
/// # Returns
///
/// A [`DefinitionCutoverOutcome`] with the classified result and receipt.
pub fn goto_definition_cutover<Q: SemanticQueries>(
    workspace_index: &WorkspaceIndex,
    semantic_queries: &Q,
    symbol: &str,
    context: &QueryContext,
) -> DefinitionCutoverOutcome {
    // ── Semantic path (primary) ──
    let all_candidates = semantic_queries.definitions(symbol, context);
    let new_summary = semantic_candidates_to_summary(&all_candidates);

    // Filter to usable candidates: exclude dynamic-boundary provenance and
    // low-confidence results that cannot drive a reliable jump.
    let usable: Vec<DefinitionCandidate> = all_candidates
        .iter()
        .filter(|c| c.provenance != Provenance::DynamicBoundary && c.confidence != Confidence::Low)
        .cloned()
        .collect();

    // ── Legacy path (for fallback and receipt) ──
    let legacy_location = workspace_index.find_definition(symbol);
    let old_summary = legacy_location_to_summary(legacy_location.as_ref());

    // ── Classify result ──
    let result = classify_cutover_result(usable, legacy_location);
    let fallback_state = definition_cutover_fallback_state(&result);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::FindDefinition,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
        definition_fact_source_traces(&all_candidates, fallback_state),
    );

    tracing::debug!(
        symbol = %symbol,
        verdict = ?receipt.verdict,
        classification = match &result {
            DefinitionCutoverResult::Exact(_) => "exact",
            DefinitionCutoverResult::Ambiguous(_) => "ambiguous",
            DefinitionCutoverResult::LegacyFallback(_) => "legacy_fallback",
        },
        "goto-definition cutover"
    );

    DefinitionCutoverOutcome { result, receipt }
}

/// Classify filtered candidates into the cutover result category.
fn classify_cutover_result(
    usable: Vec<DefinitionCandidate>,
    legacy_location: Option<Location>,
) -> DefinitionCutoverResult {
    match usable.len() {
        0 => DefinitionCutoverResult::LegacyFallback(legacy_location),
        1 => {
            // Safety: len() == 1 guarantees into_iter().next() is Some.
            let candidate = usable.into_iter().next();
            match candidate {
                Some(c) => DefinitionCutoverResult::Exact(c),
                // Unreachable given len() == 1, but handle gracefully.
                None => DefinitionCutoverResult::LegacyFallback(legacy_location),
            }
        }
        _ => DefinitionCutoverResult::Ambiguous(usable),
    }
}

fn definition_cutover_fallback_state(result: &DefinitionCutoverResult) -> ProviderFallbackState {
    match result {
        DefinitionCutoverResult::Exact(_) | DefinitionCutoverResult::Ambiguous(_) => {
            ProviderFallbackState::Primary
        }
        DefinitionCutoverResult::LegacyFallback(_) => ProviderFallbackState::Fallback,
    }
}

fn definition_fact_source_traces(
    candidates: &[DefinitionCandidate],
    fallback_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    let mut traces: Vec<ProviderFactTrace> = candidates
        .iter()
        .map(|candidate| {
            let (source, provenance, state) = definition_trace_shape(candidate, fallback_state);
            ProviderFactTrace::new(
                ProviderSurface::Definition,
                source,
                provenance,
                candidate.confidence,
                ProviderFactFreshness::Fresh,
                state,
                None,
                Some(candidate.anchor_id),
                Some(1),
            )
        })
        .collect();

    if traces.is_empty() {
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Definition,
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::NotApplicable,
            ProviderFallbackState::Fallback,
            None,
            None,
            Some(1),
        ));
    }

    traces
}

fn definition_shadow_quality_note(
    legacy_location: Option<&Location>,
    candidates: &[DefinitionCandidate],
) -> String {
    let legacy_count = usize::from(legacy_location.is_some());
    let answer_count = definition_answer_candidate_count(candidates);
    let generated_label_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.kind == EntityKind::GeneratedMember
                || candidate.provenance == Provenance::FrameworkSynthesis
        })
        .count();
    let dynamic_boundary_blockers = candidates
        .iter()
        .filter(|candidate| candidate.provenance == Provenance::DynamicBoundary)
        .count();
    let noise_delta = candidates
        .iter()
        .filter(|candidate| {
            candidate.confidence == Confidence::Low
                || matches!(
                    candidate.provenance,
                    Provenance::NameHeuristic | Provenance::SearchFallback
                )
        })
        .count();

    format!(
        "definition shadow proof: legacy_candidates={legacy_count}; compiler_fact_candidates={}; answer_candidates={answer_count}; rank_delta={}; noise_delta={noise_delta}; generated_labels={generated_label_count}; dynamic_boundary_blockers={dynamic_boundary_blockers}; stale_fact_blockers=0; blocked_candidates={dynamic_boundary_blockers}; no live navigation behavior change",
        candidates.len(),
        signed_count_delta(legacy_count, answer_count)
    )
}

fn definition_answer_candidate_count(candidates: &[DefinitionCandidate]) -> usize {
    candidates
        .iter()
        .filter(|candidate| {
            let (source, _, state) =
                definition_trace_shape(candidate, ProviderFallbackState::Shadow);
            state != ProviderFallbackState::Blocked
                && source != ProviderFactSourceKind::Fallback
                && candidate.confidence != Confidence::Low
        })
        .count()
}

fn signed_count_delta(old_count: usize, new_count: usize) -> String {
    if new_count >= old_count {
        format!("+{}", new_count - old_count)
    } else {
        format!("-{}", old_count - new_count)
    }
}

fn definition_trace_shape(
    candidate: &DefinitionCandidate,
    fallback_state: ProviderFallbackState,
) -> (ProviderFactSourceKind, Provenance, ProviderFallbackState) {
    if candidate.provenance == Provenance::DynamicBoundary {
        return (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            ProviderFallbackState::Blocked,
        );
    }

    match candidate.provenance {
        Provenance::FrameworkSynthesis => (
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            fallback_state,
        ),
        Provenance::ImportExportInference | Provenance::PragmaInference => {
            (ProviderFactSourceKind::CompilerFact, candidate.provenance, fallback_state)
        }
        Provenance::NameHeuristic | Provenance::SearchFallback => (
            ProviderFactSourceKind::Fallback,
            candidate.provenance,
            ProviderFallbackState::Fallback,
        ),
        Provenance::ExactAst | Provenance::DesugaredAst | Provenance::SemanticAnalyzer
            if candidate.kind == EntityKind::GeneratedMember =>
        {
            (
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                fallback_state,
            )
        }
        Provenance::ExactAst | Provenance::DesugaredAst | Provenance::SemanticAnalyzer => {
            (ProviderFactSourceKind::SemanticFact, candidate.provenance, fallback_state)
        }
        Provenance::DynamicBoundary => (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            ProviderFallbackState::Blocked,
        ),
    }
}

/// Convert a legacy `Location` (if any) into a [`ShadowResultSummary`].
fn legacy_location_to_summary(location: Option<&Location>) -> ShadowResultSummary {
    match location {
        Some(loc) => {
            let identity =
                format!("{}:{}:{}", loc.uri, loc.range.start.line, loc.range.start.column,);
            summarize_identities(Some(vec![identity]))
        }
        None => summarize_identities(None),
    }
}

/// Convert semantic `DefinitionCandidate` results into a [`ShadowResultSummary`].
fn semantic_candidates_to_summary(
    candidates: &[perl_semantic_facts::DefinitionCandidate],
) -> ShadowResultSummary {
    if candidates.is_empty() {
        return summarize_identities(Some(Vec::new()));
    }

    let identities: Vec<String> = candidates
        .iter()
        .map(|c| {
            // Use canonical_name + anchor_id as a stable identity since we
            // don't have the resolved URI/line from the semantic path yet.
            format!("{}:anchor:{}", c.canonical_name, c.anchor_id.0)
        })
        .collect();

    summarize_identities(Some(identities))
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
    use url::Url;

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

    fn make_candidate(name: &str, anchor_id: u64, entity_id: u64) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(entity_id),
            AnchorId(anchor_id),
            name.to_string(),
            name.to_string(),
            None,
            EntityKind::Subroutine,
            Provenance::ExactAst,
            Confidence::High,
            DefinitionRank::ExactQualified,
            DefinitionRankReason::ExactQualifiedName,
        )
    }

    #[test]
    fn shadow_both_unavailable_yields_unavailable() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { definitions_result: vec![] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "No::Such::Symbol", &ctx);

        // Legacy returns None -> unavailable; new returns empty -> available but 0 matches.
        // The receipt should reflect the old path as unavailable.
        assert!(result.legacy_result.is_none());
        assert_eq!(result.receipt.query, ShadowQueryName::FindDefinition);
        assert_eq!(result.receipt.old_result.available, false);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    #[test]
    fn shadow_new_path_has_candidates_old_unavailable() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries =
            StubSemanticQueries { definitions_result: vec![make_candidate("Foo::bar", 10, 20)] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "Foo::bar", &ctx);

        // Legacy unavailable, new has 1 candidate -> Unavailable verdict
        // (because old path is unavailable).
        assert!(result.legacy_result.is_none());
        assert_eq!(result.receipt.old_result.available, false);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    #[test]
    fn legacy_location_to_summary_some() -> Result<(), Box<dyn std::error::Error>> {
        use perl_parser_core::position::{Position, Range};

        let loc = Location {
            uri: "file:///test.pm".to_string(),
            range: Range { start: Position::new(0, 5, 3), end: Position::new(0, 5, 10) },
        };
        let summary = super::legacy_location_to_summary(Some(&loc));
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        assert_eq!(summary.identities, vec!["file:///test.pm:5:3"]);
        Ok(())
    }

    #[test]
    fn legacy_location_to_summary_none() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_location_to_summary(None);
        assert!(!summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_candidates_to_summary_empty() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::semantic_candidates_to_summary(&[]);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_candidates_to_summary_multiple() -> Result<(), Box<dyn std::error::Error>> {
        let candidates =
            vec![make_candidate("Foo::bar", 10, 20), make_candidate("Baz::bar", 30, 40)];
        let summary = super::semantic_candidates_to_summary(&candidates);
        assert!(summary.available);
        assert_eq!(summary.match_count, 2);
        // Identities should be sorted and deduplicated.
        assert_eq!(summary.identities.len(), 2);
        Ok(())
    }

    #[test]
    fn receipt_uses_find_definition_query_name() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { definitions_result: vec![] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "test", &ctx);
        assert_eq!(result.receipt.query, ShadowQueryName::FindDefinition);
        assert_eq!(result.receipt.input.symbol, "test");
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    // ── Cutover tests ──

    fn make_candidate_with_confidence(
        name: &str,
        anchor_id: u64,
        entity_id: u64,
        confidence: Confidence,
        provenance: Provenance,
        rank: DefinitionRank,
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
            rank,
            DefinitionRankReason::ExactQualifiedName,
        )
    }

    fn make_candidate_with_trace_fields(
        name: &str,
        anchor_id: u64,
        entity_id: u64,
        kind: EntityKind,
        confidence: Confidence,
        provenance: Provenance,
        rank: DefinitionRank,
        rank_reason: DefinitionRankReason,
    ) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(entity_id),
            AnchorId(anchor_id),
            name.to_string(),
            name.to_string(),
            None,
            kind,
            provenance,
            confidence,
            rank,
            rank_reason,
        )
    }

    fn first_trace<'a>(
        receipt: &'a SemanticShadowCompareReceipt,
    ) -> Result<&'a ProviderFactTrace, Box<dyn std::error::Error>> {
        match receipt.fact_source_traces.first() {
            Some(trace) => Ok(trace),
            None => Err("missing fact-source trace".into()),
        }
    }

    fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
        Ok(Url::parse(&format!("file://{path}"))?)
    }

    fn build_real_workspace_definition_index() -> Result<WorkspaceIndex, Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();
        index.index_file(
            file_url("/lib/Real/Nav.pm")?,
            "package Real::Nav;\nsub legacy_helper { 1 }\n1;\n".to_string(),
        )?;
        index.index_file(
            file_url("/script/app.pl")?,
            "use Real::Nav;\nReal::Nav::legacy_helper();\n".to_string(),
        )?;
        Ok(index)
    }

    #[test]
    fn cutover_exact_single_high_confidence_candidate() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let candidate = make_candidate("Foo::bar", 10, 20);
        let queries = StubSemanticQueries { definitions_result: vec![candidate.clone()] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        assert_eq!(outcome.result, DefinitionCutoverResult::Exact(candidate));
        assert_eq!(outcome.receipt.query, ShadowQueryName::FindDefinition);
        Ok(())
    }

    #[test]
    fn cutover_ambiguous_multiple_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let c1 = make_candidate("Foo::bar", 10, 20);
        let c2 = make_candidate("Baz::bar", 30, 40);
        let queries = StubSemanticQueries { definitions_result: vec![c1.clone(), c2.clone()] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "bar", &ctx);

        match &outcome.result {
            DefinitionCutoverResult::Ambiguous(candidates) => {
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0], c1);
                assert_eq!(candidates[1], c2);
            }
            other => return Err(format!("expected Ambiguous, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_no_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { definitions_result: vec![] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "No::Such::Symbol", &ctx);

        match &outcome.result {
            DefinitionCutoverResult::LegacyFallback(loc) => {
                // Legacy also finds nothing for an empty index.
                assert!(loc.is_none());
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_all_dynamic_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let dynamic_candidate = make_candidate_with_confidence(
            "Foo::bar",
            10,
            20,
            Confidence::Low,
            Provenance::DynamicBoundary,
            DefinitionRank::Heuristic,
        );
        let queries = StubSemanticQueries { definitions_result: vec![dynamic_candidate] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        match &outcome.result {
            DefinitionCutoverResult::LegacyFallback(_) => {}
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_all_low_confidence() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let low_candidate = make_candidate_with_confidence(
            "Foo::bar",
            10,
            20,
            Confidence::Low,
            Provenance::NameHeuristic,
            DefinitionRank::Heuristic,
        );
        let queries = StubSemanticQueries { definitions_result: vec![low_candidate] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        match &outcome.result {
            DefinitionCutoverResult::LegacyFallback(_) => {}
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_filters_dynamic_keeps_exact() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let good = make_candidate("Foo::bar", 10, 20);
        let dynamic = make_candidate_with_confidence(
            "Foo::bar",
            30,
            40,
            Confidence::Low,
            Provenance::DynamicBoundary,
            DefinitionRank::Heuristic,
        );
        let queries = StubSemanticQueries { definitions_result: vec![good.clone(), dynamic] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        // Dynamic candidate filtered out, leaving exactly one usable → Exact.
        assert_eq!(outcome.result, DefinitionCutoverResult::Exact(good));
        assert!(outcome.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::SemanticFact
                && trace.provenance == Provenance::ExactAst
                && trace.fallback_state == ProviderFallbackState::Primary
        }));
        assert!(outcome.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::DynamicBoundary
                && trace.provenance == Provenance::DynamicBoundary
                && trace.fallback_state == ProviderFallbackState::Blocked
        }));
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_all_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let c1 = make_candidate("Foo::bar", 10, 20);
        let c2 = make_candidate_with_confidence(
            "Foo::bar",
            30,
            40,
            Confidence::Low,
            Provenance::DynamicBoundary,
            DefinitionRank::Heuristic,
        );
        let queries = StubSemanticQueries { definitions_result: vec![c1, c2] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        // Receipt should reflect ALL candidates (before filtering), not just usable ones.
        assert_eq!(outcome.receipt.new_result.match_count, 2);
        assert_eq!(outcome.receipt.query, ShadowQueryName::FindDefinition);
        Ok(())
    }

    #[test]
    fn definition_compiler_shadow_traces_import_export_candidate()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let candidate = make_candidate_with_trace_fields(
            "Foo::imported_func",
            10,
            20,
            EntityKind::Subroutine,
            Confidence::High,
            Provenance::ImportExportInference,
            DefinitionRank::ExplicitImport,
            DefinitionRankReason::ExplicitImport { module: "Foo".to_string() },
        );
        let queries = StubSemanticQueries { definitions_result: vec![candidate] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "imported_func", &ctx);
        let trace = first_trace(&result.receipt)?;

        assert!(result.legacy_result.is_none());
        assert_eq!(trace.surface, ProviderSurface::Definition);
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::ImportExportInference);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn definition_compiler_shadow_traces_framework_generated_candidate()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let candidate = make_candidate_with_trace_fields(
            "Foo::generated_accessor",
            11,
            21,
            EntityKind::GeneratedMember,
            Confidence::Medium,
            Provenance::FrameworkSynthesis,
            DefinitionRank::WorkspaceCandidate,
            DefinitionRankReason::WorkspaceSymbol,
        );
        let queries = StubSemanticQueries { definitions_result: vec![candidate] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "generated_accessor", &ctx);
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::Definition);
        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn definition_compiler_shadow_traces_dynamic_boundary_as_blocked()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let candidate = make_candidate_with_trace_fields(
            "Foo::dynamic_symbol",
            12,
            22,
            EntityKind::Unknown,
            Confidence::High,
            Provenance::DynamicBoundary,
            DefinitionRank::Heuristic,
            DefinitionRankReason::HeuristicNameMatch,
        );
        let queries = StubSemanticQueries { definitions_result: vec![candidate] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "dynamic_symbol", &ctx);
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::Definition);
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        assert!(result.legacy_result.is_none());
        Ok(())
    }

    #[test]
    fn definition_compiler_shadow_low_confidence_does_not_outrank_exact()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let exact = make_candidate("Foo::bar", 10, 20);
        let low = make_candidate_with_trace_fields(
            "Foo::bar",
            30,
            40,
            EntityKind::Subroutine,
            Confidence::Low,
            Provenance::NameHeuristic,
            DefinitionRank::Heuristic,
            DefinitionRankReason::HeuristicNameMatch,
        );
        let queries = StubSemanticQueries { definitions_result: vec![exact.clone(), low] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        assert_eq!(outcome.result, DefinitionCutoverResult::Exact(exact));
        assert!(outcome.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::SemanticFact
                && trace.provenance == Provenance::ExactAst
                && trace.fallback_state == ProviderFallbackState::Primary
        }));
        assert!(outcome.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::Fallback
                && trace.provenance == Provenance::NameHeuristic
                && trace.confidence == Confidence::Low
                && trace.fallback_state == ProviderFallbackState::Fallback
        }));
        Ok(())
    }

    #[test]
    fn definition_shadow_records_real_workspace_quality_receipt()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = build_real_workspace_definition_index()?;
        let imported = make_candidate_with_trace_fields(
            "Real::Nav::imported_func",
            10,
            20,
            EntityKind::Subroutine,
            Confidence::High,
            Provenance::ImportExportInference,
            DefinitionRank::ExplicitImport,
            DefinitionRankReason::ExplicitImport { module: "Real::Nav".to_string() },
        );
        let generated = make_candidate_with_trace_fields(
            "Real::Nav::generated_accessor",
            11,
            21,
            EntityKind::GeneratedMember,
            Confidence::Medium,
            Provenance::FrameworkSynthesis,
            DefinitionRank::WorkspaceCandidate,
            DefinitionRankReason::WorkspaceSymbol,
        );
        let dynamic = make_candidate_with_trace_fields(
            "Real::Nav::dynamic_symbol",
            12,
            22,
            EntityKind::Unknown,
            Confidence::High,
            Provenance::DynamicBoundary,
            DefinitionRank::Heuristic,
            DefinitionRankReason::HeuristicNameMatch,
        );
        let low_confidence = make_candidate_with_trace_fields(
            "Real::Nav::legacy_helper",
            13,
            23,
            EntityKind::Subroutine,
            Confidence::Low,
            Provenance::NameHeuristic,
            DefinitionRank::Heuristic,
            DefinitionRankReason::HeuristicNameMatch,
        );
        let queries = StubSemanticQueries {
            definitions_result: vec![imported, generated, dynamic, low_confidence],
        };
        let ctx = QueryContext::new(FileId(1), None, None);

        let result = goto_definition_shadow(&index, &queries, "Real::Nav::legacy_helper", &ctx);

        assert!(result.legacy_result.is_some(), "legacy workspace definition should resolve");
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 4);
        let note = result.receipt.notes.join(" ");
        assert!(note.contains("legacy_candidates=1"));
        assert!(note.contains("compiler_fact_candidates=4"));
        assert!(note.contains("answer_candidates=2"));
        assert!(note.contains("rank_delta=+1"));
        assert!(note.contains("noise_delta=1"));
        assert!(note.contains("generated_labels=1"));
        assert!(note.contains("dynamic_boundary_blockers=1"));
        assert!(note.contains("stale_fact_blockers=0"));
        assert!(note.contains("blocked_candidates=1"));
        assert!(note.contains("no live navigation behavior change"));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::FrameworkAdapter
                && trace.provenance == Provenance::FrameworkSynthesis
                && trace.fallback_state == ProviderFallbackState::Shadow
        }));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::DynamicBoundary
                && trace.provenance == Provenance::DynamicBoundary
                && trace.fallback_state == ProviderFallbackState::Blocked
        }));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::Fallback
                && trace.confidence == Confidence::Low
                && trace.fallback_state == ProviderFallbackState::Fallback
        }));
        Ok(())
    }

    #[test]
    fn cutover_medium_confidence_is_usable() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let medium = make_candidate_with_confidence(
            "Foo::bar",
            10,
            20,
            Confidence::Medium,
            Provenance::SemanticAnalyzer,
            DefinitionRank::WorkspaceCandidate,
        );
        let queries = StubSemanticQueries { definitions_result: vec![medium.clone()] };
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &queries, "Foo::bar", &ctx);

        // Medium confidence is usable — should produce Exact, not fallback.
        assert_eq!(outcome.result, DefinitionCutoverResult::Exact(medium));
        Ok(())
    }

    #[test]
    fn classify_cutover_result_empty_is_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let result = super::classify_cutover_result(vec![], None);
        assert_eq!(result, DefinitionCutoverResult::LegacyFallback(None));
        Ok(())
    }

    #[test]
    fn classify_cutover_result_single_is_exact() -> Result<(), Box<dyn std::error::Error>> {
        let c = make_candidate("test", 1, 1);
        let result = super::classify_cutover_result(vec![c.clone()], None);
        assert_eq!(result, DefinitionCutoverResult::Exact(c));
        Ok(())
    }

    #[test]
    fn classify_cutover_result_multiple_is_ambiguous() -> Result<(), Box<dyn std::error::Error>> {
        let c1 = make_candidate("a", 1, 1);
        let c2 = make_candidate("b", 2, 2);
        let result = super::classify_cutover_result(vec![c1.clone(), c2.clone()], None);
        match result {
            DefinitionCutoverResult::Ambiguous(candidates) => {
                assert_eq!(candidates.len(), 2);
            }
            other => return Err(format!("expected Ambiguous, got {:?}", other).into()),
        }
        Ok(())
    }
}
