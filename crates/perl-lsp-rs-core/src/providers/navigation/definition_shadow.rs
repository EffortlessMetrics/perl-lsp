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

use perl_semantic_facts::{Confidence, DefinitionCandidate, Provenance};
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
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::FindDefinition,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
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
        .into_iter()
        .filter(|c| c.provenance != Provenance::DynamicBoundary && c.confidence != Confidence::Low)
        .collect();

    // ── Legacy path (for fallback and receipt) ──
    let legacy_location = workspace_index.find_definition(symbol);
    let old_summary = legacy_location_to_summary(legacy_location.as_ref());

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::FindDefinition,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
    );

    // ── Classify result ──
    let result = classify_cutover_result(usable, legacy_location);

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
    use perl_workspace::semantic::queries::SemanticQueries;
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
        assert_eq!(result.receipt.schema_version, 1);
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
