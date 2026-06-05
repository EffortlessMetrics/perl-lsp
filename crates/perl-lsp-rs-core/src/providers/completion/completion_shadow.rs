//! Completion shadow compare and cutover paths.
//!
//! Provides entry points for semantic completion validation:
//!
//! 1. **Shadow mode** ([`completion_visibility_shadow`]) — runs both legacy
//!    completion symbol gathering and new `SemanticQueries::visible_symbols_at`
//!    side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`completion_visibility_cutover`]) — uses the semantic
//!    path as the primary source of truth:
//!    - *Exact*: high-confidence visible symbol → rank high, ordered by
//!      provenance.
//!    - *Ambiguous*: lower-confidence or heuristic → rank lower.
//!    - *Dynamic / Unavailable*: dynamic-boundary or unavailable → show low
//!      or omit.
//!
//! 3. **Method shadow mode** ([`method_completion_shadow`]) — compares method
//!    completion labels against `SemanticQueries::method_candidates` for a
//!    known receiver package and method labels/probes.
//!
//! # Requirements
//!
//! - **Req 9.3**: Completion calls `SemanticQueries::visible_symbols_at`.
//! - **Req 10.1**: Maintain existing query path as fallback during validation.
//! - **Req 10.2**: Shadow-compare runs both old and new paths, producing
//!   deterministic receipts.
//! - **Req 10.8**: Scorecard gate: explicit import fixtures pass, default
//!   export fixtures pass, empty import suppresses defaults, tag export
//!   fixtures pass.
//! - **Req 22.5**: Exact → rank high; Ambiguous → rank lower;
//!   Dynamic/Unavailable → show low or omit.

use perl_semantic_facts::{
    Confidence, DefinitionCandidate, EntityKind, FileId, Provenance, ProviderFactFreshness,
    ProviderFactSourceKind, ProviderFactTrace, ProviderFallbackState, ProviderSurface, ScopeId,
    VisibleSymbol, VisibleSymbolSource,
};
use perl_workspace::semantic::queries::SemanticQueries;
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

/// Result of a shadow-compared completion visibility request.
///
/// Contains the legacy result (which callers should use during the shadow
/// phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct CompletionShadowResult {
    /// Legacy result — the symbol names returned by the existing completion
    /// provider. Callers should use this during the shadow phase.
    pub legacy_symbols: Vec<String>,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Result of a shadow-compared method completion request.
///
/// Contains the legacy method labels and a `MethodCandidates` shadow receipt.
/// Callers should continue returning legacy completion items while this runs in
/// shadow mode.
#[derive(Debug)]
#[non_exhaustive]
pub struct MethodCompletionShadowResult {
    /// Legacy method labels returned by the existing completion provider.
    pub legacy_methods: Vec<String>,
    /// Method names probed through `SemanticQueries::method_candidates`.
    pub probed_methods: Vec<String>,
    /// Shadow-compare receipt comparing legacy labels and semantic candidates.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run completion visibility through both legacy and semantic paths,
/// producing a shadow-compare receipt.
///
/// # Arguments
///
/// * `legacy_symbols` — symbol names gathered by the existing completion
///   provider (passed in because the legacy path is provider-internal).
/// * `semantic_queries` — the new semantic query facade.
/// * `file_id` — file containing the completion request.
/// * `byte_offset` — byte offset of the cursor within the file.
/// * `scope_id` — scope enclosing the cursor, when known.
/// * `input_label` — human-readable label for the receipt input (e.g.
///   the import statement or prefix being completed).
///
/// # Returns
///
/// A [`CompletionShadowResult`] containing the legacy symbols and a receipt.
/// The caller should return the legacy symbols to the LSP client during the
/// shadow phase.
pub fn completion_visibility_shadow<Q: SemanticQueries>(
    legacy_symbols: Vec<String>,
    semantic_queries: &Q,
    file_id: FileId,
    byte_offset: u32,
    scope_id: Option<ScopeId>,
    input_label: &str,
) -> CompletionShadowResult {
    // ── Legacy path ──
    let old_summary = legacy_symbols_to_summary(&legacy_symbols);

    // ── New compiler-fact path ──
    let new_visible = semantic_queries.visible_symbols_at(file_id, byte_offset, scope_id);
    let new_summary = semantic_visible_to_summary(&new_visible);
    let notes = completion_shadow_notes(&legacy_symbols, &new_visible);
    let fact_source_traces =
        completion_fact_source_traces(&new_visible, ProviderFallbackState::Shadow);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::CompletionVisibility,
        ShadowQueryInput { symbol: input_label.to_string() },
        old_summary,
        new_summary,
        notes,
        fact_source_traces,
    );

    tracing::debug!(
        input = %input_label,
        verdict = ?receipt.verdict,
        old_count = receipt.old_result.match_count,
        new_count = receipt.new_result.match_count,
        "completion visibility shadow compare"
    );

    CompletionShadowResult { legacy_symbols, receipt }
}

/// Shadow method completion labels against semantic `method_candidates`.
///
/// `SemanticQueries::method_candidates` is an exact lookup, not a method-list
/// enumerator. The caller therefore provides the current legacy labels plus
/// any additional method names it wants to probe. Legacy labels are always
/// included in the probe set so the receipt can detect semantic regressions
/// for methods the existing provider already returns.
pub fn method_completion_shadow<Q: SemanticQueries>(
    legacy_methods: Vec<String>,
    probe_methods: Vec<String>,
    semantic_queries: &Q,
    receiver_package: &str,
    method_prefix: &str,
) -> MethodCompletionShadowResult {
    let legacy_methods_for_prefix = method_names_for_prefix(&legacy_methods, method_prefix);
    let old_summary = legacy_symbols_to_summary(&legacy_methods_for_prefix);
    let probed_methods =
        method_probe_names(&legacy_methods_for_prefix, &probe_methods, method_prefix);

    let mut semantic_candidates = Vec::new();
    for method_name in &probed_methods {
        semantic_candidates
            .extend(semantic_queries.method_candidates(receiver_package, method_name));
    }
    let new_summary = method_candidates_to_summary(&semantic_candidates);
    let fact_source_traces =
        method_completion_fact_source_traces(&semantic_candidates, ProviderFallbackState::Shadow);

    let input_label = if method_prefix.is_empty() {
        format!("{receiver_package}->")
    } else {
        format!("{receiver_package}->{method_prefix}")
    };

    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::MethodCandidates,
        ShadowQueryInput { symbol: input_label },
        old_summary,
        new_summary,
        vec![
            format!("receiver_package={receiver_package}"),
            format!("method_prefix={method_prefix}"),
            format!("probed_methods={}", probed_methods.len()),
        ],
        fact_source_traces,
    );

    tracing::debug!(
        receiver_package = %receiver_package,
        method_prefix = %method_prefix,
        probed_methods = probed_methods.len(),
        verdict = ?receipt.verdict,
        old_count = receipt.old_result.match_count,
        new_count = receipt.new_result.match_count,
        "method completion shadow compare"
    );

    MethodCompletionShadowResult { legacy_methods, probed_methods, receipt }
}

// ── Cutover types ──

/// A completion symbol with a ranking tier derived from semantic visibility.
///
/// Follows the fallback policy table (Req 22.5):
/// - Exact → rank high
/// - Ambiguous → rank lower
/// - Dynamic/Unavailable → show low or omit
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedCompletionSymbol {
    /// The visible symbol from the semantic path.
    pub symbol: VisibleSymbol,
    /// Ranking tier for completion sort order.
    pub tier: CompletionRankTier,
}

/// Ranking tier for a completion symbol based on semantic visibility
/// classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionRankTier {
    /// High confidence, exact match — rank at the top.
    High,
    /// Ambiguous or lower confidence — rank below exact matches.
    Medium,
    /// Dynamic boundary or unavailable — show low or omit.
    Low,
}

/// Classification of the semantic completion result for cutover decisions.
///
/// Follows the fallback policy table (Req 22.5):
/// - Exact → rank symbol high
/// - Ambiguous → rank symbol lower
/// - LegacyFallback → semantic path unavailable; use legacy result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionCutoverResult {
    /// Semantic path produced ranked completion symbols.
    Semantic(Vec<RankedCompletionSymbol>),
    /// Semantic path produced no usable result — fall back to legacy.
    LegacyFallback(Vec<String>),
}

/// Outcome of a cutover completion visibility request.
///
/// Contains the classified result and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct CompletionCutoverOutcome {
    /// The classified cutover result.
    pub result: CompletionCutoverResult,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run completion visibility with the semantic path as primary, falling
/// back to legacy when the semantic result is unavailable.
///
/// # Decision logic
///
/// 1. Call `SemanticQueries::visible_symbols_at` for the cursor position.
/// 2. Rank each visible symbol into a [`CompletionRankTier`]:
///    - High-confidence symbols from ExplicitImport, DefaultExport,
///      ExportTag, LocalLexical, LocalPackage, Constant, Generated →
///      `CompletionRankTier::High`.
///    - Medium-confidence or External/heuristic symbols →
///      `CompletionRankTier::Medium`.
///    - DynamicUnknown or Low-confidence symbols →
///      `CompletionRankTier::Low`.
/// 3. Sort ranked symbols by tier, then semantic provenance: local lexical,
///    same-package/constant, explicit import, default export, export tag,
///    generated, external, dynamic unknown.
/// 4. If no symbols are returned, fall back to legacy.
/// 5. Emit a shadow-compare receipt regardless of outcome.
///
/// # Arguments
///
/// * `legacy_symbols` — symbol names from the existing completion provider.
/// * `semantic_queries` — the semantic query facade (primary path).
/// * `file_id` — file containing the completion request.
/// * `byte_offset` — byte offset of the cursor within the file.
/// * `scope_id` — scope enclosing the cursor, when known.
/// * `input_label` — human-readable label for the receipt input.
///
/// # Returns
///
/// A [`CompletionCutoverOutcome`] with the classified result and receipt.
pub fn completion_visibility_cutover<Q: SemanticQueries>(
    legacy_symbols: Vec<String>,
    semantic_queries: &Q,
    file_id: FileId,
    byte_offset: u32,
    scope_id: Option<ScopeId>,
    input_label: &str,
) -> CompletionCutoverOutcome {
    // ── Semantic path (primary) ──
    let all_visible = semantic_queries.visible_symbols_at(file_id, byte_offset, scope_id);
    let new_summary = semantic_visible_to_summary(&all_visible);

    // ── Legacy path (for fallback and receipt) ──
    let old_summary = legacy_symbols_to_summary(&legacy_symbols);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::CompletionVisibility,
        ShadowQueryInput { symbol: input_label.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
    );

    // ── Classify result ──
    let result = if all_visible.is_empty() {
        CompletionCutoverResult::LegacyFallback(legacy_symbols)
    } else {
        let mut ranked: Vec<RankedCompletionSymbol> =
            all_visible.into_iter().map(rank_visible_symbol).collect();
        ranked.sort_by(compare_ranked_completion_symbols);
        CompletionCutoverResult::Semantic(ranked)
    };

    tracing::debug!(
        input = %input_label,
        verdict = ?receipt.verdict,
        classification = match &result {
            CompletionCutoverResult::Semantic(syms) => {
                if syms.iter().all(|s| s.tier == CompletionRankTier::High) {
                    "exact"
                } else {
                    "mixed"
                }
            }
            CompletionCutoverResult::LegacyFallback(_) => "legacy_fallback",
        },
        "completion visibility cutover"
    );

    CompletionCutoverOutcome { result, receipt }
}

/// Rank a single visible symbol into a completion tier.
///
/// Follows the fallback policy table:
/// - Exact (high-confidence, known source) → High
/// - Ambiguous (medium-confidence, external) → Medium
/// - Dynamic/Unavailable (dynamic-unknown, low-confidence) → Low
fn rank_visible_symbol(symbol: VisibleSymbol) -> RankedCompletionSymbol {
    let tier = match (&symbol.source, symbol.confidence) {
        // Dynamic boundary symbols always rank low.
        (VisibleSymbolSource::DynamicUnknown, _) => CompletionRankTier::Low,
        // Low confidence from any source ranks low.
        (_, Confidence::Low) => CompletionRankTier::Low,
        // High-confidence symbols from well-known sources rank high.
        (VisibleSymbolSource::ExplicitImport, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        (VisibleSymbolSource::DefaultExport, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        (VisibleSymbolSource::ExportTag, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        (VisibleSymbolSource::LocalLexical, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        (VisibleSymbolSource::LocalPackage, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        (VisibleSymbolSource::Constant, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        (VisibleSymbolSource::Generated, Confidence::High | Confidence::Medium) => {
            CompletionRankTier::High
        }
        // External symbols rank medium.
        (VisibleSymbolSource::External, _) => CompletionRankTier::Medium,
    };

    RankedCompletionSymbol { symbol, tier }
}

fn compare_ranked_completion_symbols(
    a: &RankedCompletionSymbol,
    b: &RankedCompletionSymbol,
) -> std::cmp::Ordering {
    a.tier
        .cmp(&b.tier)
        .then_with(|| {
            completion_source_priority(&a.symbol.source)
                .cmp(&completion_source_priority(&b.symbol.source))
        })
        .then_with(|| a.symbol.confidence.cmp(&b.symbol.confidence))
        .then_with(|| a.symbol.name.cmp(&b.symbol.name))
}

fn completion_source_priority(source: &VisibleSymbolSource) -> u8 {
    match source {
        VisibleSymbolSource::LocalLexical => 0,
        VisibleSymbolSource::LocalPackage => 1,
        VisibleSymbolSource::Constant => 2,
        VisibleSymbolSource::ExplicitImport => 3,
        VisibleSymbolSource::DefaultExport => 4,
        VisibleSymbolSource::ExportTag => 5,
        VisibleSymbolSource::Generated => 6,
        VisibleSymbolSource::External => 7,
        VisibleSymbolSource::DynamicUnknown => 8,
    }
}

/// Convert legacy completion symbol names into a [`ShadowResultSummary`].
fn legacy_symbols_to_summary(symbols: &[String]) -> ShadowResultSummary {
    if symbols.is_empty() {
        return summarize_identities(Some(Vec::new()));
    }

    summarize_identities(Some(symbols.to_vec()))
}

/// Convert semantic `VisibleSymbol` results into a [`ShadowResultSummary`].
fn semantic_visible_to_summary(symbols: &[VisibleSymbol]) -> ShadowResultSummary {
    if symbols.is_empty() {
        return summarize_identities(Some(Vec::new()));
    }

    let identities: Vec<String> = symbols
        .iter()
        .filter(|s| is_completion_candidate_source(&s.source))
        .map(|s| {
            // Use name + source as a stable identity for shadow comparison.
            format!("{}:{:?}", s.name, s.source)
        })
        .collect();

    summarize_identities(Some(identities))
}

fn completion_shadow_notes(
    legacy_symbols: &[String],
    visible_symbols: &[VisibleSymbol],
) -> Vec<String> {
    let candidate_count = visible_symbols
        .iter()
        .filter(|symbol| is_completion_candidate_source(&symbol.source))
        .count();
    let rank_delta = i64::try_from(candidate_count).unwrap_or(i64::MAX)
        - i64::try_from(legacy_symbols.len()).unwrap_or(i64::MAX);
    let mut notes = vec![
        format!("legacy_candidates={}", legacy_symbols.len()),
        format!("compiler_fact_candidates={candidate_count}"),
        format!("rank_delta={rank_delta:+}"),
    ];

    let mut generated_labels: Vec<String> = visible_symbols
        .iter()
        .filter(|symbol| symbol.source == VisibleSymbolSource::Generated)
        .map(|symbol| symbol.name.clone())
        .collect();
    generated_labels.sort();
    generated_labels.dedup();
    if !generated_labels.is_empty() {
        notes.push(format!("generated_labels={}", generated_labels.join(",")));
    }

    let mut dynamic_blockers: Vec<String> = visible_symbols
        .iter()
        .filter(|symbol| symbol.source == VisibleSymbolSource::DynamicUnknown)
        .map(|symbol| symbol.name.clone())
        .collect();
    dynamic_blockers.sort();
    dynamic_blockers.dedup();
    if !dynamic_blockers.is_empty() {
        notes.push(format!("dynamic_boundary_blockers={}", dynamic_blockers.join(",")));
    }

    notes
}

fn is_completion_candidate_source(source: &VisibleSymbolSource) -> bool {
    *source != VisibleSymbolSource::DynamicUnknown
}

fn completion_fact_source_traces(
    visible_symbols: &[VisibleSymbol],
    fallback_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    let mut traces = Vec::new();
    for symbol in visible_symbols {
        let (source, provenance, state) = completion_trace_shape(symbol, fallback_state);
        let anchor_id = symbol.context.as_ref().and_then(|context| {
            context.source_import_anchor_id.or(context.source_export_anchor_id)
        });
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Completion,
            source,
            provenance,
            symbol.confidence,
            ProviderFactFreshness::Fresh,
            state,
            None,
            anchor_id,
            Some(1),
        ));
    }

    if traces.is_empty() {
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Completion,
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

fn completion_trace_shape(
    symbol: &VisibleSymbol,
    fallback_state: ProviderFallbackState,
) -> (ProviderFactSourceKind, Provenance, ProviderFallbackState) {
    match symbol.source {
        VisibleSymbolSource::ExplicitImport
        | VisibleSymbolSource::DefaultExport
        | VisibleSymbolSource::ExportTag => (
            ProviderFactSourceKind::CompilerFact,
            Provenance::ImportExportInference,
            fallback_state,
        ),
        VisibleSymbolSource::Generated => (
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            fallback_state,
        ),
        VisibleSymbolSource::DynamicUnknown => (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            ProviderFallbackState::Blocked,
        ),
        VisibleSymbolSource::LocalLexical
        | VisibleSymbolSource::LocalPackage
        | VisibleSymbolSource::Constant => {
            (ProviderFactSourceKind::CompilerFact, Provenance::SemanticAnalyzer, fallback_state)
        }
        VisibleSymbolSource::External => (
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            ProviderFallbackState::Fallback,
        ),
    }
}

fn method_probe_names(
    legacy_methods: &[String],
    probe_methods: &[String],
    method_prefix: &str,
) -> Vec<String> {
    let mut names: Vec<String> = legacy_methods
        .iter()
        .chain(probe_methods.iter())
        .filter(|name| method_prefix.is_empty() || name.starts_with(method_prefix))
        .cloned()
        .collect();
    names.sort();
    names.dedup();
    names
}

fn method_names_for_prefix(methods: &[String], method_prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = methods
        .iter()
        .filter(|name| method_prefix.is_empty() || name.starts_with(method_prefix))
        .cloned()
        .collect();
    names.sort();
    names.dedup();
    names
}

fn method_candidates_to_summary(candidates: &[DefinitionCandidate]) -> ShadowResultSummary {
    let identities: Vec<String> = candidates.iter().map(method_candidate_identity).collect();
    summarize_identities(Some(identities))
}

fn method_candidate_identity(candidate: &DefinitionCandidate) -> String {
    if !candidate.display_name.is_empty() {
        return candidate.display_name.clone();
    }

    match candidate.canonical_name.rsplit("::").next() {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => candidate.canonical_name.clone(),
    }
}

fn method_completion_fact_source_traces(
    candidates: &[DefinitionCandidate],
    fallback_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    let mut traces = Vec::new();
    for candidate in candidates {
        let source = match candidate.kind {
            EntityKind::GeneratedMember => ProviderFactSourceKind::FrameworkAdapter,
            EntityKind::ExternalSymbol | EntityKind::Unknown => ProviderFactSourceKind::Fallback,
            _ => ProviderFactSourceKind::SemanticFact,
        };
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Completion,
            source,
            candidate.provenance,
            candidate.confidence,
            ProviderFactFreshness::Fresh,
            fallback_state,
            None,
            Some(candidate.anchor_id),
            Some(1),
        ));
    }

    if traces.is_empty() {
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Completion,
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

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason,
        EntityFact, EntityId, EntityKind, FileId, OccurrenceFact, Provenance, RenamePlan,
        SafeDeletePlan, ScopeId, UseLibFact, VisibleSymbol, VisibleSymbolContext,
        VisibleSymbolSource,
    };
    use perl_workspace::Parser;
    use perl_workspace::semantic::imports::ImportExportIndex;
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic::workspace_import_extractor::extract_use_lib_facts;
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;
    use perl_workspace::workspace::workspace_index::WorkspaceIndex;
    use std::collections::BTreeMap;

    // ── Minimal SemanticQueries stub for testing ──

    struct StubSemanticQueries {
        visible_result: Vec<VisibleSymbol>,
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
            self.visible_result.clone()
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

    struct MethodCandidateStub {
        candidates: BTreeMap<(String, String), Vec<DefinitionCandidate>>,
    }

    impl MethodCandidateStub {
        fn new(candidates: Vec<(&str, &str, Vec<DefinitionCandidate>)>) -> Self {
            let candidates = candidates
                .into_iter()
                .map(|(receiver, method, result)| {
                    ((receiver.to_string(), method.to_string()), result)
                })
                .collect();
            Self { candidates }
        }
    }

    impl SemanticQueries for MethodCandidateStub {
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
            receiver_package: &str,
            method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            self.candidates
                .get(&(receiver_package.to_string(), method_name.to_string()))
                .cloned()
                .unwrap_or_default()
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

    fn make_visible(
        name: &str,
        source: VisibleSymbolSource,
        confidence: Confidence,
    ) -> VisibleSymbol {
        VisibleSymbol {
            name: name.to_string(),
            entity_id: Some(EntityId(1)),
            source,
            confidence,
            context: None,
        }
    }

    fn make_visible_with_context(
        name: &str,
        source: VisibleSymbolSource,
        confidence: Confidence,
        module: &str,
    ) -> VisibleSymbol {
        VisibleSymbol {
            name: name.to_string(),
            entity_id: Some(EntityId(1)),
            source,
            confidence,
            context: Some(VisibleSymbolContext::new(Some(module.to_string()), None, None)),
        }
    }

    fn make_method_candidate(
        display_name: &str,
        canonical_name: &str,
        package: &str,
        kind: EntityKind,
        rank: DefinitionRank,
        rank_reason: DefinitionRankReason,
        entity_id: u64,
    ) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(entity_id),
            AnchorId(entity_id + 100),
            canonical_name.to_string(),
            display_name.to_string(),
            Some(package.to_string()),
            kind,
            Provenance::ExactAst,
            Confidence::High,
            rank,
            rank_reason,
        )
    }

    // ── Shadow mode tests ──

    #[test]
    fn completion_route_covers_use_lib_query_contract() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { visible_result: vec![] };
        assert!(queries.use_lib_paths(FileId(999)).is_empty());

        let file_id = FileId(7);
        let mut parser = Parser::new(
            "use lib 'lib'; use lib \"plain\"; use lib q{quoteop}; use lib q[bracket]; use lib q<angle>; use lib q!bang!; use lib qq{qqplain}; use lib qq{$dynamic}; use lib \"$dynamic\"; 1;",
        );
        let ast = parser.parse()?;
        let facts = extract_use_lib_facts(&ast, file_id);
        assert_eq!(facts.len(), 7);
        assert_eq!(facts[0].path, "lib");
        assert!(facts[0].is_active);
        assert_eq!(facts[1].path, "plain");
        assert_eq!(facts[1].provenance, Provenance::ExactAst);
        assert_eq!(facts[1].confidence, Confidence::High);
        assert_eq!(facts[2].path, "quoteop");
        assert_eq!(facts[3].path, "bracket");
        assert_eq!(facts[4].path, "angle");
        assert_eq!(facts[5].path, "bang");
        assert_eq!(facts[6].path, "qqplain");

        let mut no_lib_parser = Parser::new("no lib 'old'; 1;");
        let no_lib_ast = no_lib_parser.parse()?;
        let no_lib_facts = extract_use_lib_facts(&no_lib_ast, file_id);
        assert_eq!(no_lib_facts.len(), 1);
        assert_eq!(no_lib_facts[0].path, "old");
        assert!(!no_lib_facts[0].is_active);

        let mut qw_parser = Parser::new("use lib qw(old vendor); 1;");
        let qw_ast = qw_parser.parse()?;
        let qw_facts = extract_use_lib_facts(&qw_ast, file_id);
        assert_eq!(qw_facts.len(), 2);
        assert_eq!(qw_facts[0].path, "old");
        assert_eq!(qw_facts[1].path, "vendor");

        let mut import_index = ImportExportIndex::new();
        let manual_fact = UseLibFact::new(
            "manual/lib".to_string(),
            true,
            file_id,
            Some(AnchorId(42)),
            Provenance::ExactAst,
            Confidence::High,
        );
        import_index.add_file_use_lib("file:///manual.pl", file_id, vec![manual_fact.clone()]);
        let stored = import_index.get_use_lib_for_file(file_id);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0], manual_fact);
        import_index.remove_file_use_lib("file:///missing.pl");
        assert_eq!(import_index.get_use_lib_for_file(file_id).len(), 1);
        import_index.remove_file_use_lib("file:///manual.pl");
        assert!(import_index.get_use_lib_for_file(file_id).is_empty());

        let workspace = WorkspaceIndex::new();
        let uri = "file:///completion_use_lib_contract.pl";
        workspace
            .index_file_str(uri, "use lib 'first'; use lib \"plain\"; use lib $dynamic; 1;")?;
        let indexed = workspace
            .with_semantic_queries_for_uri(uri, |indexed_file_id, semantic_queries| {
                semantic_queries.use_lib_paths(indexed_file_id)
            })
            .ok_or("indexed file missing")?;
        assert_eq!(indexed.len(), 2);
        assert_eq!(indexed[0].path, "first");
        assert_eq!(indexed[1].path, "plain");

        workspace.index_file_str(uri, "no lib 'first'; use lib 'second'; 1;")?;
        let reindexed = workspace
            .with_semantic_queries_for_uri(uri, |indexed_file_id, semantic_queries| {
                semantic_queries.use_lib_paths(indexed_file_id)
            })
            .ok_or("reindexed file missing")?;
        assert_eq!(reindexed.len(), 2);
        assert_eq!(reindexed[0].path, "first");
        assert!(!reindexed[0].is_active);
        assert_eq!(reindexed[1].path, "second");
        assert!(reindexed[1].is_active);

        workspace.remove_file(uri);
        assert!(workspace.file_id_for_uri(uri).is_none());

        Ok(())
    }

    #[test]
    fn shadow_both_empty_yields_same() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { visible_result: vec![] };

        let result = completion_visibility_shadow(vec![], &queries, FileId(1), 0, None, "test");

        assert!(result.legacy_symbols.is_empty());
        assert_eq!(result.receipt.query, ShadowQueryName::CompletionVisibility);
        assert_eq!(result.receipt.old_result.available, true);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    #[test]
    fn shadow_new_path_has_symbols_old_empty() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("foo", VisibleSymbolSource::ExplicitImport, Confidence::High);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let result =
            completion_visibility_shadow(vec![], &queries, FileId(1), 10, None, "use Foo qw(foo)");

        assert!(result.legacy_symbols.is_empty());
        assert_eq!(result.receipt.old_result.available, true);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.available, true);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        Ok(())
    }

    #[test]
    fn shadow_returns_legacy_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { visible_result: vec![] };
        let legacy = vec!["bar".to_string(), "baz".to_string()];

        let result =
            completion_visibility_shadow(legacy.clone(), &queries, FileId(1), 0, None, "test");

        assert_eq!(result.legacy_symbols, legacy);
        assert_eq!(result.receipt.query, ShadowQueryName::CompletionVisibility);
        assert_eq!(result.receipt.input.symbol, "test");
        Ok(())
    }

    #[test]
    fn shadow_receipt_uses_completion_visibility_query_name()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { visible_result: vec![] };

        let result = completion_visibility_shadow(vec![], &queries, FileId(1), 0, None, "use Foo");

        assert_eq!(result.receipt.query, ShadowQueryName::CompletionVisibility);
        assert_eq!(result.receipt.input.symbol, "use Foo");
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    #[test]
    fn shadow_multiple_new_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let syms = vec![
            make_visible("a", VisibleSymbolSource::ExplicitImport, Confidence::High),
            make_visible("b", VisibleSymbolSource::DefaultExport, Confidence::High),
            make_visible("c", VisibleSymbolSource::ExportTag, Confidence::High),
        ];
        let queries = StubSemanticQueries { visible_result: syms };

        let result =
            completion_visibility_shadow(vec![], &queries, FileId(1), 0, None, "use Foo ':all'");

        assert_eq!(result.receipt.new_result.match_count, 3);
        assert_eq!(result.receipt.new_result.available, true);
        Ok(())
    }

    #[test]
    fn completion_compiler_shadow_traces_import_export_and_generated_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let syms = vec![
            make_visible("imported_func", VisibleSymbolSource::ExplicitImport, Confidence::High),
            make_visible("default_func", VisibleSymbolSource::DefaultExport, Confidence::High),
            make_visible("generated_accessor", VisibleSymbolSource::Generated, Confidence::Medium),
        ];
        let queries = StubSemanticQueries { visible_result: syms };

        let result = completion_visibility_shadow(
            vec!["legacy_func".to_string()],
            &queries,
            FileId(1),
            10,
            None,
            "completion shadow import/export",
        );

        assert_eq!(result.legacy_symbols, vec!["legacy_func".to_string()]);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert!(result.receipt.notes.iter().any(|note| note == "legacy_candidates=1"));
        assert!(result.receipt.notes.iter().any(|note| note == "compiler_fact_candidates=3"));
        assert!(result.receipt.notes.iter().any(|note| note == "rank_delta=+2"));
        assert!(
            result.receipt.notes.iter().any(|note| note == "generated_labels=generated_accessor"),
            "generated labels should be recorded in the shadow receipt notes"
        );

        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::CompilerFact
                && trace.provenance == Provenance::ImportExportInference
                && trace.confidence == Confidence::High
                && trace.freshness == ProviderFactFreshness::Fresh
                && trace.fallback_state == ProviderFallbackState::Shadow
        }));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::FrameworkAdapter
                && trace.provenance == Provenance::FrameworkSynthesis
                && trace.confidence == Confidence::Medium
                && trace.fallback_state == ProviderFallbackState::Shadow
        }));
        Ok(())
    }

    #[test]
    fn completion_compiler_shadow_records_constant_provider_fact_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries {
            visible_result: vec![make_visible(
                "HTTP_OK",
                VisibleSymbolSource::Constant,
                Confidence::High,
            )],
        };

        let result = completion_visibility_shadow(
            vec!["legacy_http_ok".to_string()],
            &queries,
            FileId(1),
            42,
            None,
            "use constant HTTP_OK",
        );

        assert_eq!(result.legacy_symbols, vec!["legacy_http_ok".to_string()]);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert!(
            result
                .receipt
                .new_result
                .identities
                .iter()
                .any(|identity| identity == "HTTP_OK:Constant"),
            "constant fact should be present in completion shadow receipt"
        );
        assert!(result.receipt.notes.iter().any(|note| note == "compiler_fact_candidates=1"));

        let trace = result
            .receipt
            .fact_source_traces
            .iter()
            .find(|trace| {
                trace.surface == ProviderSurface::Completion
                    && trace.source == ProviderFactSourceKind::CompilerFact
                    && trace.provenance == Provenance::SemanticAnalyzer
            })
            .ok_or("missing constant completion fact trace")?;
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn completion_compiler_shadow_labels_dynamic_boundary_blockers()
    -> Result<(), Box<dyn std::error::Error>> {
        let syms = vec![make_visible(
            "symbolic_ref_candidate",
            VisibleSymbolSource::DynamicUnknown,
            Confidence::Low,
        )];
        let queries = StubSemanticQueries { visible_result: syms };

        let result =
            completion_visibility_shadow(vec![], &queries, FileId(1), 0, None, "symbolic ref");

        assert_eq!(result.receipt.new_result.match_count, 0);
        assert!(result.receipt.notes.iter().any(|note| note == "compiler_fact_candidates=0"));
        assert!(
            result
                .receipt
                .notes
                .iter()
                .any(|note| note == "dynamic_boundary_blockers=symbolic_ref_candidate"),
            "dynamic boundary blockers should be labeled instead of ranked as ordinary completions"
        );
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::DynamicBoundary
                && trace.provenance == Provenance::DynamicBoundary
                && trace.confidence == Confidence::Low
                && trace.fallback_state == ProviderFallbackState::Blocked
        }));
        Ok(())
    }

    #[test]
    fn completion_compiler_shadow_records_fallback_trace_when_no_fact_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { visible_result: vec![] };

        let result = completion_visibility_shadow(
            vec!["legacy_func".to_string()],
            &queries,
            FileId(1),
            0,
            None,
            "legacy only",
        );

        assert_eq!(result.legacy_symbols, vec!["legacy_func".to_string()]);
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::Fallback
                && trace.provenance == Provenance::SearchFallback
                && trace.confidence == Confidence::Low
                && trace.freshness == ProviderFactFreshness::NotApplicable
                && trace.fallback_state == ProviderFallbackState::Fallback
        }));
        Ok(())
    }

    // ── Method shadow mode tests ──

    #[test]
    fn method_shadow_reports_same_when_semantic_matches_legacy()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = make_method_candidate(
            "bark",
            "Dog::bark",
            "Dog",
            EntityKind::Method,
            DefinitionRank::SamePackage,
            DefinitionRankReason::SamePackage,
            10,
        );
        let queries = MethodCandidateStub::new(vec![("Dog", "bark", vec![candidate])]);

        let result =
            method_completion_shadow(vec!["bark".to_string()], vec![], &queries, "Dog", "ba");

        assert_eq!(result.legacy_methods, vec!["bark".to_string()]);
        assert_eq!(result.probed_methods, vec!["bark".to_string()]);
        assert_eq!(result.receipt.query, ShadowQueryName::MethodCandidates);
        assert_eq!(result.receipt.input.symbol, "Dog->ba");
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    #[test]
    fn method_shadow_receipt_records_source_backed_receiver_fact_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = make_method_candidate(
            "bark",
            "Dog::bark",
            "Dog",
            EntityKind::Method,
            DefinitionRank::SamePackage,
            DefinitionRankReason::SamePackage,
            10,
        );
        let queries = MethodCandidateStub::new(vec![("Dog", "bark", vec![candidate])]);

        let result =
            method_completion_shadow(vec!["bark".to_string()], vec![], &queries, "Dog", "ba");

        assert!(result.receipt.notes.iter().any(|note| note == "receiver_package=Dog"));
        assert!(result.receipt.notes.iter().any(|note| note == "method_prefix=ba"));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::SemanticFact
                && trace.provenance == Provenance::ExactAst
                && trace.confidence == Confidence::High
                && trace.freshness == ProviderFactFreshness::Fresh
                && trace.fallback_state == ProviderFallbackState::Shadow
                && trace.anchor_id == Some(AnchorId(110))
        }));
        Ok(())
    }

    #[test]
    fn method_shadow_reports_improved_for_inherited_probe() -> Result<(), Box<dyn std::error::Error>>
    {
        let candidate = make_method_candidate(
            "speak",
            "Animal::speak",
            "Animal",
            EntityKind::Method,
            DefinitionRank::WorkspaceCandidate,
            DefinitionRankReason::WorkspaceSymbol,
            11,
        );
        let queries = MethodCandidateStub::new(vec![("Dog", "speak", vec![candidate])]);

        let result =
            method_completion_shadow(vec![], vec!["speak".to_string()], &queries, "Dog", "sp");

        assert_eq!(result.probed_methods, vec!["speak".to_string()]);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        Ok(())
    }

    #[test]
    fn method_shadow_reports_regression_when_semantic_missing_legacy()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = MethodCandidateStub::new(vec![]);

        let result =
            method_completion_shadow(vec!["name".to_string()], vec![], &queries, "Person", "");

        assert_eq!(result.probed_methods, vec!["name".to_string()]);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Regression);
        Ok(())
    }

    #[test]
    fn method_shadow_receipt_records_fallback_trace_when_receiver_fact_missing()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = MethodCandidateStub::new(vec![]);

        let result =
            method_completion_shadow(vec!["name".to_string()], vec![], &queries, "Person", "");

        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::Fallback
                && trace.provenance == Provenance::SearchFallback
                && trace.confidence == Confidence::Low
                && trace.freshness == ProviderFactFreshness::NotApplicable
                && trace.fallback_state == ProviderFallbackState::Fallback
                && trace.anchor_id.is_none()
        }));
        Ok(())
    }

    #[test]
    fn method_shadow_supports_generated_member_candidates() -> Result<(), Box<dyn std::error::Error>>
    {
        let candidate = make_method_candidate(
            "name",
            "Person::name",
            "Person",
            EntityKind::GeneratedMember,
            DefinitionRank::SamePackage,
            DefinitionRankReason::SamePackage,
            12,
        );
        let queries = MethodCandidateStub::new(vec![("Person", "name", vec![candidate])]);

        let result =
            method_completion_shadow(vec![], vec!["name".to_string()], &queries, "Person", "na");

        assert_eq!(result.receipt.new_result.identities, vec!["name".to_string()]);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.surface == ProviderSurface::Completion
                && trace.source == ProviderFactSourceKind::FrameworkAdapter
                && trace.provenance == Provenance::ExactAst
                && trace.confidence == Confidence::High
                && trace.fallback_state == ProviderFallbackState::Shadow
        }));
        Ok(())
    }

    #[test]
    fn method_shadow_probes_are_prefix_filtered_and_deduped()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = MethodCandidateStub::new(vec![]);

        let result = method_completion_shadow(
            vec!["name".to_string(), "name".to_string(), "clear".to_string()],
            vec!["notify".to_string(), "clear".to_string()],
            &queries,
            "Person",
            "n",
        );

        assert_eq!(result.probed_methods, vec!["name".to_string(), "notify".to_string()]);
        assert_eq!(result.receipt.old_result.identities, vec!["name".to_string()]);
        assert_eq!(result.receipt.old_result.match_count, 1);
        Ok(())
    }

    // ── Summary helper tests ──

    #[test]
    fn legacy_symbols_to_summary_empty() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_symbols_to_summary(&[]);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn legacy_symbols_to_summary_multiple() -> Result<(), Box<dyn std::error::Error>> {
        let symbols = vec!["foo".to_string(), "bar".to_string()];
        let summary = super::legacy_symbols_to_summary(&symbols);
        assert!(summary.available);
        assert_eq!(summary.match_count, 2);
        Ok(())
    }

    #[test]
    fn semantic_visible_to_summary_empty() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::semantic_visible_to_summary(&[]);
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_visible_to_summary_multiple() -> Result<(), Box<dyn std::error::Error>> {
        let syms = vec![
            make_visible("a", VisibleSymbolSource::ExplicitImport, Confidence::High),
            make_visible("b", VisibleSymbolSource::DefaultExport, Confidence::High),
        ];
        let summary = super::semantic_visible_to_summary(&syms);
        assert!(summary.available);
        assert_eq!(summary.match_count, 2);
        assert_eq!(summary.identities.len(), 2);
        Ok(())
    }

    #[test]
    fn method_candidates_to_summary_uses_display_name() -> Result<(), Box<dyn std::error::Error>> {
        let candidate = make_method_candidate(
            "bark",
            "Dog::bark",
            "Dog",
            EntityKind::Method,
            DefinitionRank::SamePackage,
            DefinitionRankReason::SamePackage,
            13,
        );

        let summary = super::method_candidates_to_summary(&[candidate]);

        assert_eq!(summary.identities, vec!["bark".to_string()]);
        assert_eq!(summary.match_count, 1);
        Ok(())
    }

    // ── Cutover tests ──

    #[test]
    fn cutover_semantic_with_explicit_import() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible_with_context(
            "foo",
            VisibleSymbolSource::ExplicitImport,
            Confidence::High,
            "Foo::Bar",
        );
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(
            vec!["foo".to_string()],
            &queries,
            FileId(1),
            10,
            None,
            "use Foo::Bar qw(foo)",
        );

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].symbol.name, "foo");
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        assert_eq!(outcome.receipt.query, ShadowQueryName::CompletionVisibility);
        Ok(())
    }

    #[test]
    fn cutover_semantic_with_default_export() -> Result<(), Box<dyn std::error::Error>> {
        let sym =
            make_visible("exported_sub", VisibleSymbolSource::DefaultExport, Confidence::High);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome =
            completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "use Foo");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].symbol.name, "exported_sub");
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_semantic_with_tag_export() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("tag_sym", VisibleSymbolSource::ExportTag, Confidence::High);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome =
            completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "use Foo ':all'");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].symbol.name, "tag_sym");
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_no_visible_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries { visible_result: vec![] };
        let legacy = vec!["fallback_sym".to_string()];

        let outcome =
            completion_visibility_cutover(legacy.clone(), &queries, FileId(1), 10, None, "test");

        match &outcome.result {
            CompletionCutoverResult::LegacyFallback(syms) => {
                assert_eq!(syms, &legacy);
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_dynamic_symbol_ranks_low() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("dyn_sym", VisibleSymbolSource::DynamicUnknown, Confidence::Low);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "eval");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].tier, CompletionRankTier::Low);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_low_confidence_ranks_low() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("low_conf", VisibleSymbolSource::ExplicitImport, Confidence::Low);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "test");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].tier, CompletionRankTier::Low);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_external_symbol_ranks_medium() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("ext_sym", VisibleSymbolSource::External, Confidence::High);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "test");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].tier, CompletionRankTier::Medium);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_mixed_tiers() -> Result<(), Box<dyn std::error::Error>> {
        let high = make_visible("a", VisibleSymbolSource::ExplicitImport, Confidence::High);
        let medium = make_visible("b", VisibleSymbolSource::External, Confidence::Medium);
        let low = make_visible("c", VisibleSymbolSource::DynamicUnknown, Confidence::Low);
        let queries = StubSemanticQueries { visible_result: vec![high, medium, low] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "mixed");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 3);
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
                assert_eq!(ranked[1].tier, CompletionRankTier::Medium);
                assert_eq!(ranked[2].tier, CompletionRankTier::Low);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_sorts_by_semantic_provenance() -> Result<(), Box<dyn std::error::Error>> {
        let symbols = vec![
            make_visible("dyn", VisibleSymbolSource::DynamicUnknown, Confidence::Low),
            make_visible("defaulted", VisibleSymbolSource::DefaultExport, Confidence::High),
            make_visible("imported", VisibleSymbolSource::ExplicitImport, Confidence::High),
            make_visible("tagged", VisibleSymbolSource::ExportTag, Confidence::High),
            make_visible("external", VisibleSymbolSource::External, Confidence::High),
            make_visible("accessor", VisibleSymbolSource::Generated, Confidence::Medium),
            make_visible("CONST", VisibleSymbolSource::Constant, Confidence::High),
            make_visible("same_package", VisibleSymbolSource::LocalPackage, Confidence::High),
            make_visible("$local", VisibleSymbolSource::LocalLexical, Confidence::High),
        ];
        let queries = StubSemanticQueries { visible_result: symbols };

        let outcome =
            completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "ranked");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                let names: Vec<&str> =
                    ranked.iter().map(|entry| entry.symbol.name.as_str()).collect();
                assert_eq!(
                    names,
                    vec![
                        "$local",
                        "same_package",
                        "CONST",
                        "imported",
                        "defaulted",
                        "tagged",
                        "accessor",
                        "external",
                        "dyn",
                    ]
                );
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_all_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let syms = vec![
            make_visible("a", VisibleSymbolSource::ExplicitImport, Confidence::High),
            make_visible("b", VisibleSymbolSource::DynamicUnknown, Confidence::Low),
        ];
        let queries = StubSemanticQueries { visible_result: syms };

        let outcome = completion_visibility_cutover(
            vec!["a".to_string()],
            &queries,
            FileId(1),
            10,
            None,
            "test",
        );

        // DynamicUnknown blockers are traced separately instead of counted as candidates.
        assert_eq!(outcome.receipt.new_result.match_count, 1);
        assert_eq!(outcome.receipt.query, ShadowQueryName::CompletionVisibility);
        Ok(())
    }

    #[test]
    fn cutover_local_lexical_ranks_high() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("$my_var", VisibleSymbolSource::LocalLexical, Confidence::High);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "test");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_constant_ranks_high() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("MAX_SIZE", VisibleSymbolSource::Constant, Confidence::High);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "test");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_generated_ranks_high() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("accessor", VisibleSymbolSource::Generated, Confidence::Medium);
        let queries = StubSemanticQueries { visible_result: vec![sym] };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "test");

        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 1);
                assert_eq!(ranked[0].tier, CompletionRankTier::High);
            }
            other => return Err(format!("expected Semantic, got {:?}", other).into()),
        }
        Ok(())
    }

    // ── rank_visible_symbol tests ──

    #[test]
    fn rank_explicit_import_high_confidence_is_high() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("foo", VisibleSymbolSource::ExplicitImport, Confidence::High);
        let ranked = rank_visible_symbol(sym);
        assert_eq!(ranked.tier, CompletionRankTier::High);
        Ok(())
    }

    #[test]
    fn rank_dynamic_unknown_is_low() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("dyn", VisibleSymbolSource::DynamicUnknown, Confidence::High);
        let ranked = rank_visible_symbol(sym);
        assert_eq!(ranked.tier, CompletionRankTier::Low);
        Ok(())
    }

    #[test]
    fn rank_any_source_low_confidence_is_low() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("low", VisibleSymbolSource::LocalPackage, Confidence::Low);
        let ranked = rank_visible_symbol(sym);
        assert_eq!(ranked.tier, CompletionRankTier::Low);
        Ok(())
    }

    #[test]
    fn rank_external_high_confidence_is_medium() -> Result<(), Box<dyn std::error::Error>> {
        let sym = make_visible("external", VisibleSymbolSource::External, Confidence::High);
        let ranked = rank_visible_symbol(sym);
        assert_eq!(ranked.tier, CompletionRankTier::Medium);
        Ok(())
    }

    #[test]
    fn rank_tier_ordering() -> Result<(), Box<dyn std::error::Error>> {
        assert!(CompletionRankTier::High < CompletionRankTier::Medium);
        assert!(CompletionRankTier::Medium < CompletionRankTier::Low);
        Ok(())
    }

    #[test]
    fn cutover_sorts_symbols_by_tier_source_confidence_then_name()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries = StubSemanticQueries {
            visible_result: vec![
                make_visible("zlex", VisibleSymbolSource::LocalLexical, Confidence::Medium),
                make_visible("bext", VisibleSymbolSource::External, Confidence::High),
                make_visible("adyn", VisibleSymbolSource::DynamicUnknown, Confidence::High),
                make_visible("alex", VisibleSymbolSource::LocalLexical, Confidence::High),
                make_visible("aext", VisibleSymbolSource::External, Confidence::High),
            ],
        };

        let outcome = completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "test");
        let CompletionCutoverResult::Semantic(ranked) = outcome.result else {
            return Err("expected semantic completion result".into());
        };

        let ordered_names: Vec<&str> = ranked.iter().map(|r| r.symbol.name.as_str()).collect();
        assert_eq!(ordered_names, vec!["alex", "zlex", "aext", "bext", "adyn"]);
        Ok(())
    }

    // ── Empty import suppresses defaults test ──

    #[test]
    fn cutover_empty_import_suppresses_defaults() -> Result<(), Box<dyn std::error::Error>> {
        // When `use Foo ()` is used, visible_symbols_at should return no
        // default exports. The semantic path returns empty → fallback.
        let queries = StubSemanticQueries { visible_result: vec![] };

        let outcome =
            completion_visibility_cutover(vec![], &queries, FileId(1), 10, None, "use Foo ()");

        match &outcome.result {
            CompletionCutoverResult::LegacyFallback(syms) => {
                assert!(syms.is_empty());
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }
}
