//! Reference handlers for find references and document highlights
//!
//! Handles textDocument/references and textDocument/documentHighlight requests.
//!
//! # Lifecycle-Aware Behavior
//!
//! Uses `IndexCoordinator` for state-aware dispatch:
//! - **Ready state**: Full workspace index + text search across all files
//! - **Building/Degraded state**: Same-file semantic analysis + open document scan

use super::super::{byte_to_utf16_col, *};
use crate::protocol::{req_position, req_uri};
use crate::state::{reference_search_deadline, references_cap};
use crate::util::{is_word_boundary, token_under_cursor};
use std::sync::OnceLock;
use std::time::Instant;

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_lsp_rs_core::providers::navigation::references_shadow::{
    ReferencesCutoverResult, find_references_live_source_backed,
};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_semantic_facts::AnchorId;
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};

#[cfg(feature = "workspace")]
use crate::runtime::readiness::IndexReadinessPolicy;
#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};

static QUALIFIED_NAME_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

/// Rollback anchor for the first Phase 2 references provider promotion.
///
/// When enabled, only same-file lexical variable requests that explicitly set
/// `includeDeclaration=false` may enter the live semantic source-backed tier.
/// Declaration-including variable requests and all unsupported shapes keep the
/// existing fallback cascade. Flip to `false` to restore the pre-P8 routing
/// boundary without changing the fallback tiers.
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
const ENABLE_PIR_A_LEXICAL_REFERENCES_LIVE: bool = true;

fn lsp_location_count(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        Some(Value::Object(obj)) if obj.contains_key("uri") || obj.contains_key("targetUri") => 1,
        _ => 0,
    }
}

#[derive(Debug)]
struct ReferencesDecisionTraceContext {
    uri: String,
    line: u32,
    character: u32,
    include_declaration: bool,
}

/// Which tier of the 9-internal-tier cascade answered a `textDocument/references` request.
///
/// The 9 internal tiers are collapsed to 8 observable labels for the decision trace.
/// `WorkspaceMixed` distinguishes the case where both index and text search contributed
/// results — a signal that would be lost if collapsed into `WorkspaceExact`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferencesAnsweringTier {
    /// Tier 1 — live compiler source-backed references (most precise).
    SemanticSourceBacked,
    /// Tiers 2, 4, 5 — workspace index `find_refs`/`find_def`/`find_references` only.
    WorkspaceExact,
    /// Tier 3 combined — both index and text search contributed results.
    WorkspaceMixed,
    /// Tier 3 text-only — text search across workspace files, no index hit.
    WorkspaceText,
    /// Tier 7 — partial index `find_refs` (index building/degraded state).
    PartialIndex,
    /// Tier 8 — open-document text search (same-file fallback).
    OpenDocumentText,
    /// Tier 9 — same-file `SemanticAnalyzer::find_all_references`.
    SemanticAnalyzer,
    /// No tier produced a non-empty result.
    Empty,
}

/// Outcome of attempting the live semantic source-backed references path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceBackedReferenceAttempt {
    /// The source-backed path produced exact locations.
    Exact(Vec<Value>),
    /// The source-backed path declined with a named first-failure stage.
    Declined(SourceBackedReferenceDecline),
}

/// Named first-failure stages for the source-backed references attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceBackedReferenceDecline {
    /// The request byte offset could not be represented by the semantic query API.
    ByteOffsetOutOfRange,
    /// No workspace index was available.
    WorkspaceIndexUnavailable,
    /// Semantic queries could not be opened for the request URI.
    SemanticQueriesUnavailableForUri,
    /// Entity resolution did not produce one exact entity.
    EntityUnresolved { symbol_at_found: bool, exact_candidate_count: usize },
    /// The semantic cutover returned a non-exact class.
    CutoverNotExact { result_class: &'static str },
    /// The entity had no usable declaration anchor.
    DeclarationAnchorUnavailable,
    /// The declaration anchor had no wire location.
    DeclarationLocationUnavailable,
    /// The declaration location could not be serialized for the wire response.
    DeclarationSerializationFailed,
    /// The initialized lexical declaration gate rejected the source shape.
    InitializedLexicalGateRejected,
    /// An occurrence had no wire location anchor.
    OccurrenceLocationUnavailable,
    /// An occurrence location could not be serialized for the wire response.
    OccurrenceSerializationFailed,
    /// The exact path produced no locations after filtering.
    EmptyExactResult,
}

impl SourceBackedReferenceAttempt {
    fn receipt_fields(&self) -> SourceBackedReceiptFields {
        match self {
            Self::Exact(_) => SourceBackedReceiptFields {
                attempted: true,
                outcome: "exact",
                decline_stage: None,
                symbol_at_found: false,
                exact_candidate_count: 0,
                cutover_result: Some("exact"),
            },
            Self::Declined(decline) => {
                let (stage, symbol_at_found, exact_candidate_count, cutover_result) = match decline
                {
                    SourceBackedReferenceDecline::ByteOffsetOutOfRange => {
                        ("byte_offset", false, 0, None)
                    }
                    SourceBackedReferenceDecline::WorkspaceIndexUnavailable => {
                        ("workspace_index", false, 0, None)
                    }
                    SourceBackedReferenceDecline::SemanticQueriesUnavailableForUri => {
                        ("semantic_queries", false, 0, None)
                    }
                    SourceBackedReferenceDecline::EntityUnresolved {
                        symbol_at_found,
                        exact_candidate_count,
                    } => ("entity_resolution", *symbol_at_found, *exact_candidate_count, None),
                    SourceBackedReferenceDecline::CutoverNotExact { result_class } => {
                        ("cutover", false, 0, Some(*result_class))
                    }
                    SourceBackedReferenceDecline::DeclarationAnchorUnavailable => {
                        ("declaration_anchor", false, 0, None)
                    }
                    SourceBackedReferenceDecline::DeclarationLocationUnavailable => {
                        ("declaration_location", false, 0, None)
                    }
                    SourceBackedReferenceDecline::DeclarationSerializationFailed => {
                        ("declaration_serialization", false, 0, None)
                    }
                    SourceBackedReferenceDecline::InitializedLexicalGateRejected => {
                        ("initialized_lexical_gate", false, 0, None)
                    }
                    SourceBackedReferenceDecline::OccurrenceLocationUnavailable => {
                        ("occurrence_location", false, 0, None)
                    }
                    SourceBackedReferenceDecline::OccurrenceSerializationFailed => {
                        ("occurrence_serialization", false, 0, None)
                    }
                    SourceBackedReferenceDecline::EmptyExactResult => {
                        ("empty_exact_result", false, 0, None)
                    }
                };
                SourceBackedReceiptFields {
                    attempted: true,
                    outcome: "declined",
                    decline_stage: Some(stage),
                    symbol_at_found,
                    exact_candidate_count,
                    cutover_result,
                }
            }
        }
    }
}

/// Stable receipt fields derived from a source-backed references attempt.
pub(crate) struct SourceBackedReceiptFields {
    pub(crate) attempted: bool,
    pub(crate) outcome: &'static str,
    pub(crate) decline_stage: Option<&'static str>,
    pub(crate) symbol_at_found: bool,
    pub(crate) exact_candidate_count: usize,
    pub(crate) cutover_result: Option<&'static str>,
}

impl ReferencesAnsweringTier {
    /// Stable snake_case label for the tier, written to the decision trace JSON.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SemanticSourceBacked => "semantic_source_backed",
            Self::WorkspaceExact => "workspace_exact",
            Self::WorkspaceMixed => "workspace_mixed",
            Self::WorkspaceText => "workspace_text",
            Self::PartialIndex => "partial_index",
            Self::OpenDocumentText => "open_document_text",
            Self::SemanticAnalyzer => "semantic_analyzer",
            Self::Empty => "empty",
        }
    }

    /// Returns `true` only when semantic source-backed facts answered this request.
    pub(crate) fn is_source_backed(self) -> bool {
        matches!(self, Self::SemanticSourceBacked)
    }

    /// Coarse fact-source label for provider-decision receipts.
    pub(crate) fn fact_source(self) -> &'static str {
        match self {
            // Today this tier is backed by canonical semantic facts generated from
            // AST/index data. It is intentionally not labeled `compiler_fact`;
            // #3046 tracks the future PIR-A-vs-AST split once compiler facts exist.
            Self::SemanticSourceBacked => "semantic_fact",
            Self::WorkspaceExact
            | Self::WorkspaceMixed
            | Self::WorkspaceText
            | Self::PartialIndex
            | Self::OpenDocumentText
            | Self::SemanticAnalyzer
            | Self::Empty => "fallback",
        }
    }

    /// Tier-specific source-backed state for provider-decision receipts.
    pub(crate) fn source_backed_state(self) -> &'static str {
        match self {
            Self::SemanticSourceBacked => "semantic_source_backed_ast_index",
            Self::WorkspaceExact => "workspace_exact_fallback",
            Self::WorkspaceMixed => "workspace_mixed_fallback",
            Self::WorkspaceText => "workspace_text_fallback",
            Self::PartialIndex => "partial_index_fallback",
            Self::OpenDocumentText => "open_document_text_fallback",
            Self::SemanticAnalyzer => "semantic_analyzer_fallback",
            Self::Empty => "no_references_result",
        }
    }

    /// Fallback state normalized by the provider-decision receipt model.
    pub(crate) fn fallback_state(self, result_count: usize) -> &'static str {
        if result_count == 0 {
            "no_result"
        } else if self.is_source_backed() {
            "live_provider"
        } else {
            "legacy_provider"
        }
    }
}

/// Classify the combined workspace+text return site into the correct tier.
///
/// Call this with the counts of index-sourced and text-sourced results BEFORE any
/// truncation so the tier accurately reflects what contributed. Extracted as a pure
/// function so the branch is covered by `--lib` unit tests even when the surrounding
/// handler line is only reached via integration tests.
///
/// - `index_count > 0` and `text_count > 0` → `WorkspaceMixed` (both sources contributed)
/// - `index_count > 0` only → `WorkspaceExact` (pure index answer)
/// - else → `WorkspaceText` (pure text-search answer)
pub(crate) fn classify_combined_tier(
    index_count: usize,
    text_count: usize,
) -> ReferencesAnsweringTier {
    match (index_count > 0, text_count > 0) {
        (true, true) => ReferencesAnsweringTier::WorkspaceMixed,
        (true, false) => ReferencesAnsweringTier::WorkspaceExact,
        _ => ReferencesAnsweringTier::WorkspaceText,
    }
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn may_use_source_backed_references(symbol_is_variable: bool, include_declaration: bool) -> bool {
    !symbol_is_variable || (ENABLE_PIR_A_LEXICAL_REFERENCES_LIVE && !include_declaration)
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn line_has_initialized_lexical_declaration(line: &str, sigil: char, name: &str) -> bool {
    let my_pattern = format!("my {sigil}{name}");
    let state_pattern = format!("state {sigil}{name}");
    for pattern in [my_pattern, state_pattern] {
        let Some(start) = line.find(&pattern) else {
            continue;
        };
        let tail = &line[start + pattern.len()..];
        if tail.contains('=') {
            return true;
        }
    }
    false
}

fn get_qualified_name_regex() -> Option<&'static regex::Regex> {
    QUALIFIED_NAME_RE
        .get_or_init(|| regex::Regex::new(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)"))
        .as_ref()
        .ok()
}

fn search_document_texts_for_references<'a, I>(documents: I, needle: &str, cap: usize) -> Vec<Value>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    if needle.is_empty() || cap == 0 {
        return Vec::new();
    }

    let needle_bytes = needle.as_bytes();
    let mut out = Vec::new();

    'docs: for (doc_uri, doc_text) in documents {
        for (line_num, line) in doc_text.lines().enumerate() {
            let line_bytes = line.as_bytes();
            let mut start = 0usize;
            while let Some(idx) = line[start..].find(needle) {
                let byte_pos = start + idx;
                if is_word_boundary(line_bytes, byte_pos, needle_bytes.len()) {
                    let start_utf16 = byte_to_utf16_col(line, byte_pos);
                    let end_utf16 = byte_to_utf16_col(line, byte_pos + needle_bytes.len());
                    out.push(json!({
                        "uri": doc_uri,
                        "range": {
                            "start": {
                                "line": line_num,
                                "character": start_utf16,
                            },
                            "end": {
                                "line": line_num,
                                "character": end_utf16,
                            },
                        },
                    }));
                    if out.len() >= cap {
                        break 'docs;
                    }
                }
                start = byte_pos + needle_bytes.len();
            }
        }
    }

    out.sort_by_key(|loc| {
        (
            loc["uri"].as_str().unwrap_or("").to_string(),
            loc["range"]["start"]["line"].as_u64().unwrap_or(0),
            loc["range"]["start"]["character"].as_u64().unwrap_or(0),
        )
    });
    out.dedup();
    out.truncate(cap);
    out
}

fn should_skip_text_reference_match(
    line: &str,
    match_start: usize,
    sigil: Option<char>,
    include_declaration: bool,
) -> bool {
    if include_declaration {
        return false;
    }

    let Some(sigil) = sigil else {
        return false;
    };

    let symbol_start = line
        .get(..match_start)
        .and_then(|prefix| prefix.char_indices().next_back())
        .and_then(|(idx, ch)| (ch == sigil).then_some(idx))
        .unwrap_or(match_start);
    let Some(prefix) = line.get(..symbol_start) else {
        return false;
    };

    let statement_prefix =
        prefix.rfind([';', '{', '}']).map(|idx| &prefix[idx + 1..]).unwrap_or(prefix);
    if statement_prefix.contains('=') {
        return false;
    }

    statement_prefix
        .split(|ch: char| !ch.is_ascii_alphabetic() && ch != '_')
        .any(|token| matches!(token, "my" | "our" | "state" | "local"))
}

impl LspServer {
    fn references_decision_trace_context(
        params: Option<&Value>,
    ) -> Result<Option<ReferencesDecisionTraceContext>, JsonRpcError> {
        let Some(params) = params else {
            return Ok(None);
        };
        let uri = req_uri(params)?.to_string();
        let (line, character) = req_position(params)?;
        let include_declaration = if let Some(context) = params.get("context") {
            context["includeDeclaration"].as_bool().unwrap_or(true)
        } else {
            true
        };
        Ok(Some(ReferencesDecisionTraceContext { uri, line, character, include_declaration }))
    }

    #[allow(clippy::too_many_arguments)]
    fn record_references_provider_decision_trace(
        &self,
        context: Option<&ReferencesDecisionTraceContext>,
        result: Option<&Value>,
        tier: ReferencesAnsweringTier,
        index_state: &str,
        index_result_count: usize,
        text_result_count: usize,
        latency_us: u128,
        source_backed_attempt: Option<&SourceBackedReferenceAttempt>,
    ) {
        let Some(context) = context else {
            return;
        };
        let result_count = lsp_location_count(result);
        let (decision, reason) = if result_count == 0 {
            ("fallback", "no_result")
        } else {
            ("acted", "live_provider_result")
        };
        let fallback_state = tier.fallback_state(result_count);
        // Confidence is high only when the semantic source-backed tier answered.
        let confidence = if tier.is_source_backed() { "high" } else { "low" };
        // source_backed_result_count is the total result count only for source-backed answers
        let source_backed_result_count: usize =
            if tier.is_source_backed() { result_count } else { 0 };

        let SourceBackedReceiptFields {
            attempted: source_backed_attempted,
            outcome: source_backed_outcome,
            decline_stage: source_backed_decline_stage,
            symbol_at_found: source_backed_symbol_at_found,
            exact_candidate_count: source_backed_exact_candidate_count,
            cutover_result: source_backed_cutover_result,
        } = match source_backed_attempt {
            Some(attempt) => attempt.receipt_fields(),
            None => SourceBackedReceiptFields {
                attempted: false,
                outcome: "not_attempted",
                decline_stage: None,
                symbol_at_found: false,
                exact_candidate_count: 0,
                cutover_result: None,
            },
        };

        self.record_provider_decision_trace(
            "references",
            &json!({
                "provider": "references",
                "provider_action": "textDocument/references",
                "decision": decision,
                "reason": reason,
                "uri": context.uri,
                "line": context.line,
                "character": context.character,
                "include_declaration": context.include_declaration,
                "result_count": result_count,
                "index_result_count": index_result_count,
                "text_result_count": text_result_count,
                "source_backed_result_count": source_backed_result_count,
                "fact_source": tier.fact_source(),
                "confidence": confidence,
                "freshness": "fresh",
                "source_backed": tier.is_source_backed(),
                "source_backed_state": tier.source_backed_state(),
                "answering_tier": tier.as_str(),
                "index_state": index_state,
                "latency_us": latency_us,
                "fallback_state": fallback_state,
                "dynamic_boundary": false,
                "trace_only_no_live_behavior_change": true,
                "source_backed_attempted": source_backed_attempted,
                "source_backed_outcome": source_backed_outcome,
                "source_backed_decline_stage": source_backed_decline_stage,
                "source_backed_symbol_at_found": source_backed_symbol_at_found,
                "source_backed_exact_candidate_count": source_backed_exact_candidate_count,
                "source_backed_cutover_result": source_backed_cutover_result,
                "claim_boundary": "records existing references response only; no broader live references cutover"
            }),
        );
    }

    /// Handle textDocument/references request with lifecycle-aware dispatch
    ///
    /// Uses `IndexCoordinator` for state-aware behavior:
    /// - **Ready state**: Full workspace index search + text-based fallback
    /// - **Building/Degraded state**: Same-file semantic analysis only
    ///
    /// Includes deadline enforcement to prevent blocking on large workspaces.
    pub(crate) fn handle_references(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let trace_context = Self::references_decision_trace_context(params.as_ref())?;
        let (
            result,
            tier,
            index_state,
            index_result_count,
            text_result_count,
            latency_us,
            source_backed_attempt,
        ) = self.handle_references_inner(params)?;
        self.record_references_provider_decision_trace(
            trace_context.as_ref(),
            result.as_ref(),
            tier,
            index_state,
            index_result_count,
            text_result_count,
            latency_us,
            source_backed_attempt.as_ref(),
        );
        Ok(result)
    }

    /// Returns `(result, tier, index_state, index_result_count, text_result_count, latency_us)`.
    ///
    /// - `index_state`: `"full" | "partial" | "none"` derived from the observed `IndexAccessMode`
    ///   at the branch point — NOT inferred from the tier (a Full index can still fall through
    ///   to `semantic_analyzer` when no symbol is found).
    /// - `index_result_count`: results sourced from the workspace index (before truncation).
    /// - `text_result_count`: results sourced from regex/text search (before truncation).
    /// - `latency_us`: wall-clock microseconds for the full dispatch.
    #[allow(clippy::type_complexity)]
    fn handle_references_inner(
        &self,
        params: Option<Value>,
    ) -> Result<
        (
            Option<Value>,
            ReferencesAnsweringTier,
            &'static str,
            usize,
            usize,
            u128,
            Option<SourceBackedReferenceAttempt>,
        ),
        JsonRpcError,
    > {
        let start = Instant::now();
        let deadline = reference_search_deadline();
        let cap = references_cap();
        let mut source_backed_attempt: Option<SourceBackedReferenceAttempt> = None;

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            self.ensure_latest(uri, req_version)?;
            let workspace_index_stale_for_document = self.workspace_index_stale_for_document(uri);
            let include_declaration = if let Some(context) = params.get("context") {
                context["includeDeclaration"].as_bool().unwrap_or(true)
            } else {
                true
            };

            // Phase 1: grab an owned `DocumentState` clone under a brief
            // documents-map lock, then drop the guard before doing any analysis
            // (#3396 off-lock provider consumption). Below, the two full-
            // workspace text-snapshot fallbacks (qualified-name scan, open-doc
            // scan) each re-acquire a fresh, equally brief lock only at the
            // point they need it -- neither acquisition holds the guard across
            // any analysis in between. `ScopedSpan` covers the whole analysis
            // block via `Drop`, so it emits correctly regardless of which of
            // this function's several early `return` points fires.
            //
            // Consistency note: each `docs_snapshot` fetch is a fresh,
            // independent lock acquisition, so in general it observes
            // whatever generation of each document is live *at that later
            // point* -- not necessarily the same generation `doc_owned`
            // captured above. For every *other* open document that's fine
            // (the fallback is a heuristic, name-based regex scan with no
            // offset dependency on `doc_owned`). For `uri` itself it is not:
            // `symbol_key`/`offset`/`needle` below are all derived from
            // `doc_owned`'s generation, so searching them against a *fresher*
            // re-read of the same uri (if a `didChange` races in between the
            // two lock acquisitions) would pair a generation-N identity with
            // generation-N+1 text for the same document -- the exact
            // single-instance/single-generation invariant this off-lock
            // pattern must preserve to stay behavior-identical. Each
            // `docs_snapshot` construction below therefore pins `uri`'s own
            // entry to `doc.text` (i.e. `doc_owned`, not the live map) and
            // only lets *other* documents float to the freshest read.
            let timing_on = crate::runtime::timing::is_enabled();
            let t_lock_start = std::time::Instant::now();
            let doc_owned = {
                let documents = self.documents_guard();
                self.get_document(&documents, uri).cloned()
            };
            // documents guard dropped here
            if timing_on {
                crate::runtime::timing::emit(crate::runtime::timing::TimingSpan::labeled(
                    "provider.references.lock_hold",
                    crate::runtime::timing::elapsed_ms(t_lock_start),
                    crate::runtime::timing::uri_tail(uri),
                ));
            }

            if let Some(doc) = doc_owned.as_ref() {
                let _analyze_span =
                    crate::runtime::timing::ScopedSpan::start("provider.references.analyze", uri);
                let parsed = doc.current_parsed();
                if let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) {
                    let offset = self.pos16_to_offset(doc, line, character);
                    let needle = token_under_cursor(&doc.text, line as usize, character as usize)
                        .unwrap_or_default();

                    let current_package = crate::declaration::current_package_at(ast, offset);
                    let symbol_key = crate::declaration::symbol_at_cursor_with_source(
                        ast,
                        offset,
                        current_package,
                        &doc.text,
                    );

                    // Capture the true index state BEFORE branching. Declared here (outside the
                    // cfg block) so it remains in scope for the semantic-analyzer fallback path
                    // when the workspace feature is enabled but no index path early-returned.
                    // When the workspace feature is disabled, the non-workspace cfg below wins.
                    #[cfg(feature = "workspace")]
                    let index_state: &'static str;
                    #[cfg(not(feature = "workspace"))]
                    let index_state: &'static str = "none";

                    // Wait for the workspace index to finish building before querying it.
                    // Without this, a references request while the index is in Building state
                    // routes to Partial and misses cross-file usages in non-open files.
                    // The text-search fallback masks this in tests (open files are scanned),
                    // but production users on large workspaces see empty cross-file results.
                    // Mirrors the pattern used by completion (#3069) and workspace/symbol (#1514).
                    #[cfg(feature = "workspace")]
                    let _ = self.check_index_readiness(IndexReadinessPolicy::WaitBriefly);

                    // Check index state and use appropriate search strategy
                    #[cfg(feature = "workspace")]
                    {
                        let mut access_mode = route_index_access(self.coordinator());
                        if workspace_index_stale_for_document {
                            access_mode = IndexAccessMode::None;
                        }
                        // A Full index can still fall through to semantic_analyzer when no symbol
                        // is found, so inferring index_state from the tier would be wrong.
                        index_state = match &access_mode {
                            IndexAccessMode::Full(_) => "full",
                            IndexAccessMode::Partial(_) => "partial",
                            IndexAccessMode::None => "none",
                        };
                        let workspace_symbol_key =
                            symbol_key.as_ref().map(super::to_workspace_symbol_key);

                        match access_mode {
                            IndexAccessMode::Full(coordinator) => {
                                let index = coordinator.index();
                                if let Some(symbol_key) = workspace_symbol_key.as_ref() {
                                    // Guard: sigil-prefixed lexical variables may use the semantic
                                    // source-backed tier only for the Phase 2 P8 slice:
                                    // includeDeclaration=false and the rollback gate enabled.
                                    // Declaration-including lexical requests remain on the
                                    // existing fallback cascade. Subroutine references (no sigil)
                                    // may use the semantic tier with includeDeclaration=true —
                                    // that is the #2673 fix for VS Code's default request shape.
                                    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                                    let symbol_is_variable = symbol_key.sigil.is_some();
                                    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                                    if may_use_source_backed_references(
                                        symbol_is_variable,
                                        include_declaration,
                                    ) {
                                        let live_attempt = self
                                            .live_source_backed_reference_locations(
                                                uri,
                                                symbol_key.name.as_ref(),
                                                doc.text.as_str(),
                                                symbol_key.sigil,
                                                offset,
                                                include_declaration,
                                            );
                                        match live_attempt {
                                            SourceBackedReferenceAttempt::Exact(
                                                mut live_locations,
                                            ) => {
                                                // The receipt only needs the outcome marker. Keep
                                                // its vector empty so the exact response is not
                                                // cloned solely for observability.
                                                source_backed_attempt = Some(
                                                    SourceBackedReferenceAttempt::Exact(Vec::new()),
                                                );
                                                live_locations.truncate(cap);
                                                // Precompute before the tracing macro so these
                                                // expressions are unconditionally instrumented
                                                // rather than lazily evaluated only when the
                                                // debug subscriber is active.
                                                let ref_count = live_locations.len();
                                                let elapsed = start.elapsed();
                                                tracing::debug!(
                                                    ref_count,
                                                    elapsed = ?elapsed,
                                                    "References: returned live source-backed compiler facts"
                                                );
                                                let result_count = live_locations.len();
                                                return Ok((
                                                    Some(json!(live_locations)),
                                                    ReferencesAnsweringTier::SemanticSourceBacked,
                                                    index_state,
                                                    result_count,
                                                    0,
                                                    start.elapsed().as_micros(),
                                                    source_backed_attempt,
                                                ));
                                            }
                                            SourceBackedReferenceAttempt::Declined(decline) => {
                                                source_backed_attempt = Some(
                                                    SourceBackedReferenceAttempt::Declined(decline),
                                                );
                                            }
                                        }
                                    }

                                    tracing::debug!(key = ?symbol_key, "Looking for references");

                                    // Try to find references using the symbol key
                                    let mut all_refs = index.find_refs(symbol_key);

                                    // Add the definition if includeDeclaration is true
                                    if include_declaration {
                                        if let Some(def) = index.find_def(symbol_key) {
                                            all_refs.push(def);
                                        }
                                    }

                                    let mut workspace_locations: Vec<Value> = Vec::new();
                                    if !all_refs.is_empty() {
                                        tracing::debug!(
                                            count = all_refs.len(),
                                            "Found references via find_refs"
                                        );
                                        // Convert internal Locations to LSP Locations
                                        let lsp_locations =
                                            crate::workspace_index::lsp_adapter::to_lsp_locations(
                                                all_refs,
                                            );
                                        for loc in lsp_locations {
                                            workspace_locations.push(json!(loc));
                                        }
                                    }

                                    // Check deadline before text search
                                    if start.elapsed() >= deadline {
                                        tracing::debug!(
                                            "References: deadline exceeded, returning partial results"
                                        );
                                        let index_count = workspace_locations.len();
                                        workspace_locations.truncate(cap);
                                        return Ok((
                                            Some(json!(workspace_locations)),
                                            ReferencesAnsweringTier::WorkspaceExact,
                                            index_state,
                                            index_count,
                                            0,
                                            start.elapsed().as_micros(),
                                            source_backed_attempt.clone(),
                                        ));
                                    }

                                    // Enhanced fallback: always search for both qualified and unqualified references
                                    // Snapshot only (uri, text) to minimize cloning overhead - we don't need
                                    // AST, rope, or other DocumentState fields for text search.
                                    // Re-acquires a fresh, brief documents-map lock only at this
                                    // point of use (#3396 off-lock provider consumption) -- the
                                    // outer lock was already dropped after fetching `doc` above.
                                    //
                                    // `uri`'s own entry is pinned to `doc.text` (the exact
                                    // generation captured in `doc_owned` above) rather than
                                    // whatever is live now -- `symbol_name`/`package_name` below
                                    // were derived from that same capture's AST, so searching them
                                    // against a *fresher* re-read of `uri` (if a `didChange` raced
                                    // in between the two lock acquisitions) would pair a
                                    // generation-N identity with generation-N+1 text for the same
                                    // document. Every other open document is unaffected by this and
                                    // still gets the freshest available read.
                                    let docs_snapshot: Vec<(String, String)> = {
                                        let documents = self.documents_guard();
                                        documents
                                            .iter()
                                            .map(|(k, v)| {
                                                if k.as_str() == uri {
                                                    (k.clone(), doc.text.clone())
                                                } else {
                                                    (k.clone(), v.text.clone())
                                                }
                                            })
                                            .collect()
                                    };

                                    let mut enhanced_locations = Vec::new();
                                    let symbol_name = &symbol_key.name;
                                    let package_name = &symbol_key.pkg;

                                    // Search patterns: both "symbol_name" and "package::symbol_name"
                                    let patterns = vec![
                                        format!(r"\b{}\b", regex::escape(symbol_name)),
                                        format!(
                                            r"\b{}::{}\b",
                                            regex::escape(package_name),
                                            regex::escape(symbol_name)
                                        ),
                                    ];

                                    'pattern_loop: for pattern in patterns {
                                        // Check deadline between patterns
                                        if start.elapsed() >= deadline {
                                            tracing::debug!(
                                                "References: deadline exceeded during text search"
                                            );
                                            break 'pattern_loop;
                                        }
                                        if let Ok(search_regex) = regex::Regex::new(&pattern) {
                                            for (doc_uri, doc_text) in &docs_snapshot {
                                                // Early exit on cap
                                                if enhanced_locations.len() >= cap {
                                                    break 'pattern_loop;
                                                }
                                                let lines: Vec<&str> = doc_text.lines().collect();
                                                for (line_num, line) in lines.iter().enumerate() {
                                                    for mat in search_regex.find_iter(line) {
                                                        if should_skip_text_reference_match(
                                                            line,
                                                            mat.start(),
                                                            symbol_key.sigil,
                                                            include_declaration,
                                                        ) {
                                                            continue;
                                                        }
                                                        // Convert byte offsets to UTF-16 columns for LSP compliance
                                                        let start_utf16 =
                                                            byte_to_utf16_col(line, mat.start());
                                                        let end_utf16 =
                                                            byte_to_utf16_col(line, mat.end());
                                                        enhanced_locations.push(json!({
                                                            "uri": doc_uri,
                                                            "range": {
                                                                "start": {
                                                                    "line": line_num,
                                                                    "character": start_utf16,
                                                                },
                                                                "end": {
                                                                    "line": line_num,
                                                                    "character": end_utf16,
                                                                },
                                                            },
                                                        }));
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Combine workspace index results with text search results.
                                    // Capture counts BEFORE extending so classify_combined_tier
                                    // knows whether each source contributed — a mixed result
                                    // (WorkspaceMixed) must not be collapsed into WorkspaceExact.
                                    let index_count = workspace_locations.len();
                                    let text_count = enhanced_locations.len();
                                    workspace_locations.extend(enhanced_locations);
                                    let mut all_combined_locations = workspace_locations;
                                    // Cap results
                                    all_combined_locations.truncate(cap);

                                    if !all_combined_locations.is_empty() {
                                        tracing::debug!(
                                            count = all_combined_locations.len(),
                                            cap,
                                            elapsed = ?start.elapsed(),
                                            "Found total references via combined search"
                                        );
                                        return Ok((
                                            Some(json!(all_combined_locations)),
                                            classify_combined_tier(index_count, text_count),
                                            index_state,
                                            index_count,
                                            text_count,
                                            start.elapsed().as_micros(),
                                            source_backed_attempt.clone(),
                                        ));
                                    }

                                    // Also try with find_references for backward compatibility
                                    let symbol_name = if symbol_key.kind
                                        == crate::workspace_index::SymKind::Sub
                                    {
                                        format!("{}::{}", symbol_key.pkg, symbol_key.name)
                                    } else {
                                        symbol_key.name.to_string()
                                    };

                                    let refs = index.find_references(&symbol_name);
                                    if !refs.is_empty() {
                                        // Cap results before conversion
                                        let capped_refs: Vec<_> =
                                            refs.into_iter().take(cap).collect();
                                        tracing::debug!(
                                            count = capped_refs.len(),
                                            symbol = %symbol_name,
                                            cap,
                                            "Found references via find_references"
                                        );
                                        // Convert internal Locations to LSP Locations
                                        let lsp_locations =
                                            crate::workspace_index::lsp_adapter::to_lsp_locations(
                                                capped_refs,
                                            );
                                        if !lsp_locations.is_empty() {
                                            let result_count = lsp_locations.len();
                                            return Ok((
                                                Some(json!(lsp_locations)),
                                                ReferencesAnsweringTier::WorkspaceExact,
                                                index_state,
                                                result_count,
                                                0,
                                                start.elapsed().as_micros(),
                                                source_backed_attempt.clone(),
                                            ));
                                        }
                                    }
                                }

                                // Regex-based fallback for fully-qualified symbols like Package::sub references
                                let radius = 50;
                                let (text_start, text_around) =
                                    self.get_text_window_around_offset(&doc.text, offset, radius);
                                let cursor_in_text =
                                    offset.min(doc.text.len()).saturating_sub(text_start);

                                // Use cached regex to avoid per-request compilation overhead
                                if let Some(qualified_name_re) = get_qualified_name_regex() {
                                    for captures in qualified_name_re.captures_iter(&text_around) {
                                        if let Some(m) = captures.get(1) {
                                            if cursor_in_text >= m.start()
                                                && cursor_in_text <= m.end()
                                            {
                                                let parts: Vec<&str> =
                                                    m.as_str().split("::").collect();
                                                if parts.len() >= 2 {
                                                    // Only search for references when the cursor
                                                    // is on the final component (sub/function name).
                                                    // If the cursor is on a package-prefix component
                                                    // (e.g. `Foo` in `Foo::bar`), skip this match
                                                    // so we do not return references to the wrong
                                                    // symbol.
                                                    let cursor_rel =
                                                        cursor_in_text.saturating_sub(m.start());
                                                    let last_sep_offset =
                                                        m.as_str().rfind("::").map_or(0, |p| p + 2);
                                                    if cursor_rel < last_sep_offset {
                                                        break;
                                                    }

                                                    let name = parts
                                                        .last()
                                                        .copied()
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let pkg = parts[..parts.len() - 1].join("::");
                                                    let key = crate::workspace_index::SymbolKey {
                                                        pkg: pkg.clone().into(),
                                                        name: name.clone().into(),
                                                        sigil: None,
                                                        kind: crate::workspace_index::SymKind::Sub,
                                                    };

                                                    // Search for all references to this qualified symbol
                                                    let mut all_refs = Vec::new();

                                                    // Find references via symbol key
                                                    let refs = index.find_refs(&key);
                                                    all_refs.extend(refs);

                                                    // Also try with qualified name
                                                    let symbol_name = format!("{}::{}", pkg, name);
                                                    let alt_refs =
                                                        index.find_references(&symbol_name);
                                                    all_refs.extend(alt_refs);

                                                    // Add definition if includeDeclaration is true
                                                    if include_declaration {
                                                        if let Some(def) = index.find_def(&key) {
                                                            all_refs.push(def);
                                                        }
                                                    }

                                                    if !all_refs.is_empty() {
                                                        // Cap results
                                                        let capped_refs: Vec<_> = all_refs
                                                            .into_iter()
                                                            .take(cap)
                                                            .collect();
                                                        // Convert internal Locations to LSP Locations
                                                        let lsp_locations =
                                                    crate::workspace_index::lsp_adapter::to_lsp_locations(capped_refs);
                                                        if !lsp_locations.is_empty() {
                                                            let result_count = lsp_locations.len();
                                                            return Ok((
                                                                Some(json!(lsp_locations)),
                                                                ReferencesAnsweringTier::WorkspaceExact,
                                                                index_state,
                                                                result_count,
                                                                0,
                                                                start.elapsed().as_micros(),
                                                                source_backed_attempt.clone(),
                                                            ));
                                                        }
                                                    }

                                                    // Fallback: scan open documents for qualified name references
                                                    // Snapshot only (uri, text) to minimize cloning overhead.
                                                    // Re-acquires a fresh, brief documents-map lock only at
                                                    // this point of use (#3396 off-lock provider consumption).
                                                    //
                                                    // `uri`'s own entry is pinned to `doc.text` (the
                                                    // generation captured in `doc_owned` above) -- see
                                                    // the identical rationale on the enhanced-fallback
                                                    // snapshot above: `qualified_name` was derived from
                                                    // that same capture, so this document must not be
                                                    // re-read at a fresher generation for this search.
                                                    let docs_snapshot: Vec<(String, String)> = {
                                                        let documents = self.documents_guard();
                                                        documents
                                                            .iter()
                                                            .map(|(k, v)| {
                                                                if k.as_str() == uri {
                                                                    (k.clone(), doc.text.clone())
                                                                } else {
                                                                    (k.clone(), v.text.clone())
                                                                }
                                                            })
                                                            .collect()
                                                    };

                                                    let mut all_locations = Vec::new();
                                                    let qualified_name =
                                                        format!("{}::{}", pkg, name);
                                                    let Ok(search_regex) =
                                                        regex::Regex::new(&format!(
                                                            r"\b{}\b",
                                                            regex::escape(&qualified_name)
                                                        ))
                                                    else {
                                                        continue;
                                                    };

                                                    'doc_scan: for (doc_uri, doc_text) in
                                                        docs_snapshot
                                                    {
                                                        // Check deadline
                                                        if start.elapsed() >= deadline {
                                                            break 'doc_scan;
                                                        }
                                                        let lines: Vec<&str> =
                                                            doc_text.lines().collect();
                                                        for (line_num, line) in
                                                            lines.iter().enumerate()
                                                        {
                                                            for mat in search_regex.find_iter(line)
                                                            {
                                                                // Convert byte offsets to UTF-16 columns for LSP compliance
                                                                let start_utf16 = byte_to_utf16_col(
                                                                    line,
                                                                    mat.start(),
                                                                );
                                                                let end_utf16 = byte_to_utf16_col(
                                                                    line,
                                                                    mat.end(),
                                                                );
                                                                all_locations.push(json!({
                                                                "uri": doc_uri,
                                                                "range": {
                                                                    "start": {
                                                                        "line": line_num,
                                                                        "character": start_utf16,
                                                                    },
                                                                    "end": {
                                                                        "line": line_num,
                                                                        "character": end_utf16,
                                                                    },
                                                                },
                                                            }));
                                                                // Early exit if we hit the cap
                                                                if all_locations.len() >= cap {
                                                                    break 'doc_scan;
                                                                }
                                                            }
                                                        }
                                                    }

                                                    if !all_locations.is_empty() {
                                                        let text_count = all_locations.len();
                                                        // Truncate to cap
                                                        all_locations.truncate(cap);
                                                        return Ok((
                                                            Some(json!(all_locations)),
                                                            ReferencesAnsweringTier::WorkspaceText,
                                                            index_state,
                                                            0,
                                                            text_count,
                                                            start.elapsed().as_micros(),
                                                            source_backed_attempt.clone(),
                                                        ));
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            IndexAccessMode::Partial(reason) => {
                                tracing::debug!(
                                    reason,
                                    "References: attempting partial workspace lookup"
                                );
                                if let (Some(coordinator), Some(symbol_key)) =
                                    (self.coordinator(), workspace_symbol_key.as_ref())
                                {
                                    let index = coordinator.index();
                                    let mut partial_refs = index.find_refs(symbol_key);

                                    if include_declaration
                                        && let Some(def) = index.find_def(symbol_key)
                                    {
                                        partial_refs.push(def);
                                    }

                                    if !partial_refs.is_empty() {
                                        let lsp_locations =
                                            crate::workspace_index::lsp_adapter::to_lsp_locations(
                                                partial_refs.into_iter().take(cap),
                                            );
                                        if !lsp_locations.is_empty() {
                                            tracing::debug!(
                                                count = lsp_locations.len(),
                                                elapsed = ?start.elapsed(),
                                                "References: returned partial-index results"
                                            );
                                            let result_count = lsp_locations.len();
                                            return Ok((
                                                Some(json!(lsp_locations)),
                                                ReferencesAnsweringTier::PartialIndex,
                                                index_state,
                                                result_count,
                                                0,
                                                start.elapsed().as_micros(),
                                                source_backed_attempt.clone(),
                                            ));
                                        }
                                    }
                                }

                                tracing::debug!(reason, "References: using same-file fallback");
                                if !needle.is_empty() {
                                    // Re-acquires a fresh, brief documents-map lock only at this
                                    // point of use (#3396 off-lock provider consumption) -- the
                                    // outer lock was already dropped after fetching `doc` above.
                                    //
                                    // `uri`'s own entry is pinned to `doc.text` (the generation
                                    // captured in `doc_owned` above) -- `needle` was derived from
                                    // that same capture (`token_under_cursor(&doc.text, ...)`
                                    // above), so this document must not be re-read at a fresher
                                    // generation for this search. Every other open document still
                                    // gets the freshest available read.
                                    let docs_snapshot: Vec<(String, String)> = {
                                        let documents = self.documents_guard();
                                        documents
                                            .iter()
                                            .map(|(k, v)| {
                                                if k.as_str() == uri {
                                                    (k.clone(), doc.text.clone())
                                                } else {
                                                    (k.clone(), v.text.clone())
                                                }
                                            })
                                            .collect()
                                    };
                                    let open_doc_locations = search_document_texts_for_references(
                                        docs_snapshot.iter().map(|(doc_uri, doc_text)| {
                                            (doc_uri.as_str(), doc_text.as_str())
                                        }),
                                        &needle,
                                        cap,
                                    );
                                    if !open_doc_locations.is_empty() {
                                        tracing::debug!(
                                            count = open_doc_locations.len(),
                                            elapsed = ?start.elapsed(),
                                            "References: returned open-document results"
                                        );
                                        let result_count = open_doc_locations.len();
                                        return Ok((
                                            Some(json!(open_doc_locations)),
                                            ReferencesAnsweringTier::OpenDocumentText,
                                            index_state,
                                            0,
                                            result_count,
                                            start.elapsed().as_micros(),
                                            source_backed_attempt.clone(),
                                        ));
                                    }
                                }
                                // Fall through to same-file semantic analysis
                            }
                            IndexAccessMode::None => {
                                // Fall through to same-file semantic analysis
                            }
                        }
                    }

                    // Fall back to same-file references.
                    // `index_state` was captured before the workspace block above.
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find all references at the position
                    let references = analyzer.find_all_references(offset, include_declaration);

                    if !references.is_empty() {
                        // Cap same-file references
                        let locations: Vec<Value> = references
                            .iter()
                            .take(cap)
                            .map(|loc| {
                                let (start_line, start_char) = self.offset_to_pos16(doc, loc.start);
                                let (end_line, end_char) = self.offset_to_pos16(doc, loc.end);

                                json!({
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": start_line,
                                            "character": start_char,
                                        },
                                        "end": {
                                            "line": end_line,
                                            "character": end_char,
                                        },
                                    },
                                })
                            })
                            .collect();

                        tracing::debug!(
                            count = locations.len(),
                            elapsed = ?start.elapsed(),
                            "References: returned same-file results"
                        );
                        return Ok((
                            Some(json!(locations)),
                            ReferencesAnsweringTier::SemanticAnalyzer,
                            index_state,
                            0,
                            0,
                            start.elapsed().as_micros(),
                            source_backed_attempt.clone(),
                        ));
                    }
                }
            }
        }

        Ok((
            Some(json!([])),
            ReferencesAnsweringTier::Empty,
            "none",
            0,
            0,
            start.elapsed().as_micros(),
            source_backed_attempt.clone(),
        ))
    }

    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    fn live_source_backed_reference_locations(
        &self,
        uri: &str,
        symbol: &str,
        source: &str,
        sigil: Option<char>,
        byte_offset: usize,
        include_declaration: bool,
    ) -> SourceBackedReferenceAttempt {
        let byte_offset = match u32::try_from(byte_offset) {
            Ok(byte_offset) => byte_offset,
            Err(_) => {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::ByteOffsetOutOfRange,
                );
            }
        };
        let Some(workspace_index) = self.workspace_index() else {
            return SourceBackedReferenceAttempt::Declined(
                SourceBackedReferenceDecline::WorkspaceIndexUnavailable,
            );
        };

        // Resolve the semantic outcome plus the declaration anchor when either
        // the caller wants it included or the P8 lexical slice needs to prove
        // this entity is an initialized lexical declaration.
        let semantic_resolution = workspace_index
            .with_semantic_queries_for_uri(uri, |file_id, queries| {
                let ctx = QueryContext::new(file_id, None, Some(byte_offset));

                // Two-step entity resolution: prefer the typed occurrence at
                // the cursor, fall back to a uniquely-matching definition
                // candidate.  When resolving via definitions we keep the
                // anchor around so we can include it as the declaration site.
                let symbol_at = queries.symbol_at(file_id, byte_offset);
                let symbol_at_found = symbol_at.is_some();
                let entity_id =
                    match symbol_at.as_ref().and_then(|(_, occurrence)| occurrence.entity_id) {
                        Some(entity_id) => entity_id,
                        None => {
                            let exact_candidates: Vec<_> = queries
                                .definitions(symbol, &ctx)
                                .into_iter()
                                .filter(|candidate| {
                                    candidate.confidence == perl_semantic_facts::Confidence::High
                                        && matches!(
                                        candidate.provenance,
                                        perl_semantic_facts::Provenance::ExactAst
                                            | perl_semantic_facts::Provenance::ImportExportInference
                                            | perl_semantic_facts::Provenance::LiteralRequireImport
                                    ) && workspace_index
                                        .semantic_anchor_wire_location(candidate.anchor_id)
                                        .is_some()
                                })
                                .collect();
                            match exact_candidates.as_slice() {
                                [candidate] => candidate.entity_id,
                                _ => {
                                    return Some(Err(
                                        SourceBackedReferenceDecline::EntityUnresolved {
                                            symbol_at_found,
                                            exact_candidate_count: exact_candidates.len(),
                                        },
                                    ));
                                }
                            }
                        }
                    };

                // Find the declaration anchor for this entity.  We accept the
                // anchor from `symbol_at` if the occurrence is a definition
                // kind, or look up a high-confidence definition candidate
                // otherwise.
                let decl_anchor: Option<AnchorId> = if include_declaration || sigil.is_some() {
                    use perl_semantic_facts::OccurrenceKind;
                    let from_symbol_at = symbol_at
                        .as_ref()
                        .filter(|(_, occ)| occ.kind == OccurrenceKind::Definition)
                        .map(|(_, occ)| occ.anchor_id);
                    from_symbol_at.or_else(|| {
                        queries
                            .definitions(symbol, &ctx)
                            .into_iter()
                            .filter(|c| {
                                c.confidence == perl_semantic_facts::Confidence::High
                                    && c.entity_id == entity_id
                                    && workspace_index
                                        .semantic_anchor_wire_location(c.anchor_id)
                                        .is_some()
                            })
                            .map(|c| c.anchor_id)
                            .next()
                    })
                } else {
                    None
                };

                let outcome = find_references_live_source_backed(
                    workspace_index.as_ref(),
                    &queries,
                    symbol,
                    entity_id,
                );
                Some(Ok((outcome, decl_anchor)))
            })
            .flatten();
        let Some(semantic_resolution) = semantic_resolution else {
            return SourceBackedReferenceAttempt::Declined(
                SourceBackedReferenceDecline::SemanticQueriesUnavailableForUri,
            );
        };
        let (outcome, decl_anchor) = match semantic_resolution {
            Ok(resolution) => resolution,
            Err(decline) => return SourceBackedReferenceAttempt::Declined(decline),
        };

        let occurrences = match outcome.result {
            ReferencesCutoverResult::Exact(occurrences) => occurrences,
            ReferencesCutoverResult::Ambiguous(_) => {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::CutoverNotExact { result_class: "ambiguous" },
                );
            }
            ReferencesCutoverResult::LegacyFallback(_) => {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::CutoverNotExact {
                        result_class: "legacy_fallback",
                    },
                );
            }
        };

        if let Some(sigil) = sigil {
            let Some(decl_anchor) = decl_anchor else {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::DeclarationAnchorUnavailable,
                );
            };
            let Some(wire_location) = workspace_index.semantic_anchor_wire_location(decl_anchor)
            else {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::DeclarationLocationUnavailable,
                );
            };
            let Ok(decl_line) = usize::try_from(wire_location.range.start.line) else {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::DeclarationLocationUnavailable,
                );
            };
            let Some(line) = source.lines().nth(decl_line) else {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::DeclarationLocationUnavailable,
                );
            };
            if !line_has_initialized_lexical_declaration(line, sigil, symbol) {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::InitializedLexicalGateRejected,
                );
            }
        }

        let mut locations = Vec::with_capacity(occurrences.len() + 1);
        for occurrence in occurrences {
            let Some(wire_location) =
                workspace_index.semantic_anchor_wire_location(occurrence.anchor_id)
            else {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::OccurrenceLocationUnavailable,
                );
            };
            let location: lsp_types::Location = wire_location.into();
            let Ok(location) = serde_json::to_value(location) else {
                return SourceBackedReferenceAttempt::Declined(
                    SourceBackedReferenceDecline::OccurrenceSerializationFailed,
                );
            };
            locations.push(location);
        }

        // Include the declaration location when requested, deduped against the
        // reference set already collected above.
        if include_declaration {
            if let Some(anchor_id) = decl_anchor {
                if let Some(wire_location) =
                    workspace_index.semantic_anchor_wire_location(anchor_id)
                {
                    let decl_location: lsp_types::Location = wire_location.into();
                    let Ok(decl_value) = serde_json::to_value(&decl_location) else {
                        return SourceBackedReferenceAttempt::Declined(
                            SourceBackedReferenceDecline::DeclarationSerializationFailed,
                        );
                    };
                    let already_present = locations.iter().any(|loc| loc == &decl_value);
                    if !already_present {
                        locations.push(decl_value);
                    }
                }
            }
        }

        if locations.is_empty() {
            SourceBackedReferenceAttempt::Declined(SourceBackedReferenceDecline::EmptyExactResult)
        } else {
            SourceBackedReferenceAttempt::Exact(locations)
        }
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn references_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let live_provider_result = self.handle_references(params.clone())?;
        let live_provider_count = lsp_location_count(live_provider_result.as_ref());

        #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
        {
            Ok(Some(json!({
                "provider": "references",
                "live_provider_result": live_provider_result,
                "live_provider_count": live_provider_count,
                "compiler_receipt": null,
                "no_live_behavior_change": true,
                "note": "references runtime proof unavailable without workspace semantic queries"
            })))
        }

        #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
        {
            let Some(params) = params else {
                return Ok(Some(json!({
                    "provider": "references",
                    "live_provider_result": live_provider_result,
                    "live_provider_count": live_provider_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "references runtime proof missing request params"
                })));
            };

            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let include_declaration = if let Some(context) = params.get("context") {
                context["includeDeclaration"].as_bool().unwrap_or(true)
            } else {
                true
            };
            let Some((symbol, byte_offset)) = self.references_runtime_symbol(uri, line, character)
            else {
                return Ok(Some(json!({
                    "provider": "references",
                    "live_provider_result": live_provider_result,
                    "live_provider_count": live_provider_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "references runtime proof found no symbol at request position"
                })));
            };

            let compiler_receipt_and_cutover = match route_index_access(self.coordinator()) {
                IndexAccessMode::Full(coordinator) => {
                    let index = coordinator.index();
                    index
                        .with_semantic_queries_for_uri(uri, |file_id, queries| {
                            let ctx = QueryContext::new(file_id, None, Some(byte_offset));
                            let entity_id = queries
                                .symbol_at(file_id, byte_offset)
                                .and_then(|(_, occurrence)| occurrence.entity_id)
                                .or_else(|| {
                                    queries
                                        .definitions(&symbol, &ctx)
                                        .first()
                                        .map(|candidate| candidate.entity_id)
                                })?;
                            let outcome = find_references_live_source_backed(
                                index.as_ref(),
                                &queries,
                                &symbol,
                                entity_id,
                            );
                            let live_cutover =
                                matches!(outcome.result, ReferencesCutoverResult::Exact(_));
                            let mut receipt = outcome.receipt;
                            let compiler_result_count = receipt.new_result.match_count;
                            let behavior_note = if live_cutover && include_declaration {
                                "partial live exact/imported references cutover (includeDeclaration=true)"
                            } else if live_cutover {
                                "partial live exact/imported references cutover"
                            } else {
                                "legacy fallback"
                            };
                            receipt.notes.push(format!(
                                "references runtime proof: live_provider_results={live_provider_count}; compiler_fact_candidates={}; compiler_result_count={}; {behavior_note}",
                                compiler_result_count, compiler_result_count,
                            ));
                            Some((receipt, live_cutover))
                        })
                        .flatten()
                }
                IndexAccessMode::Partial(_) | IndexAccessMode::None => None,
            };
            let (compiler_receipt, live_cutover) = match compiler_receipt_and_cutover {
                Some((receipt, live_cutover)) => (Some(receipt), live_cutover),
                None => (None, false),
            };

            Ok(Some(json!({
                "provider": "references",
                "symbol": symbol,
                "live_provider_result": live_provider_result,
                "live_provider_count": live_provider_count,
                "compiler_receipt": compiler_receipt,
                "no_live_behavior_change": !live_cutover,
                "live_cutover": if live_cutover {
                    Some("partial_exact_imported")
                } else {
                    None
                }
            })))
        }
    }

    #[cfg(all(feature = "workspace", any(test, feature = "expose_lsp_test_api")))]
    fn references_runtime_symbol(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<(String, u32)> {
        let documents = self.documents_guard();
        let doc = self.get_document(&documents, uri)?;
        let offset = self.pos16_to_offset(doc, line, character);
        let symbol = token_under_cursor(&doc.text, line as usize, character as usize)?;
        if symbol.is_empty() {
            return None;
        }
        let byte_offset = u32::try_from(offset).ok()?;
        Some((symbol, byte_offset))
    }

    /// Handle textDocument/documentHighlight request
    pub(crate) fn handle_document_highlight(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Grab an owned `DocumentState` clone under a brief documents-map
            // lock, then drop the guard before doing any analysis (#3396
            // off-lock provider consumption).
            let doc_owned = {
                let documents = self.documents_guard();
                self.get_document(&documents, uri).cloned()
            };
            if let Some(doc) = doc_owned.as_ref() {
                let parsed = doc.current_parsed();
                if let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Guard: if the resolved offset doesn't map back to the
                    // requested line, the character position overflowed the
                    // line boundary (e.g. cursor on an empty line). Return
                    // empty highlights instead of highlighting the wrong line.
                    let (actual_line, _) = self.offset_to_pos16(doc, offset);
                    if actual_line != line {
                        return Ok(Some(json!([])));
                    }

                    // Create document highlight provider
                    let provider = DocumentHighlightProvider::new();

                    // Find all highlights at the position
                    let highlights = provider.find_highlights(ast, &doc.text, offset);

                    if !highlights.is_empty() {
                        let lsp_highlights: Vec<Value> = highlights
                            .iter()
                            .map(|highlight| {
                                let (start_line, start_char) =
                                    self.offset_to_pos16(doc, highlight.location.start);
                                let (end_line, end_char) =
                                    self.offset_to_pos16(doc, highlight.location.end);

                                json!({
                                    "range": {
                                        "start": {
                                            "line": start_line,
                                            "character": start_char,
                                        },
                                        "end": {
                                            "line": end_line,
                                            "character": end_char,
                                        },
                                    },
                                    "kind": highlight.kind as u32,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_highlights)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Non-blocking references handler with fallback
    pub(crate) fn on_references(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let line = params.pointer("/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let ch =
            params.pointer("/position/character").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let text = self.buffer_text(uri).unwrap_or_default();
        let needle = token_under_cursor(&text, line, ch).unwrap_or_default();
        if needle.is_empty() {
            return Ok(serde_json::json!([]));
        }

        // Fallback: search all open docs with word boundary checking
        let docs_snapshot = self.iter_open_buffers();
        let out = search_document_texts_for_references(
            docs_snapshot.iter().map(|(doc_uri, doc_text)| (doc_uri.as_str(), doc_text.as_str())),
            &needle,
            usize::MAX,
        );

        Ok(serde_json::Value::Array(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    // ── ReferencesAnsweringTier unit tests ─────────────────────────────────

    #[test]
    fn answering_tier_as_str_all_variants() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            ReferencesAnsweringTier::SemanticSourceBacked.as_str(),
            "semantic_source_backed"
        );
        assert_eq!(ReferencesAnsweringTier::WorkspaceExact.as_str(), "workspace_exact");
        assert_eq!(ReferencesAnsweringTier::WorkspaceMixed.as_str(), "workspace_mixed");
        assert_eq!(ReferencesAnsweringTier::WorkspaceText.as_str(), "workspace_text");
        assert_eq!(ReferencesAnsweringTier::PartialIndex.as_str(), "partial_index");
        assert_eq!(ReferencesAnsweringTier::OpenDocumentText.as_str(), "open_document_text");
        assert_eq!(ReferencesAnsweringTier::SemanticAnalyzer.as_str(), "semantic_analyzer");
        assert_eq!(ReferencesAnsweringTier::Empty.as_str(), "empty");
        Ok(())
    }

    #[test]
    fn answering_tier_is_source_backed_only_for_semantic_source_backed()
    -> Result<(), Box<dyn Error>> {
        assert!(ReferencesAnsweringTier::SemanticSourceBacked.is_source_backed());
        assert!(!ReferencesAnsweringTier::WorkspaceExact.is_source_backed());
        assert!(!ReferencesAnsweringTier::WorkspaceMixed.is_source_backed());
        assert!(!ReferencesAnsweringTier::WorkspaceText.is_source_backed());
        assert!(!ReferencesAnsweringTier::PartialIndex.is_source_backed());
        assert!(!ReferencesAnsweringTier::OpenDocumentText.is_source_backed());
        assert!(!ReferencesAnsweringTier::SemanticAnalyzer.is_source_backed());
        assert!(!ReferencesAnsweringTier::Empty.is_source_backed());
        Ok(())
    }

    #[test]
    fn answering_tier_receipt_fact_source_is_tier_accurate() -> Result<(), Box<dyn Error>> {
        assert_eq!(ReferencesAnsweringTier::SemanticSourceBacked.fact_source(), "semantic_fact");
        assert_eq!(
            ReferencesAnsweringTier::SemanticSourceBacked.source_backed_state(),
            "semantic_source_backed_ast_index"
        );
        assert_eq!(
            ReferencesAnsweringTier::SemanticSourceBacked.fallback_state(1),
            "live_provider"
        );
        assert_eq!(ReferencesAnsweringTier::SemanticSourceBacked.fallback_state(0), "no_result");

        for tier in [
            ReferencesAnsweringTier::WorkspaceExact,
            ReferencesAnsweringTier::WorkspaceMixed,
            ReferencesAnsweringTier::WorkspaceText,
            ReferencesAnsweringTier::PartialIndex,
            ReferencesAnsweringTier::OpenDocumentText,
            ReferencesAnsweringTier::SemanticAnalyzer,
            ReferencesAnsweringTier::Empty,
        ] {
            assert_eq!(tier.fact_source(), "fallback", "{tier:?} must not claim semantic facts");
            assert!(
                tier.source_backed_state().ends_with("_fallback")
                    || tier.source_backed_state() == "no_references_result",
                "{tier:?} source-backed state must stay fallback-shaped"
            );
            assert_eq!(tier.fallback_state(1), "legacy_provider");
            assert_eq!(tier.fallback_state(0), "no_result");
        }
        Ok(())
    }

    #[test]
    fn source_backed_attempt_receipt_preserves_named_decline_stage() -> Result<(), Box<dyn Error>> {
        let cases = [
            (SourceBackedReferenceDecline::ByteOffsetOutOfRange, "byte_offset", false, 0, None),
            (
                SourceBackedReferenceDecline::WorkspaceIndexUnavailable,
                "workspace_index",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::SemanticQueriesUnavailableForUri,
                "semantic_queries",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::EntityUnresolved {
                    symbol_at_found: true,
                    exact_candidate_count: 2,
                },
                "entity_resolution",
                true,
                2,
                None,
            ),
            (
                SourceBackedReferenceDecline::CutoverNotExact { result_class: "partial" },
                "cutover",
                false,
                0,
                Some("partial"),
            ),
            (
                SourceBackedReferenceDecline::DeclarationAnchorUnavailable,
                "declaration_anchor",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::DeclarationLocationUnavailable,
                "declaration_location",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::DeclarationSerializationFailed,
                "declaration_serialization",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::InitializedLexicalGateRejected,
                "initialized_lexical_gate",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::OccurrenceLocationUnavailable,
                "occurrence_location",
                false,
                0,
                None,
            ),
            (
                SourceBackedReferenceDecline::OccurrenceSerializationFailed,
                "occurrence_serialization",
                false,
                0,
                None,
            ),
            (SourceBackedReferenceDecline::EmptyExactResult, "empty_exact_result", false, 0, None),
        ];
        for (decline, stage, symbol_at_found, exact_candidate_count, cutover_result) in cases {
            let fields = SourceBackedReferenceAttempt::Declined(decline).receipt_fields();
            assert!(fields.attempted);
            assert_eq!(fields.outcome, "declined");
            assert_eq!(fields.decline_stage, Some(stage));
            assert_eq!(fields.symbol_at_found, symbol_at_found);
            assert_eq!(fields.exact_candidate_count, exact_candidate_count);
            assert_eq!(fields.cutover_result, cutover_result);
        }

        let fields = SourceBackedReferenceAttempt::Exact(Vec::new()).receipt_fields();
        assert!(fields.attempted);
        assert_eq!(fields.outcome, "exact");
        assert_eq!(fields.decline_stage, None);
        assert_eq!(fields.cutover_result, Some("exact"));
        Ok(())
    }

    #[test]
    fn classify_combined_tier_by_counts() -> Result<(), Box<dyn Error>> {
        // Both index and text contributed → WorkspaceMixed (heuristic augmentation visible)
        assert_eq!(classify_combined_tier(3, 2), ReferencesAnsweringTier::WorkspaceMixed);
        // Index only → WorkspaceExact (pure index answer)
        assert_eq!(classify_combined_tier(5, 0), ReferencesAnsweringTier::WorkspaceExact);
        // Text only → WorkspaceText (pure text-search answer)
        assert_eq!(classify_combined_tier(0, 4), ReferencesAnsweringTier::WorkspaceText);
        // Neither (degenerate empty call) → WorkspaceText
        assert_eq!(classify_combined_tier(0, 0), ReferencesAnsweringTier::WorkspaceText);
        Ok(())
    }

    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    #[test]
    fn source_backed_references_gate_only_opens_declaration_excluding_variables()
    -> Result<(), Box<dyn Error>> {
        assert!(
            may_use_source_backed_references(false, true),
            "subroutine references keep the existing includeDeclaration=true source-backed path"
        );
        assert!(
            may_use_source_backed_references(true, false),
            "P8 lexical references promotion is limited to includeDeclaration=false"
        );
        assert!(
            !may_use_source_backed_references(true, true),
            "declaration-including lexical references must keep the fallback cascade"
        );
        Ok(())
    }

    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    #[test]
    fn initialized_lexical_gate_requires_assignment_on_declaration_line()
    -> Result<(), Box<dyn Error>> {
        assert!(line_has_initialized_lexical_declaration("my $value = 1;", '$', "value"));
        assert!(line_has_initialized_lexical_declaration("state $value = 1;", '$', "value"));
        assert!(
            !line_has_initialized_lexical_declaration("my $value;", '$', "value"),
            "bare lexical declarations stay outside the selected P8 slice"
        );
        assert!(
            !line_has_initialized_lexical_declaration("my $other = $value;", '$', "value"),
            "RHS usages do not make the target variable's declaration initialized"
        );
        Ok(())
    }

    #[test]
    fn should_skip_text_reference_match_omits_variable_declarations_when_requested()
    -> Result<(), Box<dyn Error>> {
        let line = "my $total = 1;";
        let match_start = line.find("total").ok_or("missing total match")?;

        assert!(
            should_skip_text_reference_match(line, match_start, Some('$'), false),
            "includeDeclaration=false must omit lexical declaration matches"
        );
        assert!(
            !should_skip_text_reference_match(line, match_start, Some('$'), true),
            "includeDeclaration=true must keep declaration matches"
        );
        assert!(
            !should_skip_text_reference_match(line, match_start, None, false),
            "subroutine/bareword text matches are not variable declarations"
        );

        Ok(())
    }

    #[test]
    fn should_skip_text_reference_match_keeps_initializer_rhs_usages() -> Result<(), Box<dyn Error>>
    {
        let line = "my $other = $total;";
        let match_start = line.find("total").ok_or("missing total match")?;

        assert!(
            !should_skip_text_reference_match(line, match_start, Some('$'), false),
            "RHS usages inside a declaration statement are still references"
        );

        Ok(())
    }

    #[test]
    fn should_skip_text_reference_match_omits_variable_list_declaration_targets()
    -> Result<(), Box<dyn Error>> {
        let line = "for my ($first, $total) {";
        let match_start = line.find("total").ok_or("missing total match")?;

        assert!(
            should_skip_text_reference_match(line, match_start, Some('$'), false),
            "declaration targets inside variable lists must be omitted"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    fn make_document_index_stale(
        server: &crate::runtime::LspServer,
        uri: &str,
        text: &str,
    ) -> Result<(), Box<dyn Error>> {
        server.test_apply_did_open(uri, text, 1)?;
        server.test_index_file_in_building_state(uri, text).map_err(std::io::Error::other)?;
        server.test_simulate_indexing_complete();
        server.test_replace_document_without_index(uri, text, 2).map_err(std::io::Error::other)?;

        assert!(
            server.workspace_index_stale_for_document(uri),
            "test setup must leave the open document newer than the workspace index"
        );

        Ok(())
    }

    // ── Handler trace tests (--lib reachable) ────────────────────────────

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_references_records_none_index_state_when_open_document_index_is_stale()
    -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);
        let uri = "file:///test/stale-references.pl";
        let text = "my $value = 1;\nmy $other = $value;\n";

        make_document_index_stale(&server, uri, text)?;

        server.test_handle_references(Some(serde_json::json!({
            "textDocument": {"uri": uri, "version": 2},
            "position": {"line": 0, "character": 3},
            "context": {"includeDeclaration": true}
        })))?;

        let explanation = server
            .handle_execute_command(Some(serde_json::json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": "references"}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(serde_json::Value::as_object)
            .ok_or("missing request_receipt")?;

        assert_eq!(
            receipt.get("index_state").and_then(serde_json::Value::as_str),
            Some("none"),
            "stale current-document index must downgrade references index access"
        );

        Ok(())
    }

    #[test]
    fn handle_references_inner_call_presence_observer_rejects_stale_request_version()
    -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;

        let server = LspServer::new();
        let uri = "file:///test/references-version.pl";
        let text = "my $value = 1;\nmy $other = $value;\n";

        server.test_apply_did_open(uri, text, 3)?;

        let err = match server.test_handle_references(Some(serde_json::json!({
            "textDocument": {"uri": uri, "version": 2},
            "position": {"line": 0, "character": 3},
            "context": {"includeDeclaration": true}
        }))) {
            Ok(value) => return Err(format!("expected stale-version error, got {value:?}").into()),
            Err(err) => err,
        };

        assert_eq!(
            err.code, CONTENT_MODIFIED,
            "references request with stale textDocument.version must hit ensure_latest"
        );
        assert_eq!(
            err.message, "Document changed before request executed",
            "references stale-version rejection must preserve the ContentModified payload"
        );

        Ok(())
    }

    #[test]
    fn handle_references_workspace_variable_answering_tier_in_trace() -> Result<(), Box<dyn Error>>
    {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);

        // Open a document with a known local scalar variable; after didOpen the
        // workspace index contains the file so the cascade uses a workspace tier.
        let uri = "file:///test/scalar.pl";
        let text = "my $value = 1;\nmy $other = $value;\n";
        server.test_apply_did_open(uri, text, 1)?;

        // Position cursor on the `$` sigil of `$value` on line 0
        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 3},
            "context": {"includeDeclaration": true}
        });
        server.test_handle_references(Some(params))?;

        // Read back the trace via explain_provider_decision
        let explanation = server
            .handle_execute_command(Some(serde_json::json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": "references"}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(serde_json::Value::as_object)
            .ok_or("missing request_receipt")?;

        // `answering_tier` must be present in the trace (the exact tier depends
        // on workspace index state; what matters is the field is emitted and
        // `source_backed` is accurate — `$value` is not compiler-source-backed).
        let tier = receipt
            .get("answering_tier")
            .and_then(serde_json::Value::as_str)
            .ok_or("answering_tier field missing from references trace")?;
        assert!(!tier.is_empty(), "answering_tier must be a non-empty string; got {tier:?}");
        assert_eq!(
            receipt.get("source_backed").and_then(serde_json::Value::as_bool),
            Some(false),
            "lexical $value must not be source_backed"
        );
        // index_state and latency_us must be present (new fields from CORRECTION v2)
        let index_state = receipt
            .get("index_state")
            .and_then(serde_json::Value::as_str)
            .ok_or("index_state field missing from references trace")?;
        assert!(
            ["full", "partial", "none"].contains(&index_state),
            "index_state must be full|partial|none; got {index_state:?}"
        );
        assert!(
            receipt.get("latency_us").and_then(serde_json::Value::as_u64).is_some(),
            "latency_us field must be present and numeric"
        );
        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_references_lexical_variable_without_declaration_uses_source_backed_tier()
    -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);

        let uri = "file:///test/scalar-no-decl.pl";
        let text = "my $value = 1;\nmy $other = $value;\n";
        server.test_apply_did_open(uri, text, 1)?;

        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 12},
            "context": {"includeDeclaration": false}
        });
        let result = server.test_handle_references(Some(params))?;

        let explanation = server
            .handle_execute_command(Some(serde_json::json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": "references"}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(serde_json::Value::as_object)
            .ok_or("missing request_receipt")?;

        assert_eq!(
            receipt.get("answering_tier").and_then(serde_json::Value::as_str),
            Some("semantic_source_backed"),
            "includeDeclaration=false lexical references must use the P8 source-backed tier"
        );
        assert_eq!(
            receipt.get("source_backed").and_then(serde_json::Value::as_bool),
            Some(true),
            "P8 lexical references result must be recorded as source-backed"
        );
        assert_eq!(
            receipt.get("source_backed_attempted").and_then(serde_json::Value::as_bool),
            Some(true),
            "source-backed lexical references must record an attempted semantic path"
        );
        assert_eq!(
            receipt.get("source_backed_outcome").and_then(serde_json::Value::as_str),
            Some("exact"),
            "source-backed lexical references must record the exact attempt outcome"
        );
        assert_eq!(
            receipt.get("source_backed_decline_stage"),
            Some(&serde_json::Value::Null),
            "exact source-backed references must not report a decline stage"
        );
        assert_eq!(
            receipt.get("include_declaration").and_then(serde_json::Value::as_bool),
            Some(false),
            "provider trace must record includeDeclaration=false"
        );

        let locations = result
            .as_ref()
            .and_then(serde_json::Value::as_array)
            .ok_or("textDocument/references must return an array")?;
        if locations.is_empty() {
            return Err("P8 lexical references cutover must return at least one usage".into());
        }
        for location in locations {
            let line = location
                .pointer("/range/start/line")
                .and_then(serde_json::Value::as_u64)
                .ok_or("missing location start line")?;
            if line == 0 {
                return Err(format!(
                    "includeDeclaration=false must not return declaration location: {locations:?}"
                )
                .into());
            }
        }

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_references_bare_lexical_without_initializer_keeps_fallback_tier()
    -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);

        let uri = "file:///test/scalar-bare-decl.pl";
        let text = "my $value;\n$value = 1;\n";
        server.test_apply_did_open(uri, text, 1)?;

        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 1},
            "context": {"includeDeclaration": false}
        });
        server.test_handle_references(Some(params))?;

        let explanation = server
            .handle_execute_command(Some(serde_json::json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": "references"}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(serde_json::Value::as_object)
            .ok_or("missing request_receipt")?;

        assert_ne!(
            receipt.get("answering_tier").and_then(serde_json::Value::as_str),
            Some("semantic_source_backed"),
            "bare lexical declarations are not in the selected initialized P8 slice"
        );
        assert_eq!(
            receipt.get("source_backed").and_then(serde_json::Value::as_bool),
            Some(false),
            "bare lexical fallback must not be recorded as source-backed"
        );
        assert_eq!(
            receipt.get("source_backed_attempted").and_then(serde_json::Value::as_bool),
            Some(true),
            "eligible bare lexical requests must record the attempted semantic path"
        );
        assert_eq!(
            receipt.get("source_backed_outcome").and_then(serde_json::Value::as_str),
            Some("declined"),
            "bare lexical requests must expose the semantic decline rather than generic None"
        );
        assert_eq!(
            receipt.get("source_backed_decline_stage").and_then(serde_json::Value::as_str),
            Some("initialized_lexical_gate"),
            "bare lexical requests must identify the initialized lexical gate as first failure"
        );

        Ok(())
    }

    #[test]
    fn handle_references_empty_tier_when_cursor_on_whitespace() -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);

        let uri = "file:///test/whitespace.pl";
        // A blank line — cursor on it yields no token so no workspace lookup and
        // no semantic references → Empty tier.
        let text = "my $x = 1;\n\nmy $y = $x;\n";
        server.test_apply_did_open(uri, text, 1)?;

        // Position cursor on the blank line (line 1, character 0)
        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 0},
            "context": {"includeDeclaration": true}
        });
        server.test_handle_references(Some(params))?;

        let explanation = server
            .handle_execute_command(Some(serde_json::json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": "references"}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(serde_json::Value::as_object)
            .ok_or("missing request_receipt")?;

        // Cursor on a blank line: no token → workspace finds nothing → semantic
        // analyzer finds nothing → Empty tier.
        assert_eq!(
            receipt.get("answering_tier").and_then(serde_json::Value::as_str),
            Some("empty"),
            "blank-line cursor must report empty tier"
        );
        assert_eq!(
            receipt.get("source_backed").and_then(serde_json::Value::as_bool),
            Some(false),
            "empty tier must not be source_backed"
        );
        // index_state and latency_us must be present for empty tier too
        assert!(
            receipt.get("index_state").and_then(serde_json::Value::as_str).is_some(),
            "index_state field must be present even for empty tier"
        );
        assert!(
            receipt.get("latency_us").and_then(serde_json::Value::as_u64).is_some(),
            "latency_us field must be present and numeric"
        );
        Ok(())
    }

    fn location_start(location: &Value) -> Result<(u64, u64), Box<dyn Error>> {
        let line = location["range"]["start"]["line"].as_u64().ok_or("missing start line")?;
        let character =
            location["range"]["start"]["character"].as_u64().ok_or("missing start character")?;
        Ok((line, character))
    }

    #[test]
    fn search_document_texts_for_references_boundary_discriminator_input_that_hits_the_boundary_needle_is_empty_or_cap_zero_returns_empty()
    -> Result<(), Box<dyn Error>> {
        let docs = [("file:///refs.pl", "$var\n")];

        assert_eq!(
            search_document_texts_for_references(
                docs.iter().map(|(uri, text)| (*uri, *text)),
                "",
                10,
            )
            .len(),
            0,
            "empty needle must not produce references",
        );

        Ok(())
    }

    #[test]
    fn search_document_texts_for_references_boundary_discriminator_input_that_hits_the_boundary_cap_zero_returns_empty()
    -> Result<(), Box<dyn Error>> {
        let docs = [("file:///refs.pl", "$var\n")];

        assert_eq!(
            search_document_texts_for_references(
                docs.iter().map(|(uri, text)| (*uri, *text)),
                "var",
                0,
            )
            .len(),
            0,
            "zero cap must not produce references",
        );

        Ok(())
    }

    #[test]
    fn search_document_texts_for_references_boundary_discriminator_input_that_hits_the_boundary_out_len_reaches_cap_stops_scan()
    -> Result<(), Box<dyn Error>> {
        let docs = [("file:///refs.pl", "$var $var $var\n")];

        assert_eq!(
            search_document_texts_for_references(
                docs.iter().map(|(uri, text)| (*uri, *text)),
                "var",
                2,
            )
            .len(),
            2,
            "cap-limited scan must stop at two references",
        );

        Ok(())
    }

    #[test]
    fn search_document_texts_for_references_keeps_word_boundaries() -> Result<(), Box<dyn Error>> {
        let docs = [("file:///refs.pl", "my $var = 1;\nmy $variant = $var;\n")];

        let refs = search_document_texts_for_references(
            docs.iter().map(|(uri, text)| (*uri, *text)),
            "var",
            10,
        );
        if refs.len() != 2 {
            return Err(format!("expected 2 references, got {}", refs.len()).into());
        }

        for location in &refs {
            if location_start(location)? == (1, 4) {
                return Err("embedded match in $variant must not be reported".into());
            }
        }

        Ok(())
    }

    #[test]
    fn search_document_texts_for_references_reports_utf16_columns() -> Result<(), Box<dyn Error>> {
        let docs = [("file:///refs.pl", "my $heart = \"♥\"; $heart\n")];

        let refs = search_document_texts_for_references(
            docs.iter().map(|(uri, text)| (*uri, *text)),
            "heart",
            10,
        );
        if refs.len() != 2 {
            return Err(format!("expected 2 references, got {}", refs.len()).into());
        }

        let starts: Vec<_> = refs.iter().map(location_start).collect::<Result<_, _>>()?;
        if starts != vec![(0, 4), (0, 18)] {
            return Err(format!("unexpected UTF-16 starts: {starts:?}").into());
        }

        Ok(())
    }

    // ── includeDeclaration=true call-observation test (ripr+ gate) ──────────

    /// Drive `live_source_backed_reference_locations` (lines 862–863) through the
    /// real `textDocument/references` handler with `includeDeclaration=true`.
    ///
    /// Before #2673 the source-backed path bailed early with `return None` when
    /// `include_declaration==true`, causing every such request to fall through to
    /// the workspace-index tier and leaving the declaration off the result.
    ///
    /// This test proves the fix is live end-to-end:
    /// - The `answering_tier` in the provider-decision trace must be
    ///   `semantic_source_backed` (proving the early bail was removed).
    /// - The result returned to the client must contain the declaration site
    ///   (the `sub target` definition line), proving the append logic runs.
    ///
    /// The test would FAIL if the fix were reverted: the early bail would cause
    /// `live_source_backed_reference_locations` to return `None`, the provider
    /// would fall through to the workspace-index tier, `answering_tier` would be
    /// `workspace_exact` or `workspace_text` rather than `semantic_source_backed`,
    /// and the declaration-line assertion would be unreliable under both tiers.
    #[test]
    fn handle_references_include_declaration_true_reaches_source_backed_tier_and_appends_declaration()
    -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);

        // A package with a named sub definition followed by a call site.
        // This is the minimal fixture that triggers the semantic-source-backed tier
        // because the workspace index resolves `target` to the `sub target`
        // definition via semantic queries.
        let uri = "file:///test/incl_decl.pl";
        let text = concat!(
            "package InclDecl;\n",
            "\n",
            "sub target {\n", // line 2  — declaration site
            "    return 1;\n",
            "}\n",
            "\n",
            "sub caller {\n",
            "    target();\n",           // line 7  — first call site
            "    InclDecl::target();\n", // line 8  — second call site
            "}\n",
            "\n",
            "1;\n",
        );
        server.test_apply_did_open(uri, text, 1)?;

        // Position cursor on the bare `target` inside `caller` (line 7, col 4).
        // `includeDeclaration: true` is the VS Code default.
        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 7, "character": 4},
            "context": {"includeDeclaration": true}
        });

        let result = server.test_handle_references(Some(params))?;

        // ── Tier assertion ────────────────────────────────────────────────────
        // Read the provider-decision trace; the answering tier must be
        // `semantic_source_backed`.  If the early bail (pre-fix) were present
        // `live_source_backed_reference_locations` would return `None` and the
        // provider would fall through to `workspace_exact` / `workspace_text`.
        let explanation = server
            .handle_execute_command(Some(serde_json::json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": "references"}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(serde_json::Value::as_object)
            .ok_or("missing request_receipt")?;

        assert_eq!(
            receipt.get("answering_tier").and_then(serde_json::Value::as_str),
            Some("semantic_source_backed"),
            "includeDeclaration=true must reach the semantic_source_backed tier, \
             not fall through to a lower-fidelity workspace tier (pre-fix bail would produce workspace_exact)"
        );
        assert_eq!(
            receipt.get("include_declaration").and_then(serde_json::Value::as_bool),
            Some(true),
            "provider trace must record include_declaration=true"
        );

        // ── Declaration-append assertion ──────────────────────────────────────
        // The result must contain the `sub target` declaration line (line 2).
        // This proves that lines 940–953 (the dedup-append block) executed.
        let locations = result
            .as_ref()
            .and_then(serde_json::Value::as_array)
            .ok_or("textDocument/references must return a non-null array")?;

        let decl_line: u64 = 2; // `sub target {` is at line index 2 (0-based)
        let contains_decl = locations.iter().any(|loc| {
            loc.pointer("/range/start/line").and_then(serde_json::Value::as_u64) == Some(decl_line)
        });
        assert!(
            contains_decl,
            "includeDeclaration=true result must contain the declaration line ({decl_line}); \
             got locations: {locations:?}"
        );

        Ok(())
    }

    /// Prove that `live_source_backed_reference_locations` returns `None` when
    /// the cursor is on an identifier that has no definition in the semantic
    /// model.
    ///
    /// This exercises two branches that are not reachable when a definition IS
    /// found:
    ///
    /// - The `_ => None` arm of the `match exact_candidates.as_slice()` block
    ///   (0 candidates because `definitions("undefined_fn")` returns nothing).
    /// - The `?` propagation that exits the outer closure with `None`, causing
    ///   `live_source_backed_reference_locations` to return `None` and the
    ///   provider to fall back to the workspace-index tier.
    ///
    /// The test verifies that the request completes without error — the
    /// semantic tier gracefully yields to the lower-fidelity fallback.
    #[test]
    fn handle_references_undefined_symbol_falls_back_gracefully() -> Result<(), Box<dyn Error>> {
        use crate::runtime::LspServer;
        use parking_lot::Mutex;
        use std::io::Cursor;
        use std::sync::Arc;

        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        let server = LspServer::with_output(output);

        // A minimal fixture where `undefined_fn()` is called but never defined.
        // When the cursor lands on `undefined_fn`, the semantic model has no
        // entity for it: `definitions("undefined_fn")` returns 0 candidates,
        // so `exact_candidates` is empty, `_ => None` is matched, and
        // `entity_id` is `None`.  The `?` on that `None` exits the closure,
        // `with_semantic_queries_for_uri` returns `Some(None)` which `.flatten()`
        // resolves to `None`, and `live_source_backed_reference_locations`
        // returns `None` — falling back to the workspace-index tier.
        let uri = "file:///test/no_decl.pl";
        let text = concat!(
            "package NoDecl;\n",
            "\n",
            "sub caller {\n",
            "    undefined_fn();\n", // line 3  — call with no local definition
            "}\n",
            "\n",
            "1;\n",
        );
        server.test_apply_did_open(uri, text, 1)?;

        // Position cursor on `undefined_fn` (line 3, character 4).
        let params = serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 3, "character": 4},
            "context": {"includeDeclaration": true}
        });

        // The request must complete without error.  Coverage proof: the semantic
        // tier returns `None` here, exercising the `_ => None` arm (line 957)
        // and the `?` propagation path (line 960) in
        // `live_source_backed_reference_locations`.
        let _result = server.test_handle_references(Some(params))?;

        Ok(())
    }
}
