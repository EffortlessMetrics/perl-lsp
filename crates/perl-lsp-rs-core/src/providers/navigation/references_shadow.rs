//! Find-references shadow compare and cutover paths.
//!
//! Provides two entry points for find-references:
//!
//! 1. **Shadow mode** ([`find_references_shadow`]) — runs both legacy and
//!    semantic paths side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`find_references_cutover`]) — uses the semantic
//!    path as the primary source of truth with fallback to legacy:
//!    - *Exact*: typed occurrence references → return them.
//!    - *Ambiguous*: multiple grouped candidates → include grouped candidates.
//!    - *Dynamic / Unavailable*: no usable results → fall back to legacy.
//!
//! # Requirements
//!
//! - **Req 9.2**: Find-references calls `SemanticQueries::references`.
//! - **Req 10.1**: Maintain existing query path as fallback during validation.
//! - **Req 10.2**: Shadow-compare runs both old and new paths, producing
//!   deterministic receipts.
//! - **Req 10.7**: Scorecard gate: legacy count parity or better, definition
//!   exclusion correct.
//! - **Req 22.1**: Find-references shadow mode emits receipts before cutover.
//! - **Req 22.4**: Exact → return typed refs; Ambiguous → include grouped
//!   candidates; Dynamic/Unavailable → fall back to legacy.

use perl_semantic_facts::{
    Confidence, EntityId, OccurrenceFact, OccurrenceKind, Provenance, ProviderFactFreshness,
    ProviderFactSourceKind, ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};
use perl_workspace::workspace_index::{Location, WorkspaceIndex};

/// Result of a shadow-compared find-references request.
///
/// Contains the legacy result (which callers should use during the shadow
/// phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct ReferencesShadowResult {
    /// Legacy result — the locations returned by `WorkspaceIndex::find_references`.
    /// Callers should use this during the shadow phase.
    pub legacy_result: Vec<Location>,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run find-references through both legacy and semantic paths, producing a
/// shadow-compare receipt.
///
/// # Arguments
///
/// * `workspace_index` — the legacy workspace index for `find_references`.
/// * `semantic_queries` — the new semantic query facade.
/// * `symbol` — the symbol name to look up (used for legacy path and receipt input).
/// * `entity_id` — the entity ID to look up (used for semantic path).
///
/// # Returns
///
/// A [`ReferencesShadowResult`] containing the legacy result and a receipt.
/// The caller should return the legacy result to the LSP client during the
/// shadow phase.
pub fn find_references_shadow<Q: SemanticQueries>(
    workspace_index: &WorkspaceIndex,
    semantic_queries: &Q,
    symbol: &str,
    entity_id: EntityId,
) -> ReferencesShadowResult {
    // ── Legacy path ──
    let legacy_locations = workspace_index.find_references(symbol);
    let old_summary = legacy_locations_to_summary(&legacy_locations);

    // ── New semantic path ──
    let new_occurrences = semantic_queries.references(entity_id);
    let new_summary = semantic_occurrences_to_summary(&new_occurrences);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::FindReferences,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        vec![references_shadow_quality_note(&legacy_locations, &new_occurrences)],
        references_fact_source_traces(&new_occurrences, ProviderFallbackState::Shadow),
    );

    tracing::debug!(
        symbol = %symbol,
        entity_id = ?entity_id,
        verdict = ?receipt.verdict,
        old_count = receipt.old_result.match_count,
        new_count = receipt.new_result.match_count,
        "find-references shadow compare"
    );

    ReferencesShadowResult { legacy_result: legacy_locations, receipt }
}

// ── Cutover types ──

/// Classification of the semantic references result for cutover decisions.
///
/// Follows the fallback policy table (Req 22.4):
/// - Exact → return typed references
/// - Ambiguous → include grouped candidates
/// - LegacyFallback → semantic path unavailable or dynamic; use legacy result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferencesCutoverResult {
    /// Typed occurrence references from the semantic path.
    Exact(Vec<OccurrenceFact>),
    /// Multiple grouped candidates — present grouped results to the user.
    Ambiguous(Vec<OccurrenceFact>),
    /// Semantic path produced no usable result — fall back to legacy.
    LegacyFallback(Vec<Location>),
}

/// Outcome of a cutover find-references request.
///
/// Contains the classified result and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct ReferencesCutoverOutcome {
    /// The classified cutover result.
    pub result: ReferencesCutoverResult,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run find-references with the semantic path as primary, falling back to
/// legacy when the semantic result is unavailable or dynamic.
///
/// # Decision logic
///
/// 1. Call `SemanticQueries::references` for the entity.
/// 2. Filter out occurrences that are purely dynamic-boundary
///    (`Provenance::DynamicBoundary`) or have `Confidence::Low`.
/// 3. Classify the filtered result:
///    - **Exact**: all usable occurrences have high/medium confidence →
///      `ReferencesCutoverResult::Exact`.
///    - **Ambiguous**: some occurrences have mixed provenance →
///      `ReferencesCutoverResult::Ambiguous`.
///    - **Unavailable**: zero usable occurrences → fall back to legacy
///      `WorkspaceIndex::find_references`.
/// 4. Emit a shadow-compare receipt regardless of outcome.
///
/// # Arguments
///
/// * `workspace_index` — legacy workspace index for fallback.
/// * `semantic_queries` — the semantic query facade (primary path).
/// * `symbol` — the symbol name for legacy path and receipt input.
/// * `entity_id` — the entity ID for the semantic path.
///
/// # Returns
///
/// A [`ReferencesCutoverOutcome`] with the classified result and receipt.
pub fn find_references_cutover<Q: SemanticQueries>(
    workspace_index: &WorkspaceIndex,
    semantic_queries: &Q,
    symbol: &str,
    entity_id: EntityId,
) -> ReferencesCutoverOutcome {
    // ── Semantic path (primary) ──
    let all_occurrences = semantic_queries.references(entity_id);
    let new_summary = semantic_occurrences_to_summary(&all_occurrences);

    // Filter to usable occurrences: exclude dynamic-boundary provenance and
    // low-confidence results.
    let usable: Vec<OccurrenceFact> = all_occurrences
        .iter()
        .filter(|o| o.provenance != Provenance::DynamicBoundary && o.confidence != Confidence::Low)
        .cloned()
        .collect();

    // ── Legacy path (for fallback and receipt) ──
    let legacy_locations = workspace_index.find_references(symbol);
    let old_summary = legacy_locations_to_summary(&legacy_locations);

    // ── Classify result ──
    let result = classify_cutover_result(usable, legacy_locations);
    let fallback_state = references_cutover_fallback_state(&result);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::FindReferences,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
        references_fact_source_traces(&all_occurrences, fallback_state),
    );

    tracing::debug!(
        symbol = %symbol,
        entity_id = ?entity_id,
        verdict = ?receipt.verdict,
        classification = match &result {
            ReferencesCutoverResult::Exact(_) => "exact",
            ReferencesCutoverResult::Ambiguous(_) => "ambiguous",
            ReferencesCutoverResult::LegacyFallback(_) => "legacy_fallback",
        },
        "find-references cutover"
    );

    ReferencesCutoverOutcome { result, receipt }
}

/// Run the live source-backed find-references slice.
///
/// This accepts only source-backed, high-confidence exact syntax or
/// imported/exported occurrence references. Generated, dynamic-boundary,
/// low-confidence, ambiguous, and no-source occurrences stay on the legacy
/// provider path.
pub fn find_references_live_source_backed<Q: SemanticQueries>(
    workspace_index: &WorkspaceIndex,
    semantic_queries: &Q,
    symbol: &str,
    entity_id: EntityId,
) -> ReferencesCutoverOutcome {
    let all_occurrences = semantic_queries.references(entity_id);
    let new_summary = semantic_occurrences_to_summary(&all_occurrences);
    let legacy_locations = workspace_index.find_references(symbol);
    let old_summary = legacy_locations_to_summary(&legacy_locations);

    let live_occurrences = if !all_occurrences.is_empty()
        && all_occurrences.iter().all(|occurrence| {
            is_live_source_backed_reference_occurrence(workspace_index, occurrence)
        }) {
        Some(all_occurrences.clone())
    } else {
        None
    };

    let result = match live_occurrences {
        Some(occurrences) => ReferencesCutoverResult::Exact(occurrences),
        None => ReferencesCutoverResult::LegacyFallback(legacy_locations),
    };
    let fallback_state = references_cutover_fallback_state(&result);

    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::FindReferences,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        vec![references_live_source_backed_quality_note(
            workspace_index,
            &result,
            &all_occurrences,
        )],
        references_fact_source_traces(&all_occurrences, fallback_state),
    );

    tracing::debug!(
        symbol = %symbol,
        entity_id = ?entity_id,
        verdict = ?receipt.verdict,
        classification = match &result {
            ReferencesCutoverResult::Exact(refs)
                if refs.iter().any(is_import_export_occurrence) => "live_import_export",
            ReferencesCutoverResult::Exact(_) => "live_exact",
            ReferencesCutoverResult::Ambiguous(_) => "ambiguous_unreachable",
            ReferencesCutoverResult::LegacyFallback(_) => "legacy_fallback",
        },
        "find-references live source-backed cutover"
    );

    ReferencesCutoverOutcome { result, receipt }
}

/// Classify filtered occurrences into the cutover result category.
fn classify_cutover_result(
    usable: Vec<OccurrenceFact>,
    legacy_locations: Vec<Location>,
) -> ReferencesCutoverResult {
    if usable.is_empty() {
        return ReferencesCutoverResult::LegacyFallback(legacy_locations);
    }

    // Check if any usable occurrence has ambiguous provenance (heuristic/search fallback).
    let has_ambiguous = usable
        .iter()
        .any(|o| matches!(o.provenance, Provenance::NameHeuristic | Provenance::SearchFallback));

    if has_ambiguous {
        ReferencesCutoverResult::Ambiguous(usable)
    } else {
        ReferencesCutoverResult::Exact(usable)
    }
}

fn references_cutover_fallback_state(result: &ReferencesCutoverResult) -> ProviderFallbackState {
    match result {
        ReferencesCutoverResult::Exact(_) | ReferencesCutoverResult::Ambiguous(_) => {
            ProviderFallbackState::Primary
        }
        ReferencesCutoverResult::LegacyFallback(_) => ProviderFallbackState::Fallback,
    }
}

fn is_live_source_backed_reference_occurrence(
    workspace_index: &WorkspaceIndex,
    occurrence: &OccurrenceFact,
) -> bool {
    occurrence.confidence == Confidence::High
        && is_live_reference_provenance(occurrence.provenance)
        && matches!(
            occurrence.kind,
            OccurrenceKind::Reference
                | OccurrenceKind::Read
                | OccurrenceKind::Write
                | OccurrenceKind::Call
                | OccurrenceKind::MethodCall
                | OccurrenceKind::StaticMethodCall
        )
        && workspace_index.semantic_anchor_wire_location(occurrence.anchor_id).is_some()
}

fn is_live_reference_provenance(provenance: Provenance) -> bool {
    matches!(
        provenance,
        Provenance::ExactAst | Provenance::ImportExportInference | Provenance::LiteralRequireImport
    )
}

fn is_import_export_occurrence(occurrence: &OccurrenceFact) -> bool {
    matches!(
        occurrence.provenance,
        Provenance::ImportExportInference | Provenance::LiteralRequireImport
    )
}

fn references_fact_source_traces(
    occurrences: &[OccurrenceFact],
    fallback_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    let mut traces: Vec<ProviderFactTrace> = occurrences
        .iter()
        .map(|occurrence| {
            let (source, provenance, state) = references_trace_shape(occurrence, fallback_state);
            ProviderFactTrace::new(
                ProviderSurface::References,
                source,
                provenance,
                occurrence.confidence,
                ProviderFactFreshness::Fresh,
                state,
                None,
                Some(occurrence.anchor_id),
                Some(1),
            )
        })
        .collect();

    if traces.is_empty() {
        traces.push(ProviderFactTrace::new(
            ProviderSurface::References,
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

fn references_live_source_backed_quality_note(
    workspace_index: &WorkspaceIndex,
    result: &ReferencesCutoverResult,
    occurrences: &[OccurrenceFact],
) -> String {
    let live_occurrences = match result {
        ReferencesCutoverResult::Exact(refs) => refs.len(),
        ReferencesCutoverResult::Ambiguous(_) | ReferencesCutoverResult::LegacyFallback(_) => 0,
    };
    let live_exact_occurrences = match result {
        ReferencesCutoverResult::Exact(refs) => {
            refs.iter().filter(|occurrence| occurrence.provenance == Provenance::ExactAst).count()
        }
        ReferencesCutoverResult::Ambiguous(_) | ReferencesCutoverResult::LegacyFallback(_) => 0,
    };
    let live_import_export_occurrences = match result {
        ReferencesCutoverResult::Exact(refs) => {
            refs.iter().filter(|occurrence| is_import_export_occurrence(occurrence)).count()
        }
        ReferencesCutoverResult::Ambiguous(_) | ReferencesCutoverResult::LegacyFallback(_) => 0,
    };
    let legacy_fallbacks =
        usize::from(matches!(result, ReferencesCutoverResult::LegacyFallback(_)));
    let dynamic_boundary_blockers =
        occurrences.iter().filter(|occurrence| is_dynamic_boundary_occurrence(occurrence)).count();
    let generated_no_source_fallbacks = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.kind == OccurrenceKind::GeneratedUse
                || workspace_index.semantic_anchor_wire_location(occurrence.anchor_id).is_none()
        })
        .count();
    let low_confidence_fallbacks =
        occurrences.iter().filter(|occurrence| occurrence.confidence != Confidence::High).count();
    let import_export_candidates =
        occurrences.iter().filter(|occurrence| is_import_export_occurrence(occurrence)).count();
    let import_export_fallbacks =
        import_export_candidates.saturating_sub(live_import_export_occurrences);
    let ambiguous_fallbacks = usize::from(!occurrences.is_empty() && live_occurrences == 0);

    format!(
        "references live source-backed proof: live_occurrences={live_occurrences}; live_exact_occurrences={live_exact_occurrences}; live_import_export_occurrences={live_import_export_occurrences}; legacy_fallbacks={legacy_fallbacks}; compiler_fact_candidates={}; ambiguous_fallbacks={ambiguous_fallbacks}; import_export_candidates={import_export_candidates}; import_export_fallbacks={import_export_fallbacks}; stale_fact_blockers=0; generated_no_source_fallbacks={generated_no_source_fallbacks}; dynamic_boundary_blockers={dynamic_boundary_blockers}; low_confidence_fallbacks={low_confidence_fallbacks}; partial live exact/imported references cutover",
        occurrences.len()
    )
}

fn references_shadow_quality_note(
    legacy_locations: &[Location],
    occurrences: &[OccurrenceFact],
) -> String {
    let answer_count = references_answer_occurrence_count(occurrences);
    let generated_label_count = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.kind == OccurrenceKind::GeneratedUse
                || occurrence.provenance == Provenance::FrameworkSynthesis
        })
        .count();
    let dynamic_boundary_blockers =
        occurrences.iter().filter(|occurrence| is_dynamic_boundary_occurrence(occurrence)).count();
    let noise_delta = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.confidence == Confidence::Low
                || matches!(
                    occurrence.provenance,
                    Provenance::NameHeuristic | Provenance::SearchFallback
                )
        })
        .count();

    format!(
        "references shadow proof: legacy_candidates={}; compiler_fact_candidates={}; answer_candidates={answer_count}; rank_delta={}; noise_delta={noise_delta}; generated_labels={generated_label_count}; dynamic_boundary_blockers={dynamic_boundary_blockers}; stale_fact_blockers=0; blocked_candidates={dynamic_boundary_blockers}; no live navigation behavior change",
        legacy_locations.len(),
        occurrences.len(),
        signed_count_delta(legacy_locations.len(), answer_count)
    )
}

fn references_answer_occurrence_count(occurrences: &[OccurrenceFact]) -> usize {
    occurrences
        .iter()
        .filter(|occurrence| {
            let (source, _, state) =
                references_trace_shape(occurrence, ProviderFallbackState::Shadow);
            state != ProviderFallbackState::Blocked
                && source != ProviderFactSourceKind::Fallback
                && occurrence.confidence != Confidence::Low
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

fn references_trace_shape(
    occurrence: &OccurrenceFact,
    fallback_state: ProviderFallbackState,
) -> (ProviderFactSourceKind, Provenance, ProviderFallbackState) {
    if is_dynamic_boundary_occurrence(occurrence) {
        return (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            ProviderFallbackState::Blocked,
        );
    }

    match occurrence.provenance {
        Provenance::FrameworkSynthesis => (
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            fallback_state,
        ),
        Provenance::ImportExportInference | Provenance::PragmaInference => {
            (ProviderFactSourceKind::CompilerFact, occurrence.provenance, fallback_state)
        }
        Provenance::NameHeuristic | Provenance::SearchFallback => (
            ProviderFactSourceKind::Fallback,
            occurrence.provenance,
            ProviderFallbackState::Fallback,
        ),
        Provenance::ExactAst
        | Provenance::DesugaredAst
        | Provenance::SemanticAnalyzer
        | Provenance::LiteralRequireImport => {
            (ProviderFactSourceKind::SemanticFact, occurrence.provenance, fallback_state)
        }
        Provenance::DynamicBoundary => (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            ProviderFallbackState::Blocked,
        ),
    }
}

fn is_dynamic_boundary_occurrence(occurrence: &OccurrenceFact) -> bool {
    occurrence.provenance == Provenance::DynamicBoundary
        || matches!(
            occurrence.kind,
            OccurrenceKind::DynamicBoundary | OccurrenceKind::TypeglobReference
        )
}

/// Convert legacy `Location` results into a [`ShadowResultSummary`].
fn legacy_locations_to_summary(locations: &[Location]) -> ShadowResultSummary {
    if locations.is_empty() {
        // Legacy returned results but found nothing — available with 0 matches.
        // Note: unlike definition where None means unavailable, find_references
        // always returns a Vec (possibly empty), so it's always "available".
        return summarize_identities(Some(Vec::new()));
    }

    let identities: Vec<String> = locations
        .iter()
        .map(|loc| format!("{}:{}:{}", loc.uri, loc.range.start.line, loc.range.start.column))
        .collect();

    summarize_identities(Some(identities))
}

/// Convert semantic `OccurrenceFact` results into a [`ShadowResultSummary`].
fn semantic_occurrences_to_summary(occurrences: &[OccurrenceFact]) -> ShadowResultSummary {
    if occurrences.is_empty() {
        return summarize_identities(Some(Vec::new()));
    }

    let identities: Vec<String> = occurrences
        .iter()
        .map(|o| {
            // Use occurrence_id + anchor_id as a stable identity since we
            // don't have the resolved URI/line from the semantic path yet.
            format!("occ:{}:anchor:{}", o.id.0, o.anchor_id.0)
        })
        .collect();

    summarize_identities(Some(identities))
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact,
        OccurrenceId, OccurrenceKind, Provenance, RenamePlan, SafeDeletePlan, ScopeId,
        VisibleSymbol,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;
    use url::Url;

    // ── Minimal SemanticQueries stub for testing ──

    struct StubSemanticQueries {
        references_result: Vec<OccurrenceFact>,
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
            self.references_result.clone()
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

    fn make_occurrence(
        occ_id: u64,
        anchor_id: u64,
        entity_id: u64,
        kind: OccurrenceKind,
        provenance: Provenance,
        confidence: Confidence,
    ) -> OccurrenceFact {
        OccurrenceFact {
            id: OccurrenceId(occ_id),
            kind,
            entity_id: Some(EntityId(entity_id)),
            anchor_id: AnchorId(anchor_id),
            scope_id: None,
            provenance,
            confidence,
        }
    }

    fn make_ref_occurrence(occ_id: u64, anchor_id: u64, entity_id: u64) -> OccurrenceFact {
        make_occurrence(
            occ_id,
            anchor_id,
            entity_id,
            OccurrenceKind::Reference,
            Provenance::ExactAst,
            Confidence::High,
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

    fn build_real_workspace_references_index() -> Result<WorkspaceIndex, Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();
        index.index_file(
            file_url("/lib/Real/Nav.pm")?,
            "package Real::Nav;\nsub legacy_helper { 1 }\n1;\n".to_string(),
        )?;
        index.index_file(
            file_url("/script/app.pl")?,
            "use Real::Nav;\nReal::Nav::legacy_helper();\nlegacy_helper();\n".to_string(),
        )?;
        Ok(index)
    }

    fn source_backed_exact_references()
    -> Result<(WorkspaceIndex, EntityId, Vec<OccurrenceFact>), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = file_url("/lib/LiveRefs.pm")?;
        index.index_file(
            uri.clone(),
            "package LiveRefs;\nsub target { 1 }\nsub caller {\n    target();\n    LiveRefs::target();\n}\n1;\n"
                .to_string(),
        )?;

        let (entity_id, references) = index
            .with_semantic_queries_for_uri(uri.as_str(), |file_id, queries| {
                let ctx = QueryContext::new(file_id, None, Some(0));
                let candidate = queries.definitions("LiveRefs::target", &ctx).into_iter().next()?;
                let references = queries.references(candidate.entity_id);
                Some((candidate.entity_id, references))
            })
            .flatten()
            .ok_or("missing source-backed exact references")?;

        if references.is_empty() {
            return Err("expected at least one source-backed exact reference".into());
        }

        Ok((index, entity_id, references))
    }

    // ── Shadow mode tests ──

    #[test]
    fn shadow_both_empty_yields_same() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { references_result: vec![] };

        let result = find_references_shadow(&index, &queries, "No::Such::Symbol", EntityId(999));

        assert!(result.legacy_result.is_empty());
        assert_eq!(result.receipt.query, ShadowQueryName::FindReferences);
        // Both paths available with 0 matches → Same.
        assert_eq!(result.receipt.old_result.available, true);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    #[test]
    fn shadow_new_path_has_occurrences_old_empty() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ = make_ref_occurrence(1, 10, 20);
        let queries = StubSemanticQueries { references_result: vec![occ] };

        let result = find_references_shadow(&index, &queries, "Foo::bar", EntityId(20));

        assert!(result.legacy_result.is_empty());
        assert_eq!(result.receipt.old_result.available, true);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.new_result.match_count, 1);
        // New has more matches → Improved.
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        Ok(())
    }

    #[test]
    fn shadow_returns_legacy_result() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { references_result: vec![] };

        let result = find_references_shadow(&index, &queries, "test_symbol", EntityId(1));

        // Legacy result is always returned during shadow phase.
        // With an empty workspace index, legacy returns empty.
        assert!(result.legacy_result.is_empty());
        assert_eq!(result.receipt.query, ShadowQueryName::FindReferences);
        assert_eq!(result.receipt.input.symbol, "test_symbol");
        Ok(())
    }

    #[test]
    fn shadow_receipt_uses_find_references_query_name() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { references_result: vec![] };

        let result = find_references_shadow(&index, &queries, "test", EntityId(1));

        assert_eq!(result.receipt.query, ShadowQueryName::FindReferences);
        assert_eq!(result.receipt.input.symbol, "test");
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    #[test]
    fn shadow_multiple_new_occurrences() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ1 = make_ref_occurrence(1, 10, 20);
        let occ2 = make_ref_occurrence(2, 30, 20);
        let occ3 = make_ref_occurrence(3, 50, 20);
        let queries = StubSemanticQueries { references_result: vec![occ1, occ2, occ3] };

        let result = find_references_shadow(&index, &queries, "Foo::bar", EntityId(20));

        assert_eq!(result.receipt.new_result.match_count, 3);
        assert_eq!(result.receipt.new_result.available, true);
        Ok(())
    }

    // ── Summary helper tests ──

    #[test]
    fn legacy_locations_to_summary_empty() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_locations_to_summary(&[]);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn legacy_locations_to_summary_multiple() -> Result<(), Box<dyn std::error::Error>> {
        use perl_parser_core::position::{Position, Range};

        let locations = vec![
            Location {
                uri: "file:///a.pm".to_string(),
                range: Range { start: Position::new(0, 1, 5), end: Position::new(0, 1, 10) },
            },
            Location {
                uri: "file:///b.pm".to_string(),
                range: Range { start: Position::new(0, 3, 2), end: Position::new(0, 3, 8) },
            },
        ];
        let summary = super::legacy_locations_to_summary(&locations);
        assert!(summary.available);
        assert_eq!(summary.match_count, 2);
        assert_eq!(summary.identities.len(), 2);
        Ok(())
    }

    #[test]
    fn semantic_occurrences_to_summary_empty() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::semantic_occurrences_to_summary(&[]);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_occurrences_to_summary_multiple() -> Result<(), Box<dyn std::error::Error>> {
        let occurrences = vec![make_ref_occurrence(1, 10, 20), make_ref_occurrence(2, 30, 20)];
        let summary = super::semantic_occurrences_to_summary(&occurrences);
        assert!(summary.available);
        assert_eq!(summary.match_count, 2);
        assert_eq!(summary.identities.len(), 2);
        Ok(())
    }

    // ── Cutover tests ──

    #[test]
    fn cutover_exact_typed_references() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ = make_ref_occurrence(1, 10, 20);
        let queries = StubSemanticQueries { references_result: vec![occ.clone()] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        match &outcome.result {
            ReferencesCutoverResult::Exact(refs) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0], occ);
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        assert_eq!(outcome.receipt.query, ShadowQueryName::FindReferences);
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_no_occurrences() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let queries = StubSemanticQueries { references_result: vec![] };

        let outcome = find_references_cutover(&index, &queries, "No::Such", EntityId(999));

        match &outcome.result {
            ReferencesCutoverResult::LegacyFallback(locs) => {
                // Legacy also finds nothing for an empty index.
                assert!(locs.is_empty());
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_all_dynamic_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let dynamic_occ = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::Reference,
            Provenance::DynamicBoundary,
            Confidence::Low,
        );
        let queries = StubSemanticQueries { references_result: vec![dynamic_occ] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        match &outcome.result {
            ReferencesCutoverResult::LegacyFallback(_) => {}
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_all_low_confidence() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let low_occ = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::Reference,
            Provenance::NameHeuristic,
            Confidence::Low,
        );
        let queries = StubSemanticQueries { references_result: vec![low_occ] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        match &outcome.result {
            ReferencesCutoverResult::LegacyFallback(_) => {}
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_filters_dynamic_keeps_exact() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let good = make_ref_occurrence(1, 10, 20);
        let dynamic = make_occurrence(
            2,
            30,
            20,
            OccurrenceKind::Reference,
            Provenance::DynamicBoundary,
            Confidence::Low,
        );
        let queries = StubSemanticQueries { references_result: vec![good.clone(), dynamic] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        // Dynamic occurrence filtered out, leaving one usable → Exact.
        match &outcome.result {
            ReferencesCutoverResult::Exact(refs) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0], good);
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
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
    fn cutover_ambiguous_when_heuristic_provenance() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let exact = make_ref_occurrence(1, 10, 20);
        let heuristic = make_occurrence(
            2,
            30,
            20,
            OccurrenceKind::Reference,
            Provenance::NameHeuristic,
            Confidence::Medium,
        );
        let queries = StubSemanticQueries { references_result: vec![exact, heuristic] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        match &outcome.result {
            ReferencesCutoverResult::Ambiguous(refs) => {
                assert_eq!(refs.len(), 2);
            }
            other => return Err(format!("expected Ambiguous, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_all_occurrences() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ1 = make_ref_occurrence(1, 10, 20);
        let occ2 = make_occurrence(
            2,
            30,
            20,
            OccurrenceKind::Reference,
            Provenance::DynamicBoundary,
            Confidence::Low,
        );
        let queries = StubSemanticQueries { references_result: vec![occ1, occ2] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        // Receipt should reflect ALL occurrences (before filtering).
        assert_eq!(outcome.receipt.new_result.match_count, 2);
        assert_eq!(outcome.receipt.query, ShadowQueryName::FindReferences);
        Ok(())
    }

    #[test]
    fn references_compiler_shadow_traces_import_export_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::Reference,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let queries = StubSemanticQueries { references_result: vec![occ] };

        let result = find_references_shadow(&index, &queries, "imported_func", EntityId(20));
        let trace = first_trace(&result.receipt)?;

        assert!(result.legacy_result.is_empty());
        assert_eq!(trace.surface, ProviderSurface::References);
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::ImportExportInference);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn references_compiler_shadow_traces_framework_generated_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ = make_occurrence(
            2,
            20,
            30,
            OccurrenceKind::GeneratedUse,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let queries = StubSemanticQueries { references_result: vec![occ] };

        let result = find_references_shadow(&index, &queries, "generated_accessor", EntityId(30));
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::References);
        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn references_compiler_shadow_traces_dynamic_boundary_as_blocked()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occ = make_occurrence(
            3,
            30,
            40,
            OccurrenceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::High,
        );
        let queries = StubSemanticQueries { references_result: vec![occ] };

        let result = find_references_shadow(&index, &queries, "dynamic_symbol", EntityId(40));
        let trace = first_trace(&result.receipt)?;

        assert_eq!(trace.surface, ProviderSurface::References);
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        assert!(result.legacy_result.is_empty());
        Ok(())
    }

    #[test]
    fn references_compiler_shadow_low_confidence_does_not_outrank_exact()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let exact = make_ref_occurrence(1, 10, 20);
        let low = make_occurrence(
            2,
            30,
            20,
            OccurrenceKind::Reference,
            Provenance::NameHeuristic,
            Confidence::Low,
        );
        let queries = StubSemanticQueries { references_result: vec![exact.clone(), low] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        match &outcome.result {
            ReferencesCutoverResult::Exact(refs) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0], exact);
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
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
    fn references_shadow_records_real_workspace_quality_receipt()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = build_real_workspace_references_index()?;
        let imported = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::Reference,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let generated = make_occurrence(
            2,
            11,
            20,
            OccurrenceKind::GeneratedUse,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let dynamic = make_occurrence(
            3,
            12,
            20,
            OccurrenceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::High,
        );
        let low_confidence = make_occurrence(
            4,
            13,
            20,
            OccurrenceKind::Reference,
            Provenance::NameHeuristic,
            Confidence::Low,
        );
        let queries = StubSemanticQueries {
            references_result: vec![imported, generated, dynamic, low_confidence],
        };

        let result =
            find_references_shadow(&index, &queries, "Real::Nav::legacy_helper", EntityId(20));

        assert!(!result.legacy_result.is_empty(), "legacy workspace references should resolve");
        assert_eq!(result.receipt.new_result.match_count, 4);
        let note = result.receipt.notes.join(" ");
        assert!(note.contains("compiler_fact_candidates=4"));
        assert!(note.contains("answer_candidates=2"));
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
    fn references_live_source_backed_accepts_source_backed_exact_ast_occurrences()
    -> Result<(), Box<dyn std::error::Error>> {
        let (index, entity_id, references) = source_backed_exact_references()?;
        let queries = StubSemanticQueries { references_result: references.clone() };

        let outcome =
            find_references_live_source_backed(&index, &queries, "LiveRefs::target", entity_id);

        assert_eq!(outcome.result, ReferencesCutoverResult::Exact(references.clone()));
        assert!(
            references.iter().all(|reference| index
                .semantic_anchor_wire_location(reference.anchor_id)
                .is_some())
        );
        let note = outcome.receipt.notes.join(" ");
        assert!(note.contains("live_exact_occurrences="));
        assert!(note.contains("legacy_fallbacks=0"));
        assert!(note.contains("partial live exact/imported references cutover"));
        assert!(outcome.receipt.fact_source_traces.iter().all(|trace| {
            trace.source == ProviderFactSourceKind::SemanticFact
                && trace.provenance == Provenance::ExactAst
                && trace.confidence == Confidence::High
                && trace.freshness == ProviderFactFreshness::Fresh
                && trace.fallback_state == ProviderFallbackState::Primary
        }));
        Ok(())
    }

    #[test]
    fn references_live_source_backed_falls_back_for_non_source_backed_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let occurrence = make_ref_occurrence(1, 10, 20);
        let queries = StubSemanticQueries { references_result: vec![occurrence] };

        let outcome =
            find_references_live_source_backed(&index, &queries, "Foo::bar", EntityId(20));

        assert!(matches!(outcome.result, ReferencesCutoverResult::LegacyFallback(_)));
        let note = outcome.receipt.notes.join(" ");
        assert!(note.contains("generated_no_source_fallbacks=1"));
        let trace = first_trace(&outcome.receipt)?;
        assert_eq!(trace.fallback_state, ProviderFallbackState::Fallback);
        Ok(())
    }

    #[test]
    fn references_live_source_backed_accepts_import_export_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let (index, entity_id, references) = source_backed_exact_references()?;
        let mut imported = references.first().ok_or("missing reference")?.clone();
        imported.provenance = Provenance::ImportExportInference;
        let queries = StubSemanticQueries { references_result: vec![imported.clone()] };

        let outcome = find_references_live_source_backed(&index, &queries, "target", entity_id);

        assert_eq!(outcome.result, ReferencesCutoverResult::Exact(vec![imported]));
        let note = outcome.receipt.notes.join(" ");
        assert!(note.contains("live_import_export_occurrences=1"));
        assert!(note.contains("import_export_fallbacks=0"));
        assert!(note.contains("partial live exact/imported references cutover"));
        let trace = first_trace(&outcome.receipt)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn references_live_source_backed_accepts_literal_require_import_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let (index, entity_id, references) = source_backed_exact_references()?;
        let mut imported = references.first().ok_or("missing reference")?.clone();
        imported.provenance = Provenance::LiteralRequireImport;
        let queries = StubSemanticQueries { references_result: vec![imported.clone()] };

        let outcome = find_references_live_source_backed(&index, &queries, "target", entity_id);

        assert_eq!(outcome.result, ReferencesCutoverResult::Exact(vec![imported]));
        let note = outcome.receipt.notes.join(" ");
        assert!(note.contains("live_import_export_occurrences=1"));
        assert!(note.contains("import_export_fallbacks=0"));
        let trace = first_trace(&outcome.receipt)?;
        assert_eq!(trace.source, ProviderFactSourceKind::SemanticFact);
        assert_eq!(trace.provenance, Provenance::LiteralRequireImport);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn references_live_source_backed_blocks_dynamic_boundary_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let dynamic = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::High,
        );
        let queries = StubSemanticQueries { references_result: vec![dynamic] };

        let outcome =
            find_references_live_source_backed(&index, &queries, "Foo::dynamic", EntityId(20));

        assert!(matches!(outcome.result, ReferencesCutoverResult::LegacyFallback(_)));
        let note = outcome.receipt.notes.join(" ");
        assert!(note.contains("dynamic_boundary_blockers=1"));
        let trace = first_trace(&outcome.receipt)?;
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn cutover_medium_confidence_is_usable() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let medium = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::Call,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
        );
        let queries = StubSemanticQueries { references_result: vec![medium.clone()] };

        let outcome = find_references_cutover(&index, &queries, "Foo::bar", EntityId(20));

        // Medium confidence is usable — should produce Exact, not fallback.
        match &outcome.result {
            ReferencesCutoverResult::Exact(refs) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0], medium);
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    // ── classify_cutover_result tests ──

    #[test]
    fn classify_cutover_result_empty_is_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let result = super::classify_cutover_result(vec![], vec![]);
        match result {
            ReferencesCutoverResult::LegacyFallback(locs) => {
                assert!(locs.is_empty());
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn classify_cutover_result_exact_refs() -> Result<(), Box<dyn std::error::Error>> {
        let occ = make_ref_occurrence(1, 10, 20);
        let result = super::classify_cutover_result(vec![occ], vec![]);
        match result {
            ReferencesCutoverResult::Exact(refs) => {
                assert_eq!(refs.len(), 1);
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn classify_cutover_result_ambiguous_with_search_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let occ = make_occurrence(
            1,
            10,
            20,
            OccurrenceKind::Reference,
            Provenance::SearchFallback,
            Confidence::Medium,
        );
        let result = super::classify_cutover_result(vec![occ], vec![]);
        match result {
            ReferencesCutoverResult::Ambiguous(refs) => {
                assert_eq!(refs.len(), 1);
            }
            other => return Err(format!("expected Ambiguous, got {:?}", other).into()),
        }
        Ok(())
    }
}
