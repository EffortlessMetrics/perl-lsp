//! Hover shadow compare and cutover paths.
//!
//! Provides two entry points for hover:
//!
//! 1. **Shadow mode** ([`hover_shadow`]) — runs both legacy and semantic
//!    paths side-by-side, always returning the legacy result.
//!    Emits a [`SemanticShadowCompareReceipt`] for scorecard aggregation.
//!
//! 2. **Cutover mode** ([`hover_cutover`]) — uses the semantic path as
//!    the primary source of truth:
//!    - *Exact*: explain the symbol with entity info and origin metadata.
//!    - *Ambiguous*: explain the ambiguity (multiple candidates).
//!    - *Dynamic / Unavailable*: explain the dynamic boundary or return
//!      a legacy-fallback explanation.
//!
//! # Requirements
//!
//! - **Req 9.5**: Hover calls `SemanticQueries::symbol_at` for entity info.
//! - **Req 22.7**: Exact → explain symbol; Ambiguous → explain ambiguity;
//!   Dynamic/Unavailable → explain dynamic boundary.

use perl_semantic_facts::{
    AnchorId, Confidence, DefinitionCandidate, DefinitionRankReason, EntityFact, EntityKind,
    FileId, OccurrenceFact, OccurrenceKind, Provenance, ProviderFactFreshness,
    ProviderFactSourceKind, ProviderFactTrace, ProviderFallbackState, ProviderSurface, ScopeId,
    VisibleSymbol, VisibleSymbolSource,
};
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, ShadowResultSummary,
    summarize_identities,
};

/// Result of a shadow-compared hover request.
///
/// Contains the legacy hover text (which callers should use during the
/// shadow phase) and the shadow-compare receipt for scorecard aggregation.
#[derive(Debug)]
pub struct HoverShadowResult {
    /// Legacy hover text — callers should use this during the shadow phase.
    pub legacy_text: Option<String>,
    /// Shadow-compare receipt comparing old and new paths.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Run hover through both legacy and semantic paths, producing a
/// shadow-compare receipt.
///
/// # Arguments
///
/// * `legacy_hover_text` — the hover text produced by the legacy path
///   (caller is responsible for running the legacy hover logic).
/// * `semantic_queries` — the new semantic query facade.
/// * `symbol` — the symbol name at the hover position.
/// * `file_id` — the file containing the hover position.
/// * `byte_offset` — the byte offset of the hover position.
/// * `scope_id` — the scope enclosing the hover position, when known.
///
/// # Returns
///
/// A [`HoverShadowResult`] containing the legacy text and a receipt.
pub fn hover_shadow<Q: SemanticQueries>(
    legacy_hover_text: Option<String>,
    semantic_queries: &Q,
    symbol: &str,
    file_id: FileId,
    byte_offset: u32,
    scope_id: Option<ScopeId>,
) -> HoverShadowResult {
    // ── Legacy path summary ──
    let old_summary = legacy_hover_to_summary(legacy_hover_text.as_deref());

    // ── New semantic path ──
    let entity_occ = semantic_queries.symbol_at(file_id, byte_offset);
    let visible = semantic_queries.visible_symbols_at(file_id, byte_offset, scope_id);
    let new_summary = semantic_hover_to_summary(entity_occ.as_ref(), &visible, symbol);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::Hover,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
        hover_fact_source_traces(
            entity_occ.as_ref(),
            &visible,
            symbol,
            ProviderFallbackState::Shadow,
        ),
    );

    tracing::debug!(
        symbol = %symbol,
        verdict = ?receipt.verdict,
        old_count = receipt.old_result.match_count,
        new_count = receipt.new_result.match_count,
        "hover shadow compare"
    );

    HoverShadowResult { legacy_text: legacy_hover_text, receipt }
}

// ── Cutover types ──

/// Classification of the semantic hover result for cutover decisions.
///
/// Follows the fallback policy table (Req 22.7):
/// - Exact → explain symbol
/// - Ambiguous → explain ambiguity
/// - DynamicBoundary → explain dynamic boundary
/// - LegacyFallback → semantic path unavailable; use legacy hover text
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoverCutoverResult {
    /// Exact entity found — explain the symbol with origin metadata.
    Exact(HoverExplanation),
    /// Multiple candidates or ambiguous resolution — explain the ambiguity.
    Ambiguous(HoverExplanation),
    /// Symbol is at a dynamic boundary — explain the dynamic boundary.
    DynamicBoundary(HoverExplanation),
    /// Semantic path produced no usable result — fall back to legacy.
    LegacyFallback(Option<String>),
}

/// Structured hover explanation built from semantic query results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverExplanation {
    /// Markdown-formatted hover content.
    pub markdown: String,
}

/// Outcome of a cutover hover request.
///
/// Contains the classified result and a shadow-compare receipt for
/// scorecard tracking.
#[derive(Debug)]
pub struct HoverCutoverOutcome {
    /// The classified cutover result.
    pub result: HoverCutoverResult,
    /// Shadow-compare receipt for scorecard aggregation.
    pub receipt: SemanticShadowCompareReceipt,
}

// ── Cutover entry point ──

/// Run hover with the semantic path as primary, falling back to legacy
/// when the semantic result is unavailable.
///
/// # Decision logic
///
/// 1. Call `SemanticQueries::symbol_at` for the entity at the hover position.
/// 2. Call `SemanticQueries::visible_symbols_at` for origin metadata.
/// 3. Classify the result:
///    - **Exact**: entity found with high/medium confidence → explain symbol.
///    - **DynamicBoundary**: entity has dynamic-boundary provenance → explain boundary.
///    - **Ambiguous**: multiple visible symbols match → explain ambiguity.
///    - **Unavailable**: no entity found → fall back to legacy hover text.
/// 4. Emit a shadow-compare receipt regardless of outcome.
pub fn hover_cutover<Q: SemanticQueries>(
    legacy_hover_text: Option<String>,
    semantic_queries: &Q,
    symbol: &str,
    file_id: FileId,
    byte_offset: u32,
    scope_id: Option<ScopeId>,
) -> HoverCutoverOutcome {
    // ── Semantic path (primary) ──
    let entity_occ = semantic_queries.symbol_at(file_id, byte_offset);
    let visible = semantic_queries.visible_symbols_at(file_id, byte_offset, scope_id);
    let query_context = QueryContext::new(file_id, scope_id, Some(byte_offset));
    let definitions = semantic_queries.definitions(symbol, &query_context);
    let new_summary = semantic_hover_to_summary(entity_occ.as_ref(), &visible, symbol);

    // ── Legacy path (for fallback and receipt) ──
    let old_summary = legacy_hover_to_summary(legacy_hover_text.as_deref());

    // ── Classify result ──
    let result = classify_hover_result(
        entity_occ.clone(),
        &visible,
        &definitions,
        symbol,
        legacy_hover_text,
    );
    let fallback_state = hover_result_fallback_state(&result);

    // ── Build receipt ──
    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::Hover,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_summary,
        new_summary,
        Vec::new(),
        hover_fact_source_traces(entity_occ.as_ref(), &visible, symbol, fallback_state),
    );

    tracing::debug!(
        symbol = %symbol,
        verdict = ?receipt.verdict,
        classification = match &result {
            HoverCutoverResult::Exact(_) => "exact",
            HoverCutoverResult::Ambiguous(_) => "ambiguous",
            HoverCutoverResult::DynamicBoundary(_) => "dynamic_boundary",
            HoverCutoverResult::LegacyFallback(_) => "legacy_fallback",
        },
        "hover cutover"
    );

    HoverCutoverOutcome { result, receipt }
}

// ── Classification logic ──

/// Classify the semantic hover result into a cutover category.
fn classify_hover_result(
    entity_occ: Option<(EntityFact, OccurrenceFact)>,
    visible: &[VisibleSymbol],
    definitions: &[DefinitionCandidate],
    symbol: &str,
    legacy_text: Option<String>,
) -> HoverCutoverResult {
    match entity_occ {
        Some((ref entity, ref occurrence)) => {
            // Dynamic boundary: provenance is DynamicBoundary or occurrence kind marks one.
            if entity.provenance == Provenance::DynamicBoundary
                || matches!(
                    occurrence.kind,
                    OccurrenceKind::DynamicBoundary | OccurrenceKind::TypeglobReference
                )
            {
                let explanation = build_dynamic_boundary_explanation(entity, occurrence, symbol);
                return HoverCutoverResult::DynamicBoundary(explanation);
            }

            // Low confidence with no usable entity info → fall back
            if entity.confidence == Confidence::Low
                && occurrence.provenance == Provenance::NameHeuristic
            {
                return HoverCutoverResult::LegacyFallback(legacy_text);
            }

            // Check for ambiguity: multiple visible symbols with the same name
            let matching_visible: Vec<&VisibleSymbol> =
                visible.iter().filter(|v| v.name == symbol).collect();

            if matching_visible.len() > 1 {
                let explanation =
                    build_ambiguous_explanation(entity, &matching_visible, definitions, symbol);
                return HoverCutoverResult::Ambiguous(explanation);
            }

            // Exact: single entity with usable confidence
            let visible_match = matching_visible.first().copied();
            let explanation = build_exact_explanation(entity, occurrence, visible_match, symbol);
            HoverCutoverResult::Exact(explanation)
        }
        None => {
            // No entity at position — check if visible symbols can explain
            let matching_visible: Vec<&VisibleSymbol> =
                visible.iter().filter(|v| v.name == symbol).collect();

            if matching_visible.is_empty() {
                return HoverCutoverResult::LegacyFallback(legacy_text);
            }

            // Check for dynamic symbols
            let all_dynamic =
                matching_visible.iter().all(|v| v.source == VisibleSymbolSource::DynamicUnknown);
            if all_dynamic {
                let Some(vis) = matching_visible.first().copied() else {
                    return HoverCutoverResult::LegacyFallback(legacy_text);
                };
                let explanation = build_dynamic_visible_explanation(vis, symbol);
                return HoverCutoverResult::DynamicBoundary(explanation);
            }

            if matching_visible.len() > 1 {
                let explanation = build_ambiguous_visible_explanation(&matching_visible, symbol);
                return HoverCutoverResult::Ambiguous(explanation);
            }

            // Single visible symbol — build explanation from it
            let Some(vis) = matching_visible.first().copied() else {
                return HoverCutoverResult::LegacyFallback(legacy_text);
            };
            let explanation = build_visible_symbol_explanation(vis, symbol);
            HoverCutoverResult::Exact(explanation)
        }
    }
}

// ── Explanation builders ──

/// Build a hover explanation for an exact entity match.
fn build_exact_explanation(
    entity: &EntityFact,
    occurrence: &OccurrenceFact,
    visible: Option<&VisibleSymbol>,
    symbol: &str,
) -> HoverExplanation {
    let mut parts: Vec<String> = Vec::new();

    // Entity kind and name
    let kind_label = entity_kind_label(entity.kind);
    parts.push(format!("**{kind_label}** `{}`", entity.canonical_name));

    // Occurrence kind context
    let occ_label = occurrence_kind_label(occurrence.kind);
    if !occ_label.is_empty() {
        parts.push(format!("*{occ_label}*"));
    }

    // Origin metadata from VisibleSymbolContext
    if let Some(origin) = visible.and_then(visible_origin_phrase) {
        parts.push(origin);
    }

    if let Some(vis) = visible {
        let (_source, visible_provenance, _state) =
            hover_visible_trace_shape(vis, ProviderFallbackState::Primary);
        if visible_provenance != entity.provenance || vis.confidence != entity.confidence {
            parts.push(visible_fact_source_phrase(vis));
        }
    }

    // Provenance and confidence
    if entity.provenance != Provenance::ExactAst {
        let (source, provenance, _state) =
            hover_entity_trace_shape(entity, occurrence, ProviderFallbackState::Primary);
        parts.push(fact_source_phrase(
            source,
            provenance,
            entity.confidence,
            ProviderFactFreshness::Fresh,
        ));
    }

    let _ = symbol; // used for matching; canonical_name is preferred for display
    HoverExplanation { markdown: parts.join("\n\n") }
}

/// Build a hover explanation for an ambiguous result (multiple candidates).
fn build_ambiguous_explanation(
    entity: &EntityFact,
    matching: &[&VisibleSymbol],
    definitions: &[DefinitionCandidate],
    symbol: &str,
) -> HoverExplanation {
    let mut parts: Vec<String> = Vec::new();

    let candidate_count = matching.len().max(definitions.len());
    parts.push(format!("**Ambiguous symbol** `{symbol}` — {candidate_count} candidates"));

    // Show the primary entity
    let kind_label = entity_kind_label(entity.kind);
    parts.push(format!("Primary: **{kind_label}** `{}`", entity.canonical_name));

    // List candidate sources
    for vis in matching {
        let source_label = visible_source_label(&vis.source);
        let module_info =
            vis.context.as_ref().and_then(|c| c.source_module.as_deref()).unwrap_or("unknown");
        parts.push(format!(
            "- `{}` via {source_label} from `{module_info}` ({})",
            vis.name,
            visible_fact_source_phrase(vis)
        ));
    }

    if !definitions.is_empty() {
        parts.push("Definition candidates:".to_string());
        for candidate in definitions.iter().take(5) {
            parts.push(format!(
                "- `{}` ({}, {})",
                candidate.canonical_name,
                entity_kind_label(candidate.kind),
                definition_rank_reason_label(&candidate.rank_reason)
            ));
        }
    }

    HoverExplanation { markdown: parts.join("\n\n") }
}

/// Build a hover explanation for a dynamic boundary entity.
fn build_dynamic_boundary_explanation(
    entity: &EntityFact,
    _occurrence: &OccurrenceFact,
    symbol: &str,
) -> HoverExplanation {
    let kind_label = entity_kind_label(entity.kind);
    let source = fact_source_phrase(
        ProviderFactSourceKind::DynamicBoundary,
        Provenance::DynamicBoundary,
        entity.confidence,
        ProviderFactFreshness::Fresh,
    );
    let markdown = format!(
        "**{kind_label}** `{symbol}`\n\n\
         ⚠️ Dynamic boundary — this symbol crosses a dynamic Perl construct \
         (string eval, symbolic dereference, AUTOLOAD, or runtime require). \
         Static analysis cannot fully resolve this reference.\n\n\
         {source}"
    );
    HoverExplanation { markdown }
}

/// Build a hover explanation when only visible symbols are available (no entity).
fn build_visible_symbol_explanation(vis: &VisibleSymbol, symbol: &str) -> HoverExplanation {
    let mut parts: Vec<String> = Vec::new();

    let source_label = visible_source_label(&vis.source);
    parts.push(format!("**Symbol** `{symbol}` ({source_label})"));

    if let Some(origin) = visible_origin_phrase(vis) {
        parts.push(origin);
    }

    parts.push(visible_fact_source_phrase(vis));

    HoverExplanation { markdown: parts.join("\n\n") }
}

/// Build a hover explanation for ambiguous visible symbols (no entity).
fn build_ambiguous_visible_explanation(
    matching: &[&VisibleSymbol],
    symbol: &str,
) -> HoverExplanation {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("**Ambiguous symbol** `{symbol}` — {} candidates", matching.len()));

    for vis in matching {
        let source_label = visible_source_label(&vis.source);
        let module_info =
            vis.context.as_ref().and_then(|c| c.source_module.as_deref()).unwrap_or("unknown");
        parts.push(format!("- `{}` via {source_label} from `{module_info}`", vis.name));
    }

    HoverExplanation { markdown: parts.join("\n\n") }
}

/// Build a hover explanation for dynamic-only visible symbols.
fn build_dynamic_visible_explanation(vis: &VisibleSymbol, symbol: &str) -> HoverExplanation {
    let source = visible_fact_source_phrase(vis);
    let markdown = format!(
        "**Symbol** `{symbol}`\n\n\
         ⚠️ Dynamic boundary — this symbol originates from a dynamic source. \
         Static analysis cannot determine its definition.\n\n\
         {source}"
    );
    HoverExplanation { markdown }
}

// ── Label helpers ──

/// Human-readable label for an [`EntityKind`].
fn entity_kind_label(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Package => "Package",
        EntityKind::Class => "Class",
        EntityKind::Role => "Role",
        EntityKind::Subroutine => "Subroutine",
        EntityKind::Method => "Method",
        EntityKind::Variable => "Variable",
        EntityKind::Constant => "Constant",
        EntityKind::Field => "Field",
        EntityKind::Label => "Label",
        EntityKind::Format => "Format",
        EntityKind::Module => "Module",
        EntityKind::GeneratedMember => "Generated member",
        EntityKind::ExternalSymbol => "External symbol",
        EntityKind::Unknown => "Symbol",
    }
}

/// Human-readable label for an [`OccurrenceKind`].
fn occurrence_kind_label(kind: OccurrenceKind) -> &'static str {
    match kind {
        OccurrenceKind::Definition => "definition",
        OccurrenceKind::Reference => "reference",
        OccurrenceKind::Read => "read",
        OccurrenceKind::Write => "write",
        OccurrenceKind::Call => "call",
        OccurrenceKind::MethodCall => "method call",
        OccurrenceKind::StaticMethodCall => "static method call",
        OccurrenceKind::CoderefReference => "coderef reference",
        OccurrenceKind::TypeglobReference => "typeglob reference",
        OccurrenceKind::Import => "import",
        OccurrenceKind::Export => "export",
        OccurrenceKind::Inheritance => "inheritance",
        OccurrenceKind::RoleComposition => "role composition",
        OccurrenceKind::GeneratedUse => "generated use",
        OccurrenceKind::DynamicBoundary => "dynamic boundary",
    }
}

/// Human-readable label for a [`Provenance`].
fn provenance_label(prov: Provenance) -> &'static str {
    match prov {
        Provenance::ExactAst => "exact AST",
        Provenance::DesugaredAst => "desugared AST",
        Provenance::SemanticAnalyzer => "semantic analyzer",
        Provenance::FrameworkSynthesis => "framework synthesis",
        Provenance::ImportExportInference => "import/export inference",
        Provenance::PragmaInference => "pragma inference",
        Provenance::NameHeuristic => "name heuristic",
        Provenance::SearchFallback => "search fallback",
        Provenance::DynamicBoundary => "dynamic boundary",
        Provenance::LiteralRequireImport => "literal require/import",
    }
}

/// Human-readable label for a [`Confidence`].
fn confidence_label(conf: Confidence) -> &'static str {
    match conf {
        Confidence::High => "high confidence",
        Confidence::Medium => "medium confidence",
        Confidence::Low => "low confidence",
    }
}

/// Human-readable label for a [`ProviderFactSourceKind`].
fn provider_source_label(source: ProviderFactSourceKind) -> &'static str {
    match source {
        ProviderFactSourceKind::ParserSyntax => "parser syntax",
        ProviderFactSourceKind::SemanticFact => "semantic fact",
        ProviderFactSourceKind::CompilerFact => "compiler fact",
        ProviderFactSourceKind::FrameworkAdapter => "framework adapter",
        ProviderFactSourceKind::DynamicBoundary => "dynamic boundary",
        ProviderFactSourceKind::Fallback => "fallback",
        ProviderFactSourceKind::Unknown => "unknown",
        _ => "unknown",
    }
}

/// Human-readable label for a [`ProviderFactFreshness`].
fn freshness_label(freshness: ProviderFactFreshness) -> &'static str {
    match freshness {
        ProviderFactFreshness::Fresh => "fresh",
        ProviderFactFreshness::Stale => "stale",
        ProviderFactFreshness::NotApplicable => "not applicable",
        _ => "unknown freshness",
    }
}

fn fact_source_phrase(
    source: ProviderFactSourceKind,
    provenance: Provenance,
    confidence: Confidence,
    freshness: ProviderFactFreshness,
) -> String {
    format!(
        "Source: {} / {} ({}, {})",
        provider_source_label(source),
        provenance_label(provenance),
        confidence_label(confidence),
        freshness_label(freshness)
    )
}

/// Human-readable label for a [`VisibleSymbolSource`].
fn visible_source_label(source: &VisibleSymbolSource) -> &'static str {
    match source {
        VisibleSymbolSource::LocalLexical => "local lexical",
        VisibleSymbolSource::LocalPackage => "local package",
        VisibleSymbolSource::ExplicitImport => "explicit import",
        VisibleSymbolSource::DefaultExport => "default export",
        VisibleSymbolSource::ExportTag => "export tag",
        VisibleSymbolSource::Constant => "constant",
        VisibleSymbolSource::Generated => "generated",
        VisibleSymbolSource::External => "external",
        VisibleSymbolSource::DynamicUnknown => "dynamic",
    }
}

fn visible_origin_phrase(vis: &VisibleSymbol) -> Option<String> {
    let module = vis.context.as_ref().and_then(|ctx| ctx.source_module.as_deref())?;
    let phrase = match vis.source {
        VisibleSymbolSource::ExplicitImport => {
            format!("Imported from `{module}` via explicit import list")
        }
        VisibleSymbolSource::DefaultExport => format!("Default export from `{module}`"),
        VisibleSymbolSource::ExportTag => format!("Export tag from `{module}`"),
        VisibleSymbolSource::Generated => format!("Generated by `{module}`"),
        VisibleSymbolSource::External => format!("External symbol from `{module}`"),
        VisibleSymbolSource::LocalLexical
        | VisibleSymbolSource::LocalPackage
        | VisibleSymbolSource::Constant
        | VisibleSymbolSource::DynamicUnknown => {
            format!("Origin: `{module}` via {}", visible_source_label(&vis.source))
        }
    };
    Some(phrase)
}

fn visible_fact_source_phrase(vis: &VisibleSymbol) -> String {
    let (source, provenance, _state) =
        hover_visible_trace_shape(vis, ProviderFallbackState::Primary);
    fact_source_phrase(source, provenance, vis.confidence, ProviderFactFreshness::Fresh)
}

fn hover_result_fallback_state(result: &HoverCutoverResult) -> ProviderFallbackState {
    match result {
        HoverCutoverResult::Exact(_) => ProviderFallbackState::Primary,
        HoverCutoverResult::Ambiguous(_) | HoverCutoverResult::LegacyFallback(_) => {
            ProviderFallbackState::Fallback
        }
        HoverCutoverResult::DynamicBoundary(_) => ProviderFallbackState::Blocked,
    }
}

fn hover_fact_source_traces(
    entity_occ: Option<&(EntityFact, OccurrenceFact)>,
    visible: &[VisibleSymbol],
    symbol: &str,
    fallback_state: ProviderFallbackState,
) -> Vec<ProviderFactTrace> {
    let mut traces = Vec::new();

    if let Some((entity, occurrence)) = entity_occ {
        let (source, provenance, state) =
            hover_entity_trace_shape(entity, occurrence, fallback_state);
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Hover,
            source,
            provenance,
            entity.confidence,
            ProviderFactFreshness::Fresh,
            state,
            None,
            entity.anchor_id,
            Some(1),
        ));
    }

    for symbol_fact in visible.iter().filter(|visible_symbol| visible_symbol.name == symbol) {
        let (source, provenance, state) = hover_visible_trace_shape(symbol_fact, fallback_state);
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Hover,
            source,
            provenance,
            symbol_fact.confidence,
            ProviderFactFreshness::Fresh,
            state,
            None,
            hover_visible_anchor(symbol_fact),
            Some(1),
        ));
    }

    if traces.is_empty() {
        traces.push(ProviderFactTrace::new(
            ProviderSurface::Hover,
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::NotApplicable,
            fallback_state,
            None,
            None,
            Some(1),
        ));
    }

    traces
}

fn hover_entity_trace_shape(
    entity: &EntityFact,
    occurrence: &OccurrenceFact,
    fallback_state: ProviderFallbackState,
) -> (ProviderFactSourceKind, Provenance, ProviderFallbackState) {
    if entity.provenance == Provenance::DynamicBoundary
        || occurrence.provenance == Provenance::DynamicBoundary
        || matches!(
            occurrence.kind,
            OccurrenceKind::DynamicBoundary | OccurrenceKind::TypeglobReference
        )
    {
        return (
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            ProviderFallbackState::Blocked,
        );
    }

    match entity.provenance {
        Provenance::FrameworkSynthesis => (
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            fallback_state,
        ),
        Provenance::ImportExportInference => (
            ProviderFactSourceKind::CompilerFact,
            Provenance::ImportExportInference,
            fallback_state,
        ),
        Provenance::NameHeuristic | Provenance::SearchFallback => {
            (ProviderFactSourceKind::Fallback, entity.provenance, ProviderFallbackState::Fallback)
        }
        _ => (ProviderFactSourceKind::SemanticFact, entity.provenance, fallback_state),
    }
}

fn hover_visible_trace_shape(
    visible: &VisibleSymbol,
    fallback_state: ProviderFallbackState,
) -> (ProviderFactSourceKind, Provenance, ProviderFallbackState) {
    match visible.source {
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
        VisibleSymbolSource::External => (
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            ProviderFallbackState::Fallback,
        ),
        VisibleSymbolSource::LocalLexical
        | VisibleSymbolSource::LocalPackage
        | VisibleSymbolSource::Constant => {
            (ProviderFactSourceKind::SemanticFact, Provenance::SemanticAnalyzer, fallback_state)
        }
    }
}

fn hover_visible_anchor(visible: &VisibleSymbol) -> Option<AnchorId> {
    visible
        .context
        .as_ref()
        .and_then(|context| context.source_import_anchor_id.or(context.source_export_anchor_id))
}

fn definition_rank_reason_label(reason: &DefinitionRankReason) -> String {
    match reason {
        DefinitionRankReason::ExactQualifiedName => "ranked by exact qualified name".to_string(),
        DefinitionRankReason::SamePackage => "ranked by same package".to_string(),
        DefinitionRankReason::ExplicitImport { module } if module.is_empty() => {
            "ranked by explicit import".to_string()
        }
        DefinitionRankReason::ExplicitImport { module } => {
            format!("ranked by explicit import from `{module}`")
        }
        DefinitionRankReason::DefaultExport { module } if module.is_empty() => {
            "ranked by default export".to_string()
        }
        DefinitionRankReason::DefaultExport { module } => {
            format!("ranked by default export from `{module}`")
        }
        DefinitionRankReason::WorkspaceSymbol => "ranked as workspace symbol".to_string(),
        DefinitionRankReason::HeuristicNameMatch => "ranked by heuristic name match".to_string(),
        _ => "ranked by semantic query".to_string(),
    }
}

// ── Summary helpers ──

/// Convert legacy hover text into a [`ShadowResultSummary`].
fn legacy_hover_to_summary(text: Option<&str>) -> ShadowResultSummary {
    match text {
        Some(t) if !t.is_empty() => summarize_identities(Some(vec![format!("hover:{}", t.len())])),
        Some(_) => summarize_identities(Some(Vec::new())),
        None => summarize_identities(None),
    }
}

/// Convert semantic hover results into a [`ShadowResultSummary`].
fn semantic_hover_to_summary(
    entity_occ: Option<&(EntityFact, OccurrenceFact)>,
    visible: &[VisibleSymbol],
    symbol: &str,
) -> ShadowResultSummary {
    match entity_occ {
        Some((entity, _occ)) => {
            let identity = format!("entity:{}:{}", entity.canonical_name, entity.id.0);
            summarize_identities(Some(vec![identity]))
        }
        None => {
            // No entity — check visible symbols
            let matching: Vec<String> = visible
                .iter()
                .filter(|v| v.name == symbol)
                .map(|v| format!("visible:{}:{:?}", v.name, v.source))
                .collect();
            if matching.is_empty() {
                summarize_identities(Some(Vec::new()))
            } else {
                summarize_identities(Some(matching))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason,
        EntityFact, EntityId, EntityKind, FileId, OccurrenceFact, OccurrenceId, OccurrenceKind,
        Provenance, ProviderFactFreshness, ProviderFactSourceKind, ProviderFallbackState,
        ProviderSurface, RenamePlan, SafeDeletePlan, ScopeId, VisibleSymbol, VisibleSymbolContext,
        VisibleSymbolSource,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    // ── Minimal SemanticQueries stub for testing ──

    struct StubSemanticQueries {
        symbol_at_result: Option<(EntityFact, OccurrenceFact)>,
        visible_symbols_result: Vec<VisibleSymbol>,
    }

    impl SemanticQueries for StubSemanticQueries {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            self.symbol_at_result.clone()
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
            self.visible_symbols_result.clone()
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

    struct RankedDefinitionStub {
        symbol_at_result: Option<(EntityFact, OccurrenceFact)>,
        visible_symbols_result: Vec<VisibleSymbol>,
        definitions_result: Vec<DefinitionCandidate>,
    }

    impl SemanticQueries for RankedDefinitionStub {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            self.symbol_at_result.clone()
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
            self.visible_symbols_result.clone()
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

    fn make_entity(
        name: &str,
        kind: EntityKind,
        provenance: Provenance,
        confidence: Confidence,
    ) -> EntityFact {
        EntityFact {
            id: EntityId(1),
            kind,
            canonical_name: name.to_string(),
            anchor_id: Some(AnchorId(10)),
            scope_id: None,
            provenance,
            confidence,
        }
    }

    fn make_occurrence(
        kind: OccurrenceKind,
        provenance: Provenance,
        confidence: Confidence,
    ) -> OccurrenceFact {
        OccurrenceFact {
            id: OccurrenceId(1),
            kind,
            entity_id: Some(EntityId(1)),
            anchor_id: AnchorId(10),
            scope_id: None,
            provenance,
            confidence,
        }
    }

    fn make_visible(
        name: &str,
        source: VisibleSymbolSource,
        confidence: Confidence,
        context: Option<VisibleSymbolContext>,
    ) -> VisibleSymbol {
        VisibleSymbol {
            name: name.to_string(),
            entity_id: Some(EntityId(1)),
            source,
            confidence,
            context,
        }
    }

    fn make_definition_candidate(
        id: u64,
        canonical_name: &str,
        package: &str,
        rank: DefinitionRank,
        rank_reason: DefinitionRankReason,
    ) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(id),
            AnchorId(1_000 + id),
            canonical_name.to_string(),
            canonical_name.rsplit("::").next().unwrap_or(canonical_name).to_string(),
            Some(package.to_string()),
            EntityKind::Subroutine,
            Provenance::ExactAst,
            Confidence::High,
            rank,
            rank_reason,
        )
    }

    fn assert_single_hover_trace(
        outcome: &HoverCutoverOutcome,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        fallback_state: ProviderFallbackState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let traces = outcome.receipt.fact_source_traces.as_slice();
        let [trace] = traces else {
            return Err(format!("expected one hover fact trace, got {}", traces.len()).into());
        };

        assert_eq!(trace.surface, ProviderSurface::Hover);
        assert_eq!(trace.source, source);
        assert_eq!(trace.provenance, provenance);
        assert_eq!(trace.confidence, confidence);
        assert_eq!(trace.freshness, freshness);
        assert_eq!(trace.fallback_state, fallback_state);
        Ok(())
    }

    // ── Shadow mode tests ──

    #[test]
    fn shadow_both_unavailable_yields_unavailable() -> Result<(), Box<dyn std::error::Error>> {
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![] };

        let result = hover_shadow(None, &queries, "unknown", FileId(1), 0, None);

        assert!(result.legacy_text.is_none());
        assert_eq!(result.receipt.query, ShadowQueryName::Hover);
        assert!(!result.receipt.old_result.available);
        assert!(result.receipt.new_result.available);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    #[test]
    fn shadow_legacy_has_text_new_empty() -> Result<(), Box<dyn std::error::Error>> {
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![] };

        let result =
            hover_shadow(Some("sub foo { ... }".to_string()), &queries, "foo", FileId(1), 10, None);

        assert_eq!(result.legacy_text.as_deref(), Some("sub foo { ... }"));
        assert_eq!(result.receipt.query, ShadowQueryName::Hover);
        assert!(result.receipt.old_result.available);
        assert_eq!(result.receipt.old_result.match_count, 1);
        Ok(())
    }

    #[test]
    fn shadow_new_path_has_entity_old_unavailable() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("Foo::bar", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High);
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let result = hover_shadow(None, &queries, "Foo::bar", FileId(1), 10, None);

        assert!(result.legacy_text.is_none());
        assert!(!result.receipt.old_result.available);
        assert!(result.receipt.new_result.available);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    #[test]
    fn shadow_returns_legacy_text() -> Result<(), Box<dyn std::error::Error>> {
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![] };

        let result =
            hover_shadow(Some("legacy hover".to_string()), &queries, "test", FileId(1), 0, None);

        assert_eq!(result.legacy_text.as_deref(), Some("legacy hover"));
        assert_eq!(result.receipt.input.symbol, "test");
        assert_eq!(
            result.receipt.schema_version,
            perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION
        );
        Ok(())
    }

    #[test]
    fn shadow_receipt_uses_hover_query_name() -> Result<(), Box<dyn std::error::Error>> {
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![] };

        let result = hover_shadow(None, &queries, "test", FileId(1), 0, None);

        assert_eq!(result.receipt.query, ShadowQueryName::Hover);
        assert_eq!(result.receipt.input.symbol, "test");
        Ok(())
    }

    // ── Cutover tests ──

    #[test]
    fn cutover_exact_entity_explains_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("Foo::bar", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High);
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let outcome = hover_cutover(None, &queries, "Foo::bar", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("Subroutine"));
                assert!(explanation.markdown.contains("Foo::bar"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        assert_eq!(outcome.receipt.query, ShadowQueryName::Hover);
        Ok(())
    }

    #[test]
    fn cutover_exact_with_import_context() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("bar", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High);
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let ctx =
            VisibleSymbolContext::new(Some("Foo::Module".to_string()), Some(AnchorId(20)), None);
        let vis =
            make_visible("bar", VisibleSymbolSource::ExplicitImport, Confidence::High, Some(ctx));
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![vis],
        };

        let outcome = hover_cutover(None, &queries, "bar", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("Subroutine"));
                assert!(explanation.markdown.contains("bar"));
                assert!(explanation.markdown.contains("Foo::Module"));
                assert!(explanation.markdown.contains("via explicit import list"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn hover_compiler_provenance_traces_import_primary_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = VisibleSymbolContext::new(Some("Foo::Module".to_string()), None, None);
        let vis =
            make_visible("bar", VisibleSymbolSource::ExplicitImport, Confidence::High, Some(ctx));
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![vis] };

        let outcome = hover_cutover(None, &queries, "bar", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("Imported from `Foo::Module`"));
                assert!(explanation.markdown.contains(
                    "Source: compiler fact / import/export inference (high confidence, fresh)"
                ));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        assert_single_hover_trace(
            &outcome,
            ProviderFactSourceKind::CompilerFact,
            Provenance::ImportExportInference,
            Confidence::High,
            ProviderFactFreshness::Fresh,
            ProviderFallbackState::Primary,
        )
    }

    #[test]
    fn hover_compiler_provenance_avoids_duplicate_source_labels()
    -> Result<(), Box<dyn std::error::Error>> {
        let entity = make_entity(
            "bar",
            EntityKind::Subroutine,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let occ = make_occurrence(
            OccurrenceKind::Call,
            Provenance::ImportExportInference,
            Confidence::High,
        );
        let ctx = VisibleSymbolContext::new(Some("Foo::Module".to_string()), None, None);
        let vis =
            make_visible("bar", VisibleSymbolSource::ExplicitImport, Confidence::High, Some(ctx));
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![vis],
        };

        let outcome = hover_cutover(None, &queries, "bar", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert_eq!(explanation.markdown.matches("Source:").count(), 1);
                assert!(explanation.markdown.contains(
                    "Source: compiler fact / import/export inference (high confidence, fresh)"
                ));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_dynamic_boundary_explains_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let entity = make_entity(
            "dyn_sub",
            EntityKind::Subroutine,
            Provenance::DynamicBoundary,
            Confidence::Low,
        );
        let occ = make_occurrence(
            OccurrenceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::Low,
        );
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let outcome = hover_cutover(None, &queries, "dyn_sub", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::DynamicBoundary(explanation) => {
                assert!(explanation.markdown.contains("Dynamic boundary"));
                assert!(explanation.markdown.contains("dyn_sub"));
            }
            other => return Err(format!("expected DynamicBoundary, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn hover_compiler_provenance_traces_dynamic_boundary_blocked_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let vis =
            make_visible("dyn_sym", VisibleSymbolSource::DynamicUnknown, Confidence::Low, None);
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![vis] };

        let outcome = hover_cutover(None, &queries, "dyn_sym", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::DynamicBoundary(explanation) => {
                assert!(explanation.markdown.contains("Dynamic boundary"));
                assert!(explanation.markdown.contains("dyn_sym"));
            }
            other => return Err(format!("expected DynamicBoundary, got {:?}", other).into()),
        }
        assert_single_hover_trace(
            &outcome,
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::Low,
            ProviderFactFreshness::Fresh,
            ProviderFallbackState::Blocked,
        )
    }

    #[test]
    fn cutover_ambiguous_multiple_visible_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("bar", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High);
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let vis1 = make_visible(
            "bar",
            VisibleSymbolSource::ExplicitImport,
            Confidence::High,
            Some(VisibleSymbolContext::new(Some("Foo".to_string()), None, None)),
        );
        let vis2 = make_visible(
            "bar",
            VisibleSymbolSource::DefaultExport,
            Confidence::Medium,
            Some(VisibleSymbolContext::new(Some("Baz".to_string()), None, None)),
        );
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![vis1, vis2],
        };

        let outcome = hover_cutover(None, &queries, "bar", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Ambiguous(explanation) => {
                assert!(explanation.markdown.contains("Ambiguous"));
                assert!(explanation.markdown.contains("2 candidates"));
                assert!(explanation.markdown.contains("Foo"));
                assert!(explanation.markdown.contains("Baz"));
            }
            other => return Err(format!("expected Ambiguous, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_ambiguous_definitions_explain_rank_reasons() -> Result<(), Box<dyn std::error::Error>>
    {
        let entity = make_entity(
            "Local::foo",
            EntityKind::Subroutine,
            Provenance::ExactAst,
            Confidence::High,
        );
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let queries = RankedDefinitionStub {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![
                make_visible(
                    "foo",
                    VisibleSymbolSource::LocalPackage,
                    Confidence::High,
                    Some(VisibleSymbolContext::new(Some("Local".to_string()), None, None)),
                ),
                make_visible(
                    "foo",
                    VisibleSymbolSource::ExplicitImport,
                    Confidence::High,
                    Some(VisibleSymbolContext::new(Some("Util".to_string()), None, None)),
                ),
            ],
            definitions_result: vec![
                make_definition_candidate(
                    11,
                    "Local::foo",
                    "Local",
                    DefinitionRank::SamePackage,
                    DefinitionRankReason::SamePackage,
                ),
                make_definition_candidate(
                    12,
                    "Util::foo",
                    "Util",
                    DefinitionRank::ExplicitImport,
                    DefinitionRankReason::ExplicitImport { module: "Util".to_string() },
                ),
            ],
        };

        let outcome = hover_cutover(None, &queries, "foo", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Ambiguous(explanation) => {
                assert!(explanation.markdown.contains("Definition candidates:"));
                assert!(explanation.markdown.contains("ranked by same package"));
                assert!(explanation.markdown.contains("ranked by explicit import from `Util`"));
            }
            other => return Err(format!("expected Ambiguous, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_fallback_when_no_entity() -> Result<(), Box<dyn std::error::Error>> {
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![] };

        let outcome = hover_cutover(
            Some("legacy hover".to_string()),
            &queries,
            "unknown",
            FileId(1),
            10,
            None,
        );

        match &outcome.result {
            HoverCutoverResult::LegacyFallback(text) => {
                assert_eq!(text.as_deref(), Some("legacy hover"));
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn hover_compiler_provenance_falls_back_when_no_compiler_fact()
    -> Result<(), Box<dyn std::error::Error>> {
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![] };

        let outcome = hover_cutover(
            Some("legacy hover".to_string()),
            &queries,
            "unknown",
            FileId(1),
            10,
            None,
        );

        match &outcome.result {
            HoverCutoverResult::LegacyFallback(text) => {
                assert_eq!(text.as_deref(), Some("legacy hover"));
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        assert_single_hover_trace(
            &outcome,
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::NotApplicable,
            ProviderFallbackState::Fallback,
        )
    }

    #[test]
    fn cutover_fallback_when_low_confidence_heuristic() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("maybe", EntityKind::Unknown, Provenance::NameHeuristic, Confidence::Low);
        let occ =
            make_occurrence(OccurrenceKind::Reference, Provenance::NameHeuristic, Confidence::Low);
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let outcome =
            hover_cutover(Some("legacy".to_string()), &queries, "maybe", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::LegacyFallback(text) => {
                assert_eq!(text.as_deref(), Some("legacy"));
            }
            other => return Err(format!("expected LegacyFallback, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_dynamic_visible_symbols_only() -> Result<(), Box<dyn std::error::Error>> {
        let vis =
            make_visible("dyn_sym", VisibleSymbolSource::DynamicUnknown, Confidence::Low, None);
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![vis] };

        let outcome = hover_cutover(None, &queries, "dyn_sym", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::DynamicBoundary(explanation) => {
                assert!(explanation.markdown.contains("Dynamic boundary"));
                assert!(explanation.markdown.contains("dyn_sym"));
            }
            other => return Err(format!("expected DynamicBoundary, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_visible_symbol_without_entity() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = VisibleSymbolContext::new(Some("MyModule".to_string()), None, None);
        let vis = make_visible(
            "imported_fn",
            VisibleSymbolSource::ExplicitImport,
            Confidence::High,
            Some(ctx),
        );
        let queries =
            StubSemanticQueries { symbol_at_result: None, visible_symbols_result: vec![vis] };

        let outcome = hover_cutover(None, &queries, "imported_fn", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("imported_fn"));
                assert!(explanation.markdown.contains("MyModule"));
                assert!(explanation.markdown.contains("via explicit import list"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_receipt_tracks_entity() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("Foo::bar", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High);
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let outcome = hover_cutover(None, &queries, "Foo::bar", FileId(1), 10, None);

        assert_eq!(outcome.receipt.query, ShadowQueryName::Hover);
        assert_eq!(outcome.receipt.new_result.match_count, 1);
        assert!(outcome.receipt.new_result.available);
        Ok(())
    }

    #[test]
    fn cutover_default_export_shows_source() -> Result<(), Box<dyn std::error::Error>> {
        let entity = make_entity(
            "exported_fn",
            EntityKind::Subroutine,
            Provenance::ExactAst,
            Confidence::High,
        );
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let ctx =
            VisibleSymbolContext::new(Some("Exporter::Mod".to_string()), None, Some(AnchorId(30)));
        let vis = make_visible(
            "exported_fn",
            VisibleSymbolSource::DefaultExport,
            Confidence::High,
            Some(ctx),
        );
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![vis],
        };

        let outcome = hover_cutover(None, &queries, "exported_fn", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("exported_fn"));
                assert!(explanation.markdown.contains("Exporter::Mod"));
                assert!(explanation.markdown.contains("Default export from"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_export_tag_explains_origin() -> Result<(), Box<dyn std::error::Error>> {
        let entity = make_entity(
            "tagged_fn",
            EntityKind::Subroutine,
            Provenance::ExactAst,
            Confidence::High,
        );
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let ctx = VisibleSymbolContext::new(Some("Tagged::Mod".to_string()), None, None);
        let vis =
            make_visible("tagged_fn", VisibleSymbolSource::ExportTag, Confidence::High, Some(ctx));
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![vis],
        };

        let outcome = hover_cutover(None, &queries, "tagged_fn", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("tagged_fn"));
                assert!(explanation.markdown.contains("Export tag from"));
                assert!(explanation.markdown.contains("Tagged::Mod"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn cutover_generated_member_explains_kind() -> Result<(), Box<dyn std::error::Error>> {
        let entity = make_entity(
            "x",
            EntityKind::GeneratedMember,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let occ = make_occurrence(
            OccurrenceKind::Call,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let outcome = hover_cutover(None, &queries, "x", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("Generated member"));
                assert!(explanation.markdown.contains("framework synthesis"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        Ok(())
    }

    #[test]
    fn hover_compiler_provenance_traces_generated_primary_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let entity = make_entity(
            "x",
            EntityKind::GeneratedMember,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let occ = make_occurrence(
            OccurrenceKind::Call,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let queries = StubSemanticQueries {
            symbol_at_result: Some((entity, occ)),
            visible_symbols_result: vec![],
        };

        let outcome = hover_cutover(None, &queries, "x", FileId(1), 10, None);

        match &outcome.result {
            HoverCutoverResult::Exact(explanation) => {
                assert!(explanation.markdown.contains("Generated member"));
                assert!(explanation.markdown.contains("framework synthesis"));
                assert!(explanation.markdown.contains("medium confidence"));
            }
            other => return Err(format!("expected Exact, got {:?}", other).into()),
        }
        assert_single_hover_trace(
            &outcome,
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            ProviderFallbackState::Primary,
        )
    }

    // ── Summary helper tests ──

    #[test]
    fn legacy_hover_to_summary_some_text() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_hover_to_summary(Some("hover text"));
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        Ok(())
    }

    #[test]
    fn legacy_hover_to_summary_empty_text() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_hover_to_summary(Some(""));
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn legacy_hover_to_summary_none() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::legacy_hover_to_summary(None);
        assert!(!summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_hover_to_summary_with_entity() -> Result<(), Box<dyn std::error::Error>> {
        let entity =
            make_entity("Foo::bar", EntityKind::Subroutine, Provenance::ExactAst, Confidence::High);
        let occ = make_occurrence(OccurrenceKind::Call, Provenance::ExactAst, Confidence::High);
        let summary = super::semantic_hover_to_summary(Some(&(entity, occ)), &[], "Foo::bar");
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        Ok(())
    }

    #[test]
    fn semantic_hover_to_summary_no_entity_no_visible() -> Result<(), Box<dyn std::error::Error>> {
        let summary = super::semantic_hover_to_summary(None, &[], "unknown");
        assert!(summary.available);
        assert_eq!(summary.match_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_hover_to_summary_no_entity_with_visible() -> Result<(), Box<dyn std::error::Error>>
    {
        let vis = make_visible("bar", VisibleSymbolSource::ExplicitImport, Confidence::High, None);
        let summary = super::semantic_hover_to_summary(None, &[vis], "bar");
        assert!(summary.available);
        assert_eq!(summary.match_count, 1);
        Ok(())
    }

    // ── Label helper tests ──

    #[test]
    fn entity_kind_labels_cover_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        // Verify all EntityKind variants produce non-empty labels.
        let kinds = [
            EntityKind::Package,
            EntityKind::Class,
            EntityKind::Role,
            EntityKind::Subroutine,
            EntityKind::Method,
            EntityKind::Variable,
            EntityKind::Constant,
            EntityKind::Field,
            EntityKind::Label,
            EntityKind::Format,
            EntityKind::Module,
            EntityKind::GeneratedMember,
            EntityKind::ExternalSymbol,
            EntityKind::Unknown,
        ];
        for kind in &kinds {
            let label = entity_kind_label(*kind);
            assert!(!label.is_empty(), "EntityKind::{kind:?} should have a non-empty label");
        }
        Ok(())
    }

    #[test]
    fn occurrence_kind_labels_cover_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let kinds = [
            OccurrenceKind::Definition,
            OccurrenceKind::Reference,
            OccurrenceKind::Read,
            OccurrenceKind::Write,
            OccurrenceKind::Call,
            OccurrenceKind::MethodCall,
            OccurrenceKind::StaticMethodCall,
            OccurrenceKind::CoderefReference,
            OccurrenceKind::TypeglobReference,
            OccurrenceKind::Import,
            OccurrenceKind::Export,
            OccurrenceKind::Inheritance,
            OccurrenceKind::RoleComposition,
            OccurrenceKind::GeneratedUse,
            OccurrenceKind::DynamicBoundary,
        ];
        for kind in &kinds {
            let label = occurrence_kind_label(*kind);
            // All should produce a label (DynamicBoundary is "dynamic boundary")
            assert!(
                label.len() > 0 || *kind == OccurrenceKind::Definition,
                "OccurrenceKind::{kind:?} should have a label"
            );
        }
        Ok(())
    }

    #[test]
    fn visible_source_labels_cover_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let sources = [
            VisibleSymbolSource::LocalLexical,
            VisibleSymbolSource::LocalPackage,
            VisibleSymbolSource::ExplicitImport,
            VisibleSymbolSource::DefaultExport,
            VisibleSymbolSource::ExportTag,
            VisibleSymbolSource::Constant,
            VisibleSymbolSource::Generated,
            VisibleSymbolSource::External,
            VisibleSymbolSource::DynamicUnknown,
        ];
        for source in &sources {
            let label = visible_source_label(source);
            assert!(
                !label.is_empty(),
                "VisibleSymbolSource::{source:?} should have a non-empty label"
            );
        }
        Ok(())
    }
}
