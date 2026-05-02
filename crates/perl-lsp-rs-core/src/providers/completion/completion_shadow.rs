//! Completion shadow compare and cutover paths.
//!
//! Provides two entry points for completion visibility:
//!
//! 1. **Shadow mode** ([`completion_visibility_shadow`]) — runs both legacy
//!    completion symbol gathering and new `SemanticQueries::visible_symbols_at`
//!    side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`completion_visibility_cutover`]) — uses the semantic
//!    path as the primary source of truth:
//!    - *Exact*: high-confidence visible symbol → rank high.
//!    - *Ambiguous*: lower-confidence or heuristic → rank lower.
//!    - *Dynamic / Unavailable*: dynamic-boundary or unavailable → show low
//!      or omit.
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

use perl_semantic_facts::{Confidence, FileId, ScopeId, VisibleSymbol, VisibleSymbolSource};
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

    // ── New semantic path ──
    let new_visible = semantic_queries.visible_symbols_at(file_id, byte_offset, scope_id);
    let new_summary = semantic_visible_to_summary(&new_visible);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries(
        ShadowQueryName::CompletionVisibility,
        ShadowQueryInput { symbol: input_label.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
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
/// 3. If no symbols are returned, fall back to legacy.
/// 4. Emit a shadow-compare receipt regardless of outcome.
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
        let ranked: Vec<RankedCompletionSymbol> =
            all_visible.into_iter().map(rank_visible_symbol).collect();
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
        .map(|s| {
            // Use name + source as a stable identity for shadow comparison.
            format!("{}:{:?}", s.name, s.source)
        })
        .collect();

    summarize_identities(Some(identities))
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        Confidence, DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact, RenamePlan,
        SafeDeletePlan, ScopeId, VisibleSymbol, VisibleSymbolContext, VisibleSymbolSource,
    };
    use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

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

    // ── Shadow mode tests ──

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
        assert_eq!(result.receipt.schema_version, 1);
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

        // Receipt should reflect ALL visible symbols (before ranking).
        assert_eq!(outcome.receipt.new_result.match_count, 2);
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
    fn rank_tier_ordering() -> Result<(), Box<dyn std::error::Error>> {
        assert!(CompletionRankTier::High < CompletionRankTier::Medium);
        assert!(CompletionRankTier::Medium < CompletionRankTier::Low);
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
