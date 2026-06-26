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
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};

#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};

static QUALIFIED_NAME_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

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

    /// Returns `true` only when compiler source-backed facts answered this request.
    pub(crate) fn is_source_backed(self) -> bool {
        matches!(self, Self::SemanticSourceBacked)
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
    ) {
        let Some(context) = context else {
            return;
        };
        let result_count = lsp_location_count(result);
        let (decision, reason, fallback_state) = if result_count == 0 {
            ("fallback", "no_result", "no_result")
        } else {
            ("acted", "live_provider_result", "live_provider")
        };
        // confidence is "high" when compiler source-backed facts answered, else "low"
        let confidence = if tier.is_source_backed() { "high" } else { "low" };
        // source_backed_result_count is the total result count only for source-backed answers
        let source_backed_result_count: usize =
            if tier.is_source_backed() { result_count } else { 0 };

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
                "fact_source": "navigation_provider",
                "confidence": confidence,
                "freshness": "fresh",
                "source_backed": tier.is_source_backed(),
                "source_backed_state": "not_proven_by_provider_trace",
                "answering_tier": tier.as_str(),
                "index_state": index_state,
                "latency_us": latency_us,
                "fallback_state": fallback_state,
                "dynamic_boundary": false,
                "trace_only_no_live_behavior_change": true,
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
        let (result, tier, index_state, index_result_count, text_result_count, latency_us) =
            self.handle_references_inner(params)?;
        self.record_references_provider_decision_trace(
            trace_context.as_ref(),
            result.as_ref(),
            tier,
            index_state,
            index_result_count,
            text_result_count,
            latency_us,
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
        (Option<Value>, ReferencesAnsweringTier, &'static str, usize, usize, u128),
        JsonRpcError,
    > {
        let start = Instant::now();
        let deadline = reference_search_deadline();
        let cap = references_cap();

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let include_declaration = if let Some(context) = params.get("context") {
                context["includeDeclaration"].as_bool().unwrap_or(true)
            } else {
                true
            };

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
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
                    self.wait_for_index_ready_if_building();

                    // Check index state and use appropriate search strategy
                    #[cfg(feature = "workspace")]
                    {
                        let access_mode = route_index_access(self.coordinator());
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
                                    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                                    if let Some(mut live_locations) = self
                                        .live_source_backed_reference_locations(
                                            uri,
                                            symbol_key.name.as_ref(),
                                            offset,
                                            include_declaration,
                                        )
                                    {
                                        live_locations.truncate(cap);
                                        tracing::debug!(
                                            count = live_locations.len(),
                                            elapsed = ?start.elapsed(),
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
                                        ));
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
                                        ));
                                    }

                                    // Enhanced fallback: always search for both qualified and unqualified references
                                    // Snapshot only (uri, text) to minimize cloning overhead - we don't need
                                    // AST, rope, or other DocumentState fields for text search
                                    let docs_snapshot: Vec<(String, String)> = documents
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.text.clone()))
                                        .collect();

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
                                                            ));
                                                        }
                                                    }

                                                    // Fallback: scan open documents for qualified name references
                                                    // Snapshot only (uri, text) to minimize cloning overhead
                                                    let docs_snapshot: Vec<(String, String)> =
                                                        documents
                                                            .iter()
                                                            .map(|(k, v)| {
                                                                (k.clone(), v.text.clone())
                                                            })
                                                            .collect();

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
                                            ));
                                        }
                                    }
                                }

                                tracing::debug!(reason, "References: using same-file fallback");
                                if !needle.is_empty() {
                                    let open_doc_locations = search_document_texts_for_references(
                                        documents.iter().map(|(doc_uri, doc)| {
                                            (doc_uri.as_str(), doc.text.as_str())
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
        ))
    }

    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    fn live_source_backed_reference_locations(
        &self,
        uri: &str,
        symbol: &str,
        byte_offset: usize,
        include_declaration: bool,
    ) -> Option<Vec<Value>> {
        if include_declaration {
            return None;
        }

        let byte_offset = u32::try_from(byte_offset).ok()?;
        let workspace_index = self.workspace_index()?;
        let outcome = workspace_index
            .with_semantic_queries_for_uri(uri, |file_id, queries| {
                let ctx = QueryContext::new(file_id, None, Some(byte_offset));
                let entity_id = queries
                    .symbol_at(file_id, byte_offset)
                    .and_then(|(_, occurrence)| occurrence.entity_id)
                    .or_else(|| {
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
                                    )
                                    && workspace_index
                                        .semantic_anchor_wire_location(candidate.anchor_id)
                                        .is_some()
                            })
                            .collect();
                        match exact_candidates.as_slice() {
                            [candidate] => Some(candidate.entity_id),
                            _ => None,
                        }
                    })?;
                Some(find_references_live_source_backed(
                    workspace_index.as_ref(),
                    &queries,
                    symbol,
                    entity_id,
                ))
            })
            .flatten()?;

        let ReferencesCutoverResult::Exact(occurrences) = outcome.result else {
            return None;
        };

        let mut locations = Vec::with_capacity(occurrences.len());
        for occurrence in occurrences {
            let wire_location =
                workspace_index.semantic_anchor_wire_location(occurrence.anchor_id)?;
            let location: lsp_types::Location = wire_location.into();
            locations.push(serde_json::to_value(location).ok()?);
        }

        if locations.is_empty() { None } else { Some(locations) }
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
                            let live_cutover = !include_declaration
                                && matches!(outcome.result, ReferencesCutoverResult::Exact(_));
                            let mut receipt = outcome.receipt;
                            let compiler_result_count = receipt.new_result.match_count;
                            let behavior_note = if live_cutover {
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

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
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

    // ── Handler trace tests (--lib reachable) ────────────────────────────

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
}
