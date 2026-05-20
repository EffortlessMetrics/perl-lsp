//! Workspace-level operations
//!
//! Handles workspace symbols, configuration, file watching, and edits.
//!
//! # Lifecycle-Aware Behavior
//!
//! Uses the routing module for state-aware dispatch:
//! - **Ready state**: Full workspace index search with cooperative yielding
//! - **Building/Degraded state**: Open document search only (partial results)

use super::*;
#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
use crate::runtime::workspace_progress::{
    send_index_ready_notification, send_progress_begin, send_progress_create, send_progress_end,
    send_progress_report,
};
use crate::state::workspace_symbol_cap;
use perl_module::path::file_path_to_module_name;
use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};
#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{DegradationReason, EarlyExitReason, ResourceKind, SymbolKind};
#[cfg(feature = "workspace")]
use perl_parser_core::source_file::{is_perl_source_path, is_perl_source_uri};
#[cfg(any(test, feature = "expose_lsp_test_api"))]
use perl_semantic_facts::{
    Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind, ProviderFallbackState,
};
use perl_workspace::folder::extract_workspace_folder_change;
#[cfg(feature = "workspace")]
use perl_workspace::ignore::is_skipped_dir_name;
use std::collections::HashSet;
#[cfg(feature = "workspace")]
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "workspace")]
use std::time::Instant;
#[cfg(feature = "workspace")]
use url::Url;

#[cfg(feature = "workspace")]
mod configuration_response;
#[cfg(feature = "workspace")]
mod text_decode;

const WORKSPACE_CONFIGURATION_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
// Note: WalkDir logic has been extracted to super::file_discovery.
// These helper functions are retained for potential future use by
// other workspace operations (e.g., file watcher filtering).
#[cfg(feature = "workspace")]
#[allow(dead_code)]
fn is_perl_source_file(path: &Path) -> bool {
    is_perl_source_path(path)
}

#[cfg(feature = "workspace")]
#[allow(dead_code)]
fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    is_skipped_dir_name(&entry.file_name().to_string_lossy())
}

/// Returns `true` when an I/O error represents a permission-denied condition.
///
/// Covers both the portable `ErrorKind::PermissionDenied` and the Windows
/// `ERROR_ACCESS_DENIED` code (os error 5), which may surface as
/// `ErrorKind::Uncategorized` on older Rust toolchains.
#[cfg(feature = "workspace")]
fn is_permission_denied_error(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        return true;
    }
    // Windows ERROR_ACCESS_DENIED = os error 5
    #[cfg(windows)]
    if e.raw_os_error() == Some(5) {
        return true;
    }
    false
}

#[cfg(feature = "workspace")]
use text_decode::read_text_with_encoding_fallback;

#[cfg(feature = "workspace")]
fn read_watched_file_content(uri: &str, purpose: &str) -> Option<String> {
    uri_to_fs_path(uri).and_then(|path| match read_text_with_encoding_fallback(&path) {
        Ok(content) => Some(content),
        Err(e) => {
            tracing::debug!("Failed to read file for {} ({}): {}", purpose, path.display(), e);
            None
        }
    })
}

/// RAII guard that clears the `indexing_in_progress` flag on drop.
///
/// Ensures the flag is always cleared, even if the indexing thread panics.
#[cfg(feature = "workspace")]
struct IndexingGuard(Arc<AtomicBool>);

#[cfg(feature = "workspace")]
impl Drop for IndexingGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn parse_configuration_response_id(value: &Value) -> Option<i64> {
    if let Some(id) = value.as_i64() {
        return Some(id);
    }

    value.as_str().and_then(|raw| raw.parse::<i64>().ok())
}

impl LspServer {
    /// Request `workspace/configuration` for each workspace folder (if supported).
    pub(crate) fn request_workspace_configuration_for_folders(&self) {
        if !self.client_capabilities.lock().workspace_configuration_support {
            tracing::debug!("Client does not support workspace/configuration; using local config");
            return;
        }

        let now = std::time::Instant::now();

        let folder_uris: Vec<String> =
            self.workspace_folders.lock().iter().map(|folder| folder.uri.clone()).collect();
        if folder_uris.is_empty() {
            return;
        }

        let mut items: Vec<Value> = vec![json!({ "section": "perl" })];
        items.extend(folder_uris.iter().map(|uri| json!({ "scopeUri": uri, "section": "perl" })));
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        if let Err(error) = self.outbound.send_request(
            request_id,
            "workspace/configuration",
            json!({ "items": items }),
        ) {
            tracing::warn!(%error, "Failed to send workspace/configuration request");
            return;
        }

        let mut pending = self.pending_workspace_configuration_requests.lock();

        // Count cap backstop: keep at most 10 pending requests to prevent unbounded growth
        // even if client responses are slow or missing.
        if pending.len() >= 10 {
            let to_remove = pending.len() - 9;
            let mut entries: Vec<_> =
                pending.iter().map(|(id, req)| (*id, req.created_at)).collect();
            entries.sort_by_key(|(_, created_at)| *created_at);
            for (id, _) in entries.iter().take(to_remove) {
                tracing::debug!(
                    request_id = *id,
                    "Dropping excess workspace/configuration request (count cap)"
                );
                pending.remove(id);
            }
        }

        if !pending.is_empty() {
            tracing::debug!(
                superseded_requests = pending.len(),
                "Dropping older workspace/configuration requests in favor of latest snapshot"
            );
            pending.clear();
        }
        pending.insert(
            request_id,
            PendingWorkspaceConfigurationRequest {
                folder_uris,
                includes_global_item: true,
                created_at: now,
            },
        );
    }

    /// Apply a `workspace/configuration` response for a previously sent request.
    pub(crate) fn handle_client_response(&self, params: Option<Value>) {
        let Some(params) = params else {
            return;
        };
        let Some(id) = params.get("id").and_then(parse_configuration_response_id) else {
            return;
        };

        let maybe_pending = self.pending_workspace_configuration_requests.lock().remove(&id);
        let Some(pending) = maybe_pending else {
            return;
        };
        let response_age = std::time::Instant::now().saturating_duration_since(pending.created_at);
        if response_age > WORKSPACE_CONFIGURATION_REQUEST_TIMEOUT {
            tracing::warn!(
                request_id = id,
                age_ms = response_age.as_millis(),
                "Ignoring stale workspace/configuration response"
            );
            return;
        }

        if params.get("error").is_some() {
            tracing::debug!(
                request_id = id,
                "workspace/configuration request failed; keeping TOML/default config"
            );
            return;
        }

        let Some(results) = params.get("result").and_then(Value::as_array) else {
            tracing::warn!(
                request_id = id,
                "workspace/configuration response was not an array; keeping TOML/default config"
            );
            return;
        };
        let mut folders = self.workspace_folders.lock();
        configuration_response::apply_workspace_configuration_results(
            &mut folders,
            &pending.folder_uris,
            pending.includes_global_item,
            results,
            id,
        );
    }

    /// Handle workspace/symbol request (v2 implementation with lifecycle-aware dispatch)
    ///
    /// Uses routing helper for state-aware behavior:
    /// - **Ready state**: Full workspace index search with cooperative yielding
    /// - **Building/Degraded state**: Query partial index first; fall through to open-doc
    ///   search only when the partial index is also empty (Gap 2 fix, issue #4152)
    pub(super) fn handle_workspace_symbols_v2(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query"))
            .and_then(|q| q.as_str())
            .unwrap_or("")
            .trim();
        let cap = workspace_symbol_cap();

        tracing::debug!(query, cap, "Workspace symbol search v2");

        // Use routing helper for lifecycle-aware dispatch
        #[cfg(feature = "workspace")]
        {
            let access_mode = route_index_access(self.coordinator());

            match access_mode {
                IndexAccessMode::Full(coordinator) => {
                    // Full query path: use workspace index
                    let mut symbols = coordinator.index().search_source_symbols(query);
                    symbols.extend(coordinator.index().search_generated_workspace_symbols(query));

                    // Convert to LSP format with yielding and result cap
                    let lsp_symbols: Vec<LspWorkspaceSymbol> = symbols
                        .iter()
                        .take(cap)
                        .enumerate()
                        .map(|(i, sym)| {
                            // Cooperative yield every 64 symbols
                            if i & 0x3f == 0 {
                                std::thread::yield_now();
                            }
                            sym.into()
                        })
                        .collect();
                    let generated_pilot_count = lsp_symbols
                        .iter()
                        .filter(|symbol| {
                            workspace_symbol_has_generated_label(
                                &symbol.name,
                                symbol.container_name.as_deref(),
                            )
                        })
                        .count();

                    if !lsp_symbols.is_empty() {
                        tracing::debug!(
                            count = lsp_symbols.len(),
                            "Workspace symbol: returned results from index (Ready state)"
                        );
                        self.record_workspace_symbols_provider_decision_trace(
                            query,
                            lsp_symbols.len(),
                            WorkspaceSymbolsTraceKind::SourceBackedReadyIndex,
                            generated_pilot_count,
                        );
                        return Ok(Some(json!(lsp_symbols)));
                    }
                    // If index is empty, fall through to open-doc search
                }
                IndexAccessMode::Partial(reason) => {
                    // Building/Degraded: still query the partial index so users get
                    // results from files already scanned.  Fall through to the
                    // open-doc path only when the partial index is also empty.
                    tracing::debug!(reason, "Workspace symbol: querying partial index");
                    if let Some(coordinator) = self.coordinator() {
                        let symbols = coordinator.index().search_source_symbols(query);
                        let lsp_symbols: Vec<LspWorkspaceSymbol> = symbols
                            .iter()
                            .take(cap)
                            .enumerate()
                            .map(|(i, sym)| {
                                if i & 0x3f == 0 {
                                    std::thread::yield_now();
                                }
                                sym.into()
                            })
                            .collect();
                        if !lsp_symbols.is_empty() {
                            tracing::debug!(
                                count = lsp_symbols.len(),
                                "Workspace symbol: returned results from partial index"
                            );
                            self.record_workspace_symbols_provider_decision_trace(
                                query,
                                lsp_symbols.len(),
                                WorkspaceSymbolsTraceKind::PartialIndexFallback,
                                0,
                            );
                            return Ok(Some(json!(lsp_symbols)));
                        }
                    }
                    tracing::debug!(
                        reason,
                        "Workspace symbol: partial index empty, falling back to open-docs"
                    );
                }
                IndexAccessMode::None => {
                    tracing::debug!(
                        "Workspace symbol: no workspace feature, using open-doc fallback"
                    );
                }
            }
        }

        // Fallback/degraded path: search open documents only
        self.search_open_documents_for_symbols(query, cap)
    }

    /// Search only open documents for symbols (degraded/fallback path)
    #[cfg(feature = "workspace")]
    fn search_open_documents_for_symbols(
        &self,
        query: &str,
        cap: usize,
    ) -> Result<Option<Value>, JsonRpcError> {
        let docs_snapshot: Vec<(String, String, Option<Arc<perl_parser::ast::Node>>)> = {
            let documents = self.documents.lock();
            documents.iter().map(|(k, v)| (k.clone(), v.text.clone(), v.ast.clone())).collect()
        };

        let mut provider =
            perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolsProvider::new();
        let mut source_map = std::collections::HashMap::new();
        let mut text_fallback_symbols = Vec::new();

        for (i, (uri, text, ast)) in docs_snapshot.iter().enumerate() {
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }
            source_map.insert(uri.clone(), text.clone());
            if let Some(ast) = ast {
                provider.index_document(uri, ast, text);
            } else {
                text_fallback_symbols.extend(self.extract_text_based_symbols(text, uri, query));
            }
        }

        let mut candidates = self.symbol_index.lock().search_prefix(query);
        if candidates.is_empty() && !query.is_empty() {
            candidates = self.symbol_index.lock().search_fuzzy(query);
        }
        let mut dedup = HashSet::new();
        candidates.retain(|candidate| dedup.insert(candidate.clone()));

        let mut provider_results = provider.search_with_candidates(query, &source_map, &candidates);
        if provider_results.is_empty() && !query.is_empty() {
            provider_results = provider.search(query, &source_map);
        }
        let mut all_symbols: Vec<Value> = provider_results
            .into_iter()
            .filter_map(|symbol| serde_json::to_value(symbol).ok())
            .collect();
        all_symbols.extend(
            text_fallback_symbols
                .into_iter()
                .filter_map(|symbol| serde_json::to_value(symbol).ok()),
        );
        all_symbols.truncate(cap);
        tracing::debug!(
            count = all_symbols.len(),
            "Workspace symbol: returned results from open documents"
        );
        self.record_workspace_symbols_provider_decision_trace(
            query,
            all_symbols.len(),
            WorkspaceSymbolsTraceKind::OpenDocumentFallback,
            0,
        );
        Ok(Some(json!(all_symbols)))
    }

    /// Workspace symbol runtime quality receipt for staged trust proof.
    ///
    /// Calls the live `workspace/symbol` handler and wraps the result in a typed
    /// receipt for staged cutover proof. Ready-index, non-empty queries can be
    /// classified as the narrow source-backed live slice; fallback and unproven
    /// shapes remain gated.
    ///
    /// Promotes only labeled source-backed generated/framework members in the
    /// full ready index. Stale, dynamic, ambiguous, partial-index,
    /// generated/no-source, and open-document fallback candidates remain gated.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn workspace_symbols_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();

        let live_provider_result = self.handle_workspace_symbols_v2(params)?;
        let live_provider_count = match live_provider_result.as_ref() {
            Some(Value::Array(items)) => items.len(),
            _ => 0,
        };
        let source_backed_count = self.workspace_symbols_source_backed_count(&query);
        let generated_pilot_count =
            workspace_symbols_labeled_generated_count(live_provider_result.as_ref());
        let no_live_behavior_change = source_backed_count == 0 && generated_pilot_count == 0;
        let shadow_state = match (source_backed_count > 0, generated_pilot_count > 0) {
            (true, true) => "partial_live_source_backed_generated_pilot",
            (true, false) => "partial_live_source_backed",
            (false, true) => "partial_live_generated_labeled_pilot",
            (false, false) => "shadowed",
        };
        let compiler_receipt = if no_live_behavior_change {
            Value::Null
        } else {
            let (source, provenance, confidence) =
                match (source_backed_count > 0, generated_pilot_count > 0) {
                    (true, true) => {
                        ("CompilerFact+FrameworkAdapter", "MixedExactAstFrameworkAnchor", "Medium")
                    }
                    (true, false) => ("CompilerFact", "ExactAst", "High"),
                    (false, true) => ("FrameworkAdapter", "FrameworkAnchor", "Medium"),
                    (false, false) => ("None", "None", "None"),
                };
            json!({
                "source": source,
                "provenance": provenance,
                "confidence": confidence,
                "freshness": "Fresh",
                "fallback_state": "Primary",
                "source_backed_count": source_backed_count,
                "source_backed_provenance": if source_backed_count > 0 { "ExactAst" } else { "None" },
                "generated_pilot_count": generated_pilot_count,
                "generated_pilot_provenance": if generated_pilot_count > 0 { "FrameworkAnchor" } else { "None" },
                "generated_pilot_confidence": if generated_pilot_count > 0 { "Medium" } else { "None" },
                "generated_pilot_location_semantics": if generated_pilot_count > 0 { "source_anchor_not_exact_generated_body" } else { "None" },
                "claim_boundary": "ready workspace index source-backed symbols plus labeled source-backed generated/framework pilot symbols only; dynamic, stale, ambiguous, fallback/noise, and partial-index candidates remain gated"
            })
        };
        let (gated_expansion_receipt, gated_expansion_candidate_count) =
            workspace_symbols_generated_dynamic_noise_receipt(&query);

        Ok(Some(json!({
            "provider": "workspace_symbols",
            "query": query,
            "live_provider_result": live_provider_result,
            "live_provider_count": live_provider_count,
            "shadow_state": shadow_state,
            "compiler_receipt": compiler_receipt,
            "gated_expansion_receipt": gated_expansion_receipt,
            "no_live_behavior_change": no_live_behavior_change,
            "notes": [
                format!(
                    "workspace-symbol runtime quality receipt: query={:?}; live_provider_count={}; \
                     source_backed_compiler_symbols={}; \
                     labeled_generated_pilot_symbols={}; \
                     generated_dynamic_noise_candidates={}; \
                     fresh ready-state workspace index symbols and labeled source-backed generated/framework pilot symbols are live for non-empty queries; \
                     empty, partial-index, stale, dynamic, generated/no-source, ambiguous, and fallback/noise cases keep fallback or gated behavior",
                    query,
                    live_provider_count,
                    source_backed_count,
                    generated_pilot_count,
                    gated_expansion_candidate_count
                )
            ]
        })))
    }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
fn workspace_symbols_generated_dynamic_noise_receipt(query: &str) -> (Value, usize) {
    let candidates = vec![
        perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolShadowCandidate::shadow(
            "generated:source-anchor:workspace-symbol:framework_accessor:virtual",
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
        ),
        perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolShadowCandidate::blocked(
            "generated:no-source:workspace-symbol:runtime_installed_method:unanchored",
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
        ),
        perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolShadowCandidate::blocked(
            "generated:no-source:workspace-symbol:role_composed_method:unanchored",
            ProviderFactSourceKind::FrameworkAdapter,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
        ),
        perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolShadowCandidate::blocked(
            "blocker:workspace-symbol:dynamic_symbolic_reference",
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::High,
            ProviderFactFreshness::Fresh,
        ),
        perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolShadowCandidate::blocked(
            "stale:workspace-symbol:removed_symbol",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Low,
            ProviderFactFreshness::Stale,
        ),
        perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolShadowCandidate::fallback(
            "fallback:workspace-symbol:low_confidence_text_match",
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::Unknown,
        ),
    ];
    let candidate_count = candidates.len();

    let generated_candidate_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.source == ProviderFactSourceKind::FrameworkAdapter
                && candidate.fallback_state != ProviderFallbackState::Blocked
        })
        .count();
    let generated_no_source_blocker_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.source == ProviderFactSourceKind::FrameworkAdapter
                && candidate.fallback_state == ProviderFallbackState::Blocked
                && candidate.identity.contains(":no-source:")
        })
        .count();
    let generated_no_source_candidate_identities = candidates
        .iter()
        .filter(|candidate| {
            candidate.source == ProviderFactSourceKind::FrameworkAdapter
                && candidate.fallback_state == ProviderFallbackState::Blocked
                && candidate.identity.contains(":no-source:")
        })
        .map(|candidate| candidate.identity.clone())
        .collect::<Vec<_>>();
    let dynamic_boundary_blocker_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.fallback_state == ProviderFallbackState::Blocked
                && candidate.source == ProviderFactSourceKind::DynamicBoundary
        })
        .count();
    let stale_fact_blocker_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.fallback_state == ProviderFallbackState::Blocked
                && candidate.freshness == ProviderFactFreshness::Stale
        })
        .count();
    let fallback_noise_candidate_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.fallback_state == ProviderFallbackState::Fallback
                || candidate.confidence == Confidence::Low
                || candidate.freshness != ProviderFactFreshness::Fresh
        })
        .count();

    let shadow = perl_lsp_rs_core::providers::workspace_symbols::workspace_symbol_source_shadow(
        Vec::new(),
        candidates,
        query,
    );

    (
        json!({
            "schema_version": 1,
            "receipt_kind": "generated_dynamic_noise_expansion",
            "generated_candidate_count": generated_candidate_count,
            "generated_false_exact_candidate_count": generated_candidate_count,
            "generated_no_source_candidate_count": generated_no_source_blocker_count,
            "generated_no_source_blocker_count": generated_no_source_blocker_count,
            "generated_no_source_candidate_identities": generated_no_source_candidate_identities,
            "generated_location_semantics": "source_anchor_not_exact_generated_body",
            "dynamic_boundary_blocker_count": dynamic_boundary_blocker_count,
            "dynamic_false_exact_blocker_count": dynamic_boundary_blocker_count,
            "stale_fact_blocker_count": stale_fact_blocker_count,
            "fallback_noise_candidate_count": fallback_noise_candidate_count,
            "no_live_behavior_change": true,
            "edit_freshness_policy": "labeled generated workspace-symbol queries must recompute from fresh document state after didChange; stale compiler-fact shadow candidates remain blocked by the gated-expansion receipt",
            "claim_boundary": "workspace-symbol generated/dynamic/noise expansion receipt only; generated/no-source false-exact candidates, dynamic-boundary candidates, stale compiler-fact shadow candidates, and fallback/noise candidates remain gated outside the labeled source-backed generated pilot",
            "shadow_receipt": shadow.receipt,
        }),
        candidate_count,
    )
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
fn workspace_symbols_labeled_generated_count(result: Option<&Value>) -> usize {
    match result {
        Some(Value::Array(items)) => items
            .iter()
            .filter(|item| {
                workspace_symbol_has_generated_label(
                    item.get("name").and_then(Value::as_str).unwrap_or_default(),
                    item.get("containerName").and_then(Value::as_str),
                )
            })
            .count(),
        _ => 0,
    }
}

fn workspace_symbol_has_generated_label(name: &str, container_name: Option<&str>) -> bool {
    name.contains("[generated/framework]")
        || container_name.is_some_and(|container| container.contains("[generated/framework]"))
}

impl LspServer {
    /// Search open documents for symbols (non-workspace stub)
    #[cfg(not(feature = "workspace"))]
    fn search_open_documents_for_symbols(
        &self,
        query: &str,
        _cap: usize,
    ) -> Result<Option<Value>, JsonRpcError> {
        tracing::debug!(query, "Workspace symbol: no workspace feature, returning empty");
        Ok(Some(json!([])))
    }

    /// Handle workspace/symbol request (legacy implementation)
    #[cfg(not(feature = "workspace"))]
    pub(super) fn handle_workspace_symbols(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query"))
            .and_then(|q| q.as_str())
            .unwrap_or("")
            .trim();

        tracing::debug!(query, "Workspace symbol search");

        // Lightweight snapshot: only clone fields needed for symbol extraction,
        // avoiding expensive Rope, ParentMap, LineStartsCache, and parse_errors clones.
        let docs_snapshot: Vec<(String, String, Option<Arc<perl_parser::ast::Node>>)> = {
            let documents = self.documents.lock();
            documents.iter().map(|(k, v)| (k.clone(), v.text.clone(), v.ast.clone())).collect()
        };

        // Build source map and index documents with WorkspaceSymbolsProvider.
        let cap = workspace_symbol_cap();
        let mut provider =
            perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolsProvider::new();
        let mut source_map = std::collections::HashMap::new();
        for (uri, text, ast) in docs_snapshot.iter() {
            if let Some(ast) = ast {
                provider.index_document(uri, ast, text);
            }
            source_map.insert(uri.clone(), text.clone());
        }

        let mut candidates = self.symbol_index.lock().search_prefix(query);
        if candidates.is_empty() && !query.is_empty() {
            candidates = self.symbol_index.lock().search_fuzzy(query);
        }
        let mut dedup = HashSet::new();
        candidates.retain(|candidate| dedup.insert(candidate.clone()));

        let mut symbols = provider.search_with_candidates(query, &source_map, &candidates);
        if symbols.is_empty() && !query.is_empty() {
            symbols = provider.search(query, &source_map);
        }
        symbols.truncate(cap);

        tracing::debug!(count = symbols.len(), cap, "Found symbols total");

        let result = serde_json::to_value(&symbols).unwrap_or_else(|_| json!([]));

        Ok(Some(result))
    }

    /// Handle workspaceSymbol/resolve request
    pub(super) fn handle_workspace_symbol_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            // Extract the symbol to resolve
            let symbol = params.as_object().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            })?;

            // Get the URI and name from the symbol
            let uri = symbol
                .get("location")
                .and_then(|l| l.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("");

            let name = symbol.get("name").and_then(|n| n.as_str()).unwrap_or("");

            // Normalize the URI for lookup
            let uri_key = self.normalize_uri_key(uri);

            // Look up the symbol in our index to get more details
            let documents = self.documents.lock();
            let doc_opt = documents.get(&uri_key).or_else(|| documents.get(uri)); // try raw as a fallback

            if let Some(doc) = doc_opt {
                if let Some(ast) = &doc.ast {
                    // Find the symbol in the AST to get more accurate information
                    let extractor = crate::symbol::SymbolExtractor::new_with_source(&doc.text);
                    let symbol_table = extractor.extract(ast);

                    // Find matching symbol
                    for symbols in symbol_table.symbols.values() {
                        for sym in symbols {
                            if sym.name == name {
                                // Return enhanced symbol with detail and accurate range
                                let start_pos = doc
                                    .line_starts
                                    .offset_to_position(&doc.text, sym.location.start);
                                let end_pos =
                                    doc.line_starts.offset_to_position(&doc.text, sym.location.end);

                                // Start with the provided symbol JSON so we can add
                                // additional details without panicking if fields are missing
                                let mut resolved = json!(symbol);

                                use crate::symbol::VarKind;
                                // Add detail based on symbol kind
                                let detail = match sym.kind {
                                    crate::symbol::SymbolKind::Subroutine => {
                                        format!("sub {}", name)
                                    }
                                    crate::symbol::SymbolKind::Method => {
                                        format!("method {}", name)
                                    }
                                    crate::symbol::SymbolKind::Variable(VarKind::Scalar) => {
                                        format!("${}", name)
                                    }
                                    crate::symbol::SymbolKind::Variable(VarKind::Array) => {
                                        format!("@{}", name)
                                    }
                                    crate::symbol::SymbolKind::Variable(VarKind::Hash) => {
                                        format!("%{}", name)
                                    }
                                    crate::symbol::SymbolKind::Package => {
                                        format!("package {}", name)
                                    }
                                    crate::symbol::SymbolKind::Constant => {
                                        format!("constant {}", name)
                                    }
                                    _ => name.to_string(),
                                };
                                resolved["detail"] = json!(detail);
                                if let Some(doc) = &sym.documentation {
                                    resolved["documentation"] = json!(doc);
                                }

                                // Update location with accurate range
                                resolved["location"]["range"] = json!({
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    }
                                });

                                // Add container name derived from qualified symbol name
                                if let Some(container) =
                                    perl_parser_core::qualified_name::container_name(
                                        &sym.qualified_name,
                                    )
                                {
                                    resolved["containerName"] = json!(
                                        perl_module::path::normalize_package_separator(container)
                                    );
                                }

                                return Ok(Some(json!(resolved)));
                            }
                        }
                    }
                }
            }

            // Return the original symbol if we couldn't enhance it
            Ok(Some(json!(symbol)))
        } else {
            Err(JsonRpcError { code: -32602, message: "Missing params".to_string(), data: None })
        }
    }

    /// Handle workspace/configuration request
    ///
    /// Supports both direct array format and ConfigurationParams with items property
    pub(super) fn handle_configuration(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            // Support both direct array format and ConfigurationParams with items property
            let items =
                params.get("items").and_then(|i| i.as_array()).or_else(|| params.as_array());

            if let Some(items) = items {
                let mut results = Vec::new();

                for item in items {
                    if let Some(section) = item.get("section").and_then(|s| s.as_str()) {
                        tracing::debug!(section, "Configuration requested");

                        // Handle workspace configuration sections
                        let value = if section.starts_with("perl.workspace.") {
                            let workspace_config = self.workspace_config.lock();
                            match section {
                                "perl.workspace.includePaths" => {
                                    json!(workspace_config.include_paths)
                                }
                                "perl.workspace.useSystemInc" => {
                                    json!(workspace_config.use_system_inc)
                                }
                                "perl.workspace.usePerl5lib" => {
                                    json!(workspace_config.use_perl5lib)
                                }
                                "perl.workspace.perl5libPrecedence" => {
                                    let precedence = match workspace_config.perl5lib_precedence {
                                        perl_lsp_rs_core::config::Perl5LibPrecedence::Prepend => {
                                            "prepend"
                                        }
                                        perl_lsp_rs_core::config::Perl5LibPrecedence::Append => {
                                            "append"
                                        }
                                    };
                                    json!(precedence)
                                }
                                "perl.workspace.resolutionTimeout" => {
                                    json!(workspace_config.resolution_timeout_ms)
                                }
                                _ => json!(null),
                            }
                        } else {
                            let config = self.config.lock();
                            match section {
                                "perl.inlayHints.enabled" => json!(config.inlay_hints_enabled),
                                "perl.inlayHints.parameterHints" => {
                                    json!(config.inlay_hints_parameter_hints)
                                }
                                "perl.inlayHints.typeHints" => json!(config.inlay_hints_type_hints),
                                "perl.inlayHints.chainedHints" => {
                                    json!(config.inlay_hints_chained_hints)
                                }
                                "perl.inlayHints.maxLength" => json!(config.inlay_hints_max_length),
                                "perl.testRunner.enabled" => json!(config.test_runner_enabled),
                                "perl.testRunner.testCommand" => json!(config.test_runner_command),
                                "perl.testRunner.testArgs" => json!(config.test_runner_args),
                                "perl.testRunner.testTimeout" => json!(config.test_runner_timeout),
                                "perl.formatting.enabled" => json!(config.perltidy_enabled),
                                "perl.formatting.engine" => {
                                    let engine = match config.formatting_engine {
                                        perl_lsp_rs_core::config::FormatterMode::Native => "native",
                                        perl_lsp_rs_core::config::FormatterMode::Compat => "compat",
                                        perl_lsp_rs_core::config::FormatterMode::ExternalLegacy => {
                                            "external-perltidy"
                                        }
                                        perl_lsp_rs_core::config::FormatterMode::Off => "off",
                                    };
                                    json!(engine)
                                }
                                "perl.formatting.profile" => json!(config.perltidy_profile),
                                "perl.formatting.maximumLineLength" => {
                                    json!(config.perltidy_maximum_line_length)
                                }
                                "perl.formatting.indentColumns" => {
                                    json!(config.perltidy_indent_columns)
                                }
                                "perl.formatting.tabs" => json!(config.perltidy_tabs),
                                "perl.formatting.openingBraceOnNewLine" => {
                                    json!(config.perltidy_opening_brace_on_new_line)
                                }
                                "perl.formatting.cuddledElse" => {
                                    json!(config.perltidy_cuddled_else)
                                }
                                "perl.formatting.spaceAfterKeyword" => {
                                    json!(config.perltidy_space_after_keyword)
                                }
                                "perl.formatting.addTrailingCommas" => {
                                    json!(config.perltidy_add_trailing_commas)
                                }
                                "perl.formatting.verticalAlignment" => {
                                    json!(config.perltidy_vertical_alignment)
                                }
                                "perl.formatting.blockCommentIndentation" => {
                                    json!(config.perltidy_block_comment_indentation)
                                }
                                "perl.formatting.extraArgs" => json!(config.perltidy_extra_args),
                                "perl.formatting.timeoutSecs" => {
                                    json!(config.perltidy_timeout_secs)
                                }
                                _ => json!(null),
                            }
                        };

                        results.push(value);
                    }
                }

                return Ok(Some(json!(results)));
            }
        }

        Ok(Some(json!([])))
    }
}

/// Extract the perl-specific settings object from a `workspace/didChangeConfiguration` payload.
///
/// Accepts both the standard wrapped form `{"perl": {...}}` and the unwrapped form `{...}` used
/// by clients such as Sublime Text's LSP package that omit the outer `"perl"` key.
fn extract_perl_settings(settings: &Value) -> Option<&Value> {
    if let Some(perl) = settings.get("perl") {
        if perl.is_object() {
            return Some(perl);
        }
    }
    // Unwrapped: the settings object itself contains perl config keys directly.
    if settings.is_object() { Some(settings) } else { None }
}

impl LspServer {
    /// Handle workspace/didChangeConfiguration notification
    ///
    /// Updates both ServerConfig and WorkspaceConfig when the client
    /// notifies of configuration changes.
    pub(super) fn handle_did_change_configuration(&self, params: Option<Value>) {
        if let Some(params) = params {
            if let Some(settings) = params.get("settings") {
                tracing::debug!("Configuration changed, updating server settings");

                // Read perl settings once and update both configs.
                // Some clients (e.g. Sublime Text's LSP package) send settings without
                // wrapping them under a top-level "perl" key. Accept both shapes:
                //   - Wrapped:   {"perl": { "workspace": { "includePaths": [...] } }}
                //   - Unwrapped: { "workspace": { "includePaths": [...] } }
                if let Some(perl) = extract_perl_settings(settings) {
                    // Check whether any perlcritic-related setting is changing before
                    // updating config so we can decide whether to reset the shared
                    // CriticAnalyzer.  The analyzer is config-bound (severity, profile)
                    // so any change to those fields requires a fresh instance.
                    #[cfg(not(target_arch = "wasm32"))]
                    let critic_config_changed = {
                        let cfg = self.config.lock();
                        let new_enabled = perl
                            .get("perlcritic")
                            .and_then(|v| v.get("enabled"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(cfg.perlcritic_enabled);
                        let new_severity = perl
                            .get("perlcritic")
                            .and_then(|v| v.get("severity"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u8)
                            .unwrap_or(cfg.perlcritic_severity);
                        let new_profile = perl
                            .get("perlcritic")
                            .and_then(|v| v.get("profile"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let new_theme = perl
                            .get("perlcritic")
                            .and_then(|v| v.get("theme"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let new_native_profile = perl
                            .get("critic")
                            .and_then(|v| v.get("profile"))
                            .and_then(|v| v.as_str())
                            .and_then(
                                perl_lsp_rs_core::tooling::perl_critic::NativeCriticProfile::parse,
                            )
                            .map(|profile| profile.as_str().to_string())
                            .unwrap_or_else(|| cfg.native_critic_profile.clone());
                        let new_native_include = perl
                            .get("critic")
                            .and_then(|v| v.get("include"))
                            .and_then(|v| v.as_array())
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_else(|| cfg.native_critic_include.clone());
                        let new_native_exclude = perl
                            .get("critic")
                            .and_then(|v| v.get("exclude"))
                            .and_then(|v| v.as_array())
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_else(|| cfg.native_critic_exclude.clone());
                        new_enabled != cfg.perlcritic_enabled
                            || new_severity != cfg.perlcritic_severity
                            || new_profile != cfg.perlcritic_profile
                            || new_theme != cfg.perlcritic_theme
                            || new_native_profile != cfg.native_critic_profile
                            || new_native_include != cfg.native_critic_include
                            || new_native_exclude != cfg.native_critic_exclude
                    };

                    // Update server config (inlay hints, test runner)
                    {
                        let mut config = self.config.lock();
                        config.update_from_value(perl);
                        tracing::debug!("Updated server config from perl settings");
                    }

                    // Reset the shared CriticAnalyzer when any critic-related setting
                    // changed so the next diagnostic cycle rebuilds it with the new config.
                    #[cfg(not(target_arch = "wasm32"))]
                    if critic_config_changed {
                        *self.critic_analyzer.lock() = None;
                        self.critic_workspace_warnings_sent.lock().clear();
                        self.pull_diagnostics_orchestrator.reset();
                    }

                    // Update workspace config (include paths, @INC)
                    {
                        let mut workspace_config = self.workspace_config.lock();
                        workspace_config.update_from_value(perl);
                        tracing::debug!("Updated workspace config from perl settings");
                    }

                    // Apply global client settings to each folder's effective config immediately.
                    // The async workspace/configuration pull that follows will refine per-folder
                    // settings once the client responds, but we update now so the window between
                    // didChangeConfiguration arrival and the pull response doesn't leave folders
                    // with stale settings.
                    {
                        let mut folders = self.workspace_folders.lock();
                        for folder in folders.iter_mut() {
                            let mut effective_config =
                                perl_lsp_rs_core::config::WorkspaceConfig::default();
                            if let Some(project_config) = &folder.project_config {
                                project_config.apply_to_workspace_config(&mut effective_config);
                            }
                            effective_config.update_from_value(perl);
                            folder.effective_workspace_config = effective_config;
                        }
                    }

                    // Refresh AI backend when config changes (constructs or clears provider)
                    self.refresh_ai_backend();

                    // Trigger client refresh for configuration-dependent features
                    if let Err(e) = self.refresh_controller.refresh_all(self) {
                        tracing::warn!(error = %e, "Failed to refresh client after config change");
                    }
                }
            }
        }

        // Invalidate client-provided workspace/configuration values and re-fetch.
        self.pending_workspace_configuration_requests.lock().clear();
        self.request_workspace_configuration_for_folders();
    }

    /// Handle workspace/didChangeWatchedFiles notification
    ///
    /// Deterministic state transitions:
    /// - DELETED events are processed immediately (low frequency, state cleanup)
    /// - CREATED/CHANGED events are debounced to avoid blocking I/O storms during
    ///   bulk operations (e.g., `git checkout`, formatter rewrites)
    /// - State recovery is handled by coordinator's internal logic
    pub(super) fn handle_did_change_watched_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use lsp_types::{DidChangeWatchedFilesParams, FileChangeType};

        let Some(params) = params else {
            return Ok(None);
        };

        let Ok(params) = serde_json::from_value::<DidChangeWatchedFilesParams>(params) else {
            tracing::warn!("Failed to parse didChangeWatchedFiles params");
            return Ok(None);
        };

        for change in params.changes {
            let uri = change.uri.to_string();
            let change_type = change.typ;

            tracing::debug!(uri, change_type = ?change_type, "File change detected");

            match change_type {
                FileChangeType::DELETED => {
                    // DELETED must be processed immediately — the file is gone and
                    // stale index data should not linger.
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_change(&uri);
                    }

                    self.evict_deleted_file_state(&uri);

                    tracing::debug!(uri, "Removed deleted file from index");

                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_parse_complete(&uri);
                    }
                }
                FileChangeType::CREATED | FileChangeType::CHANGED => {
                    // CREATED and CHANGED are debounced so that bulk operations
                    // (git checkout, formatter rewrites, etc.) coalesce into a
                    // single batch rather than triggering many sequential file reads.
                    if !self.schedule_file_watcher_uri(&uri) {
                        // No debouncer installed (unit-test path) — fall through to
                        // immediate synchronous processing.
                        self.process_file_watcher_uri_immediate(&uri);
                    }
                }
                _ => {}
            }
        }

        // This is a notification, no response needed
        Ok(None)
    }

    /// Process a debounced batch of file URIs.
    ///
    /// Called by the [`FileWatcherDebouncer`] background thread after the quiet
    /// period expires.  Re-reads and re-indexes each URI.  Files that no longer
    /// exist on disk are silently skipped — they should have arrived as DELETED
    /// events and been handled immediately.
    pub(crate) fn handle_watched_file_batch(&self, uris: Vec<String>) {
        tracing::debug!("Processing debounced file watcher batch: {} URIs", uris.len());
        for uri in &uris {
            self.process_file_watcher_uri_immediate(uri);
        }
    }

    /// Re-index a single URI from the file system.
    ///
    /// Shared implementation used by both the debounced batch path and the
    /// immediate fall-through path when no debouncer is installed.
    fn process_file_watcher_uri_immediate(&self, uri: &str) {
        // Notify coordinator of pending change
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            coordinator.notify_change(uri);
        }

        let mut loaded_content: Option<String> = None;

        // Re-index the file if it is a Perl source file.
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            if is_perl_source_uri(uri) {
                if loaded_content.is_none() {
                    loaded_content = read_watched_file_content(uri, "re-indexing");
                }

                let workspace_index = coordinator.index();
                if let Ok(url) = url::Url::parse(uri) {
                    if let Some(content) = loaded_content.as_ref() {
                        // Clear old index data before re-indexing
                        workspace_index.clear_file(uri);
                        match workspace_index.index_file(url, content.clone()) {
                            Ok(()) => tracing::debug!("Re-indexed file: {}", uri),
                            Err(e) => {
                                tracing::warn!("Failed to re-index file {}: {}", uri, e);
                            }
                        }
                    }
                }
            }
        }

        // Also update our internal document store if the document is open.
        #[cfg(feature = "workspace")]
        {
            let document_is_open = {
                let documents = self.documents.lock();
                self.get_document(&documents, uri).is_some()
            };

            if document_is_open {
                if loaded_content.is_none() {
                    loaded_content = read_watched_file_content(uri, "document store update");
                }

                if let Some(content) = loaded_content {
                    let mut documents = self.documents.lock();
                    if let Some(doc) = self.get_document_mut(&mut documents, uri) {
                        doc.text = content;
                        doc.version += 1;
                        // Clear cached AST so it is regenerated on next access.
                        doc.ast = None;
                    }
                }
            }
        }

        // Notify coordinator that file processing is complete
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            coordinator.notify_parse_complete(uri);
        }

        tracing::debug!("Processed file watcher change: {}", uri);
    }

    /// Handle workspace/willRenameFiles request
    pub(super) fn handle_will_rename_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                let mut workspace_edit = json!({
                    "changes": {}
                });
                let mut planned_workspace_texts: std::collections::BTreeMap<
                    String,
                    (String, String),
                > = std::collections::BTreeMap::new();

                for file in files {
                    let Some(old_uri) = file["oldUri"].as_str() else {
                        continue;
                    };
                    let Some(new_uri) = file["newUri"].as_str() else {
                        continue;
                    };

                    tracing::debug!("File rename: {} -> {}", old_uri, new_uri);

                    // Extract module names from file paths
                    let old_module = path_to_module_name(old_uri);
                    let new_module = path_to_module_name(new_uri);

                    if !old_module.is_empty() && !new_module.is_empty() {
                        if !planned_workspace_texts.contains_key(old_uri) {
                            if let Some(text) = self.read_workspace_text(old_uri) {
                                planned_workspace_texts
                                    .insert(old_uri.to_string(), (text.clone(), text));
                            }
                        }
                        if let Some((_, current_text)) = planned_workspace_texts.get_mut(old_uri) {
                            let planned =
                                plan_module_rename_edits(current_text, &old_module, &new_module);
                            if !planned.is_empty() {
                                *current_text = apply_module_rename_edits(current_text, &planned);
                            }
                        }

                        // Find all files that reference the old module
                        // Note: Query operation - use coordinator.index() for consistency
                        #[cfg(feature = "workspace")]
                        let dependents = if let Some(coordinator) = self.coordinator() {
                            coordinator.index().find_dependents(&old_module)
                        } else {
                            Vec::new()
                        };

                        #[cfg(not(feature = "workspace"))]
                        let dependents = Vec::<String>::new();

                        for dependent_uri in dependents {
                            if !planned_workspace_texts.contains_key(&dependent_uri) {
                                let Some(text) = self.read_workspace_text(&dependent_uri) else {
                                    continue;
                                };
                                planned_workspace_texts
                                    .insert(dependent_uri.clone(), (text.clone(), text));
                            }

                            if let Some((_, current_text)) =
                                planned_workspace_texts.get_mut(&dependent_uri)
                            {
                                let planned = plan_module_rename_edits(
                                    current_text,
                                    &old_module,
                                    &new_module,
                                );
                                if !planned.is_empty() {
                                    *current_text =
                                        apply_module_rename_edits(current_text, &planned);
                                }
                            }
                        }
                    }

                    // Update the index for the renamed file
                    // Note: Mutation operation - use coordinator with lifecycle tracking
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_change(old_uri);
                        coordinator.notify_change(new_uri);
                        let workspace_index = coordinator.index();
                        workspace_index.remove_file(old_uri);
                        if let Some(path) = uri_to_fs_path(new_uri) {
                            if let Ok(content) = read_text_with_encoding_fallback(&path) {
                                if let Ok(url) = url::Url::parse(new_uri) {
                                    if let Err(e) = workspace_index.index_file(url, content.clone())
                                    {
                                        tracing::warn!(
                                            "Failed to index renamed file {}: {}",
                                            new_uri,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        coordinator.notify_parse_complete(old_uri);
                        coordinator.notify_parse_complete(new_uri);
                    }

                    // Warn the user if open documents reference the old module name via
                    // patterns that were not updated (e.g., `->` static calls, `@ISA`,
                    // qualified function calls). These are known gaps tracked in
                    // docs/reference/KNOWN_LIMITATIONS.md.
                    if !old_module.is_empty() {
                        #[cfg(feature = "workspace")]
                        let updated_uris: std::collections::HashSet<&str> =
                            planned_workspace_texts.keys().map(String::as_str).collect();
                        #[cfg(not(feature = "workspace"))]
                        let updated_uris = std::collections::HashSet::<&str>::new();
                        // Build a word-boundary pattern so "Base" does not match "Database".
                        // Perl module names consist of \w and ::, so we check that any match
                        // of old_module in the document text is not immediately preceded or
                        // followed by a word character.
                        let documents = self.documents.lock();
                        let unhandled = documents.iter().any(|(uri, doc)| {
                            // Skip the file being renamed itself — it is expected to contain
                            // the old module name (e.g., `package OldModule;`).
                            if uri.as_str() == old_uri {
                                return false;
                            }
                            if updated_uris.contains(uri.as_str()) {
                                return false;
                            }
                            // Word-boundary check: reject matches where old_module is part of
                            // a longer identifier (e.g., "Base" inside "Database").
                            module_name_appears_in_text(&doc.text, old_module.as_str())
                        });
                        drop(documents);
                        if unhandled {
                            let msg = format!(
                                "Some references to '{}' may not have been updated. \
                                 String literals, comments, and dynamic method calls \
                                 are not automatically rewritten. \
                                 Use find-and-replace to update them manually.",
                                old_module
                            );
                            if let Err(e) = self
                                .show_message(crate::runtime::window::MessageType::Warning, &msg)
                            {
                                tracing::debug!("Failed to send rename warning: {}", e);
                            }
                        }
                    }
                }

                #[cfg(feature = "workspace")]
                for (uri, (original_text, current_text)) in planned_workspace_texts {
                    self.append_workspace_edits(
                        &mut workspace_edit,
                        &uri,
                        build_module_rename_workspace_edits(&original_text, &current_text),
                    );
                }

                return Ok(Some(workspace_edit));
            }
        }

        // Return empty edit if no changes needed
        Ok(Some(json!({"changes": {}})))
    }

    /// Handle workspace/didDeleteFiles notification
    pub(super) fn handle_did_delete_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                for file in files {
                    let Some(uri) = file["uri"].as_str() else {
                        continue;
                    };

                    tracing::debug!(uri, "File deleted");

                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_change(uri);
                    }
                    self.evict_deleted_file_state(uri);
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_parse_complete(uri);
                    }
                }

                // Trigger client refresh after file deletions
                if let Err(e) = self.refresh_controller.refresh_all(self) {
                    tracing::warn!(error = %e, "Failed to refresh client after file deletions");
                }
            }
        }

        // This is a notification, no response needed
        Ok(None)
    }

    /// Handle workspace/willDeleteFiles request
    pub(super) fn handle_will_delete_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    let idx = coordinator.index();
                    let open_documents: Vec<(String, String)> = {
                        let documents = self.documents.lock();
                        documents.iter().map(|(uri, doc)| (uri.clone(), doc.text.clone())).collect()
                    };
                    let deleting_uris: std::collections::HashSet<String> = files
                        .iter()
                        .filter_map(|file| {
                            file["uri"].as_str().map(|uri| self.normalize_uri_key(uri))
                        })
                        .collect();
                    let mut unsafe_deletes: Vec<(String, usize, Vec<String>)> = Vec::new();

                    for file in files {
                        let Some(uri) = file["uri"].as_str() else {
                            continue;
                        };

                        tracing::debug!(uri, "File will be deleted");
                        let mut dependents: std::collections::BTreeSet<String> =
                            collect_cross_file_delete_dependents(idx, uri, &deleting_uris);
                        dependents.extend(collect_open_document_delete_dependents(
                            idx,
                            uri,
                            &deleting_uris,
                            &open_documents,
                        ));
                        dependents.extend(collect_symbol_reference_delete_dependents(
                            idx,
                            uri,
                            &deleting_uris,
                        ));
                        let dependents: Vec<String> = dependents.into_iter().collect();

                        if !dependents.is_empty() {
                            let examples: Vec<String> =
                                dependents.iter().take(3).map(|uri| short_uri(uri)).collect();
                            tracing::warn!(
                                uri,
                                dependent_file_count = dependents.len(),
                                "Safe delete detected dependent workspace files"
                            );
                            unsafe_deletes.push((short_uri(uri), dependents.len(), examples));
                        }
                    }

                    if !unsafe_deletes.is_empty() {
                        let msg = if unsafe_deletes.len() == 1 {
                            let (uri, dependent_count, examples) = &unsafe_deletes[0];
                            let example_suffix = if examples.is_empty() {
                                String::new()
                            } else {
                                format!(" Example dependents: {}.", examples.join(", "))
                            };
                            format!(
                                "Safe delete warning: '{}' has {} dependent workspace file(s). \
                                 Delete may break callers.{}",
                                uri, dependent_count, example_suffix
                            )
                        } else {
                            format!(
                                "Safe delete warning: {} files have dependent workspace files. \
                                 Delete may break callers.",
                                unsafe_deletes.len()
                            )
                        };
                        if let Err(e) =
                            self.show_message(crate::runtime::window::MessageType::Warning, &msg)
                        {
                            tracing::debug!("Failed to send safe-delete warning: {}", e);
                        }
                    }
                }
            }
        }

        // Return empty edit - no cleanup edits needed for now
        Ok(Some(json!({"changes": {}})))
    }

    /// Handle workspace/willCreateFiles request
    pub(super) fn handle_will_create_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                for file in files {
                    let Some(uri) = file["uri"].as_str() else {
                        continue;
                    };

                    tracing::debug!("File will be created: {}", uri);
                }
            }
        }

        // Return empty edit - no setup edits needed for now
        Ok(Some(json!({"changes": {}})))
    }

    /// Handle workspace/didCreateFiles notification
    pub(super) fn handle_did_create_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                for file in files {
                    let Some(uri) = file["uri"].as_str() else {
                        continue;
                    };

                    tracing::debug!("File created: {}", uri);

                    // Index the new file if it's a Perl file
                    // Note: Mutation operation - use coordinator with lifecycle tracking
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        if is_perl_source_uri(uri) {
                            if let Some(path) = uri_to_fs_path(uri) {
                                match read_text_with_encoding_fallback(&path) {
                                    Ok(content) => {
                                        coordinator.notify_change(uri);
                                        if let Ok(url) = url::Url::parse(uri) {
                                            match coordinator.index().index_file(url, content) {
                                                Ok(()) => {
                                                    tracing::debug!("Indexed new file: {}", uri)
                                                }
                                                Err(e) => {
                                                    tracing::warn!(
                                                        "Failed to index new file {}: {}",
                                                        uri,
                                                        e
                                                    )
                                                }
                                            }
                                        }
                                        coordinator.notify_parse_complete(uri);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Failed to read new file for indexing ({}): {}",
                                            path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Trigger client refresh after file creations
                if let Err(e) = self.refresh_controller.refresh_all(self) {
                    tracing::warn!("Failed to refresh client after file creations: {}", e);
                }
            }
        }

        // This is a notification, no response needed
        Ok(None)
    }

    /// Handle workspace/didRenameFiles notification
    pub(super) fn handle_did_rename_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                for file in files {
                    let Some(old_uri) = file["oldUri"].as_str() else {
                        continue;
                    };
                    let Some(new_uri) = file["newUri"].as_str() else {
                        continue;
                    };

                    tracing::debug!("File renamed: {} -> {}", old_uri, new_uri);

                    // Update the index for the renamed file
                    // Note: Mutation operation - use coordinator with lifecycle tracking
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_change(old_uri);
                        coordinator.notify_change(new_uri);

                        // Remove old file from index
                        coordinator.index().remove_file(old_uri);

                        // Index new file if it's a Perl file
                        if is_perl_source_uri(new_uri) {
                            if let Some(path) = uri_to_fs_path(new_uri) {
                                match read_text_with_encoding_fallback(&path) {
                                    Ok(content) => {
                                        if let Ok(url) = url::Url::parse(new_uri) {
                                            match coordinator.index().index_file(url, content) {
                                                Ok(()) => {
                                                    tracing::debug!(
                                                        "Indexed renamed file: {}",
                                                        new_uri
                                                    )
                                                }
                                                Err(e) => tracing::warn!(
                                                    "Failed to index renamed file {}: {}",
                                                    new_uri,
                                                    e
                                                ),
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Failed to read renamed file for indexing ({}): {}",
                                            path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        coordinator.notify_parse_complete(old_uri);
                        coordinator.notify_parse_complete(new_uri);
                    }

                    // Update document store
                    {
                        let mut documents = self.documents.lock();
                        if let Some(doc) = documents.remove(old_uri) {
                            documents.insert(new_uri.to_string(), doc);
                        }
                    }
                }

                // Trigger client refresh after file renames
                if let Err(e) = self.refresh_controller.refresh_all(self) {
                    tracing::warn!(error = %e, "Failed to refresh client after file renames");
                }
            }
        }

        // This is a notification, no response needed
        Ok(None)
    }

    /// Handle workspace/didChangeWorkspaceFolders notification
    pub(super) fn handle_did_change_workspace_folders(
        &self,
        params: Option<Value>,
    ) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            if let Some(event) = params.get("event") {
                let change = extract_workspace_folder_change(event);

                if !change.added.is_empty() {
                    let mut workspace_folders = self.workspace_folders.lock();
                    for uri in &change.added {
                        tracing::debug!(uri, "Added workspace folder");
                        let mut folder_state =
                            super::workspace_folder::WorkspaceFolderState::new(uri.clone());

                        // Resolve the folder path
                        if let Some(path) = super::source_path_from_uri(uri) {
                            folder_state = folder_state.with_path(path);
                        }

                        workspace_folders.push(folder_state);
                    }
                }

                if !change.removed.is_empty() {
                    let mut workspace_folders = self.workspace_folders.lock();
                    let removed_uris: std::collections::HashSet<String> =
                        change.removed.iter().cloned().collect();

                    for uri in &change.removed {
                        tracing::debug!(uri, "Removed workspace folder");
                        self.evict_workspace_folder_state(uri);
                    }

                    // Retain only folders that are not in the removed list
                    workspace_folders.retain(|f| !removed_uris.contains(&f.uri));
                }

                // Workspace folder membership changed, so any in-flight reverse
                // request now has stale per-folder scoping. Drop pending entries
                // before issuing a fresh `workspace/configuration` pull.
                self.pending_workspace_configuration_requests.lock().clear();

                // Load config for all folders after changes
                self.load_and_apply_project_config();

                // Update workspace index with new folder list
                #[cfg(feature = "workspace")]
                {
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.index().set_workspace_folders(self.workspace_folder_uris());

                        // Removed folders were evicted above before folder
                        // membership was updated.
                    }
                }

                // Trigger client refresh after workspace folder changes
                if let Err(e) = self.refresh_controller.refresh_all(self) {
                    tracing::warn!(error = %e, "Failed to refresh client after workspace folder changes");
                }

                // Rebuild workspace index after folder changes
                #[cfg(feature = "workspace")]
                self.start_workspace_indexing();
            }
        }

        Ok(())
    }

    /// Start a background workspace indexing scan
    ///
    /// Uses a compare-exchange guard on `indexing_in_progress` to ensure only
    /// one scan runs at a time.  If a scan is already running the call is
    /// silently skipped (logged via `eprintln!`).
    #[cfg(feature = "workspace")]
    pub(super) fn start_workspace_indexing(&self) {
        // Guard: if already indexing, skip.  compare_exchange ensures only one
        // thread wins the race.
        if self
            .indexing_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            tracing::debug!("Workspace indexing already in progress, skipping concurrent scan");
            return;
        }
        let indexing_guard = IndexingGuard(Arc::clone(&self.indexing_in_progress));

        let Some(coordinator) = self.coordinator().map(Arc::clone) else {
            return;
        };

        // Ensure workspace folders are set in the index before indexing starts
        let workspace_folder_uris = self.workspace_folder_uris();
        coordinator.index().set_workspace_folders(workspace_folder_uris.clone());

        let workspace_folders = self.workspace_folders.lock().clone();
        if workspace_folders.is_empty() {
            return;
        }

        let outbound = self.outbound.clone();
        let limits = coordinator.limits().clone();
        let caps = coordinator.performance_caps().clone();
        let work_done_progress = self.client_capabilities.lock().work_done_progress_support;
        // Generate a request ID for the workDoneProgress/create call. Atomically
        // increment so it doesn't collide with IDs from other server-to-client requests.
        let progress_create_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        let permission_denied_shown = Arc::clone(&self.permission_denied_shown);

        std::thread::spawn(move || {
            let _guard = indexing_guard; // moved into closure, drops when closure exits
            let budget_start = Instant::now();
            coordinator.transition_to_scanning();

            // Send progress begin if client supports work done progress.
            if work_done_progress {
                send_progress_create(&outbound, progress_create_id);
                send_progress_begin(&outbound);
            }

            let mut files: Vec<std::path::PathBuf> = Vec::new();
            let mut early_exit: Option<(EarlyExitReason, u64, usize, usize)> = None;

            'scan: for folder_state in workspace_folders {
                let Some(root) =
                    folder_state.path.clone().or_else(|| uri_to_fs_path(&folder_state.uri))
                else {
                    tracing::debug!(
                        uri = %folder_state.uri,
                        "Skipping non-filesystem workspace folder during indexing scan"
                    );
                    continue;
                };

                let discovery = super::file_discovery::discover_perl_files(&root);

                for path in discovery.files {
                    files.push(path);
                    let total_files = files.len();

                    if total_files.is_multiple_of(64) {
                        coordinator.update_scan_progress(total_files);
                    }

                    let elapsed_ms = budget_start.elapsed().as_millis() as u64;
                    if total_files >= limits.max_files {
                        early_exit = Some((EarlyExitReason::FileLimit, elapsed_ms, 0, total_files));
                        break 'scan;
                    }

                    if elapsed_ms > caps.initial_scan_budget_ms {
                        early_exit =
                            Some((EarlyExitReason::InitialTimeBudget, elapsed_ms, 0, total_files));
                        break 'scan;
                    }
                }
            }

            coordinator.update_scan_progress(files.len());
            coordinator.transition_to_indexing(files.len());

            let mut indexed_files = 0usize;
            let total_files = files.len();
            // Track the last file count at which a progress report was sent so we
            // can batch updates every 50 files (avoid flooding small workspaces).
            let mut last_reported = 0usize;

            for path in files {
                let elapsed_ms = budget_start.elapsed().as_millis() as u64;
                if elapsed_ms > caps.initial_scan_budget_ms {
                    early_exit = Some((
                        EarlyExitReason::InitialTimeBudget,
                        elapsed_ms,
                        indexed_files,
                        total_files,
                    ));
                    break;
                }

                let content = match read_text_with_encoding_fallback(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        if is_permission_denied_error(&e) {
                            // ONE-TIME window/showMessage (AtomicBool guard)
                            if permission_denied_shown
                                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                                .is_ok()
                            {
                                let msg = "Perl LSP: some workspace files could not be read \
                                           due to permission denied. Features for those files \
                                           will be unavailable. Check file permissions.";
                                if let Err(send_err) = outbound.send_notification(
                                    "window/showMessage",
                                    json!({ "type": 2, "message": msg }),
                                ) {
                                    tracing::warn!(
                                        error = %send_err,
                                        "Failed to send permission-denied showMessage"
                                    );
                                }
                            }
                            // Per-file diagnostic (always fires for each affected file)
                            if let Ok(url) = Url::from_file_path(&path) {
                                let uri_str = url.as_str();
                                if let Err(send_err) = outbound.send_notification(
                                    "textDocument/publishDiagnostics",
                                    json!({
                                        "uri": uri_str,
                                        "diagnostics": [{
                                            "range": {
                                                "start": { "line": 0, "character": 0 },
                                                "end":   { "line": 0, "character": 0 }
                                            },
                                            "severity": 1,
                                            "source": "perl-lsp",
                                            "message": format!(
                                                "File cannot be read: permission denied ({})",
                                                path.display()
                                            )
                                        }]
                                    }),
                                ) {
                                    tracing::warn!(
                                        error = %send_err,
                                        "Failed to send permission-denied diagnostic"
                                    );
                                }
                            }
                        } else {
                            tracing::debug!(
                                "Skipping unreadable file during indexing ({}): {}",
                                path.display(),
                                e
                            );
                        }
                        continue;
                    }
                };
                let Ok(url) = Url::from_file_path(&path) else {
                    continue;
                };
                if coordinator.index().index_file(url, content).is_ok() {
                    indexed_files += 1;
                    coordinator.update_building_progress(indexed_files);

                    // Send a progress report every 50 files.
                    if work_done_progress && indexed_files - last_reported >= 50 {
                        send_progress_report(&outbound, indexed_files, total_files);
                        last_reported = indexed_files;
                    }
                }
            }

            if let Some((reason, elapsed_ms, indexed_files, total_files)) = early_exit {
                coordinator.record_early_exit(reason, elapsed_ms, indexed_files, total_files);
                match reason {
                    EarlyExitReason::FileLimit => {
                        coordinator.transition_to_degraded(DegradationReason::ResourceLimit {
                            kind: ResourceKind::MaxFiles,
                        });
                    }
                    EarlyExitReason::InitialTimeBudget | EarlyExitReason::IncrementalTimeBudget => {
                        coordinator
                            .transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms });
                    }
                }
                if work_done_progress {
                    send_progress_end(&outbound, "Indexing stopped early");
                }
                send_index_ready_notification(&outbound, false);
            } else {
                let file_count = coordinator.index().file_count();
                let symbol_count = coordinator.index().symbol_count();
                coordinator.transition_to_ready(file_count, symbol_count);
                if work_done_progress {
                    send_progress_end(&outbound, "Indexing complete");
                }
                send_index_ready_notification(&outbound, true);
            }
        });
    }

    /// Handle workspace/applyEdit request
    pub(super) fn handle_apply_edit(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let Some(edit) = params.get("edit") else {
                return Ok(Some(
                    json!({"applied": false, "failureReason": "Missing 'edit' field"}),
                ));
            };

            tracing::debug!("Applying workspace edit");

            // Apply changes to each document
            if let Some(changes) = edit["changes"].as_object() {
                for (uri, edits) in changes {
                    if let Some(edits) = edits.as_array() {
                        let mut documents = self.documents.lock();
                        if let Some(doc) = self.get_document_mut(&mut documents, uri) {
                            // Apply edits in reverse order to maintain positions
                            let mut sorted_edits = edits.clone();
                            sorted_edits.sort_by(|a, b| {
                                let a_line = a["range"]["start"]["line"].as_u64().unwrap_or(0);
                                let b_line = b["range"]["start"]["line"].as_u64().unwrap_or(0);
                                b_line.cmp(&a_line)
                            });

                            for edit in sorted_edits {
                                if let Some(new_text) = edit["newText"].as_str() {
                                    let start_line =
                                        edit["range"]["start"]["line"].as_u64().unwrap_or(0)
                                            as usize;
                                    let start_char =
                                        edit["range"]["start"]["character"].as_u64().unwrap_or(0)
                                            as usize;
                                    let end_line =
                                        edit["range"]["end"]["line"].as_u64().unwrap_or(0) as usize;
                                    let end_char =
                                        edit["range"]["end"]["character"].as_u64().unwrap_or(0)
                                            as usize;

                                    // Apply the edit to the document content
                                    let lines: Vec<String> =
                                        doc.text.lines().map(String::from).collect();
                                    let mut new_lines = Vec::new();

                                    // Copy lines before the edit
                                    for i in 0..start_line {
                                        new_lines.push(lines[i].clone());
                                    }

                                    // Apply the edit
                                    if start_line == end_line {
                                        let line = &lines[start_line];
                                        let new_line = format!(
                                            "{}{}{}",
                                            &line[..start_char.min(line.len())],
                                            new_text,
                                            &line[end_char.min(line.len())..]
                                        );
                                        new_lines.push(new_line);
                                    } else {
                                        // Multi-line edit
                                        let first_line = &lines[start_line];
                                        let last_line = &lines[end_line];
                                        let new_line = format!(
                                            "{}{}{}",
                                            &first_line[..start_char.min(first_line.len())],
                                            new_text,
                                            &last_line[end_char.min(last_line.len())..]
                                        );
                                        new_lines.push(new_line);
                                    }

                                    // Copy lines after the edit
                                    for i in (end_line + 1)..lines.len() {
                                        new_lines.push(lines[i].clone());
                                    }

                                    doc.text = new_lines.join("\n");
                                    doc.version += 1;
                                }
                            }

                            // Re-index the file after changes
                            // Note: Mutation operation - use coordinator with lifecycle tracking
                            #[cfg(feature = "workspace")]
                            if let Some(coordinator) = self.coordinator() {
                                coordinator.notify_change(uri);
                                if let Ok(url) = url::Url::parse(uri) {
                                    if let Err(e) =
                                        coordinator.index().index_file(url, doc.text.clone())
                                    {
                                        tracing::warn!("Failed to re-index file {}: {}", uri, e);
                                    }
                                }
                                coordinator.notify_parse_complete(uri);
                            }

                            // Clear cached AST
                            doc.ast = None;
                        }
                    }
                }
            }

            // Return success
            return Ok(Some(json!({"applied": true})));
        }

        Ok(Some(json!({"applied": false, "failureReason": "Invalid parameters"})))
    }
}

#[cfg(feature = "workspace")]
enum WorkspaceSymbolsTraceKind {
    SourceBackedReadyIndex,
    PartialIndexFallback,
    OpenDocumentFallback,
}

#[cfg(feature = "workspace")]
impl LspServer {
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    fn workspace_symbols_source_backed_count(&self, query: &str) -> usize {
        if query.is_empty() {
            return 0;
        }
        let IndexAccessMode::Full(coordinator) = route_index_access(self.coordinator()) else {
            return 0;
        };
        coordinator.index().search_source_symbols(query).iter().take(workspace_symbol_cap()).count()
    }

    fn record_workspace_symbols_provider_decision_trace(
        &self,
        query: &str,
        result_count: usize,
        kind: WorkspaceSymbolsTraceKind,
        generated_pilot_count: usize,
    ) {
        let (
            decision,
            reason,
            fact_source,
            confidence,
            source_backed,
            source_backed_state,
            fallback_state,
            claim_boundary,
        ) = match kind {
            WorkspaceSymbolsTraceKind::SourceBackedReadyIndex
                if !query.is_empty() && generated_pilot_count > 0 =>
            {
                (
                    "acted",
                    "source_backed_generated_label_pilot",
                    "framework_adapter",
                    "medium",
                    true,
                    "ready_workspace_index_generated_label_pilot",
                    "none",
                    "returns ready-state source-backed workspace index symbols plus labeled generated/framework pilot symbols; generated symbols point to source framework declarations, not exact generated method bodies; dynamic, stale, and ambiguous compiler candidates remain gated",
                )
            }
            WorkspaceSymbolsTraceKind::SourceBackedReadyIndex if !query.is_empty() => (
                "acted",
                "source_backed_high_confidence",
                "compiler_fact",
                "high",
                true,
                "ready_workspace_index",
                "none",
                "returns ready-state source-backed workspace index symbols only; generated, dynamic, stale, and ambiguous compiler candidates remain gated",
            ),
            WorkspaceSymbolsTraceKind::SourceBackedReadyIndex => (
                "fallback",
                "empty_query",
                "legacy_workspace",
                "low",
                false,
                "empty_query_not_promoted",
                "legacy_provider",
                "empty workspace-symbol queries keep existing broad provider behavior; no compiler-symbol promotion",
            ),
            WorkspaceSymbolsTraceKind::PartialIndexFallback => (
                "fallback",
                "partial_index",
                "legacy_workspace",
                "medium",
                false,
                "partial_index_not_full_workspace",
                "legacy_provider",
                "partial-index workspace symbols remain fallback behavior until the index is fresh and ready",
            ),
            WorkspaceSymbolsTraceKind::OpenDocumentFallback => (
                "fallback",
                "open_document_fallback",
                "fallback",
                "low",
                false,
                "not_proven_by_workspace_index",
                "legacy_provider",
                "open-document workspace-symbol fallback does not promote compiler-backed workspace symbols",
            ),
        };

        self.record_provider_decision_trace(
            "workspace_symbols",
            &json!({
                "provider": "workspace_symbols",
                "provider_action": "workspace/symbol",
                "decision": decision,
                "reason": reason,
                "fact_source": fact_source,
                "confidence": confidence,
                "freshness": "fresh",
                "source_backed": source_backed,
                "source_backed_state": source_backed_state,
                "dynamic_boundary": false,
                "fallback_state": fallback_state,
                "live_provider_result_kind": "array",
                "live_provider_result_count": result_count,
                "generated_pilot_count": generated_pilot_count,
                "query_empty": query.is_empty(),
                "live_cutover": if source_backed {
                    if generated_pilot_count > 0 {
                        "partial_live_source_backed_generated_pilot"
                    } else {
                        "partial_live_source_backed"
                    }
                } else {
                    "fallback_only"
                },
                "claim_boundary": claim_boundary,
            }),
        );
    }
}

impl LspServer {
    #[cfg(feature = "workspace")]
    fn append_workspace_edits(&self, workspace_edit: &mut Value, uri: &str, mut edits: Vec<Value>) {
        if edits.is_empty() {
            return;
        }
        if let Some(existing) = workspace_edit["changes"][uri].as_array_mut() {
            existing.append(&mut edits);
        } else {
            workspace_edit["changes"][uri] = Value::Array(edits);
        }
    }

    #[cfg(feature = "workspace")]
    fn read_workspace_text(&self, uri: &str) -> Option<String> {
        // Priority 1: actively-open document (editor is authoritative).
        if let Some(doc) = self.documents.lock().get(uri) {
            return Some(doc.text.clone());
        }

        // Priority 2: workspace index document store (content from the last
        // time the file was indexed; avoids a synchronous disk read for files
        // that were open and then closed within the session).
        if let Some(coordinator) = self.coordinator() {
            if let Some(doc) = coordinator.index().document_store().get(uri) {
                return Some(doc.text.clone());
            }
        }

        // Priority 3: read from disk.  `workspace/willRenameFiles` is a
        // workspace-wide refactoring operation; returning edits for files not
        // currently open in the editor is explicitly correct per LSP 3.17 §3.17
        // (the client requests cross-file edits and applies them).  This path
        // is restricted to workspace-root-relative paths by the caller and is
        // bounded to files that `find_dependents` already knows about.
        uri_to_fs_path(uri).and_then(|path| read_text_with_encoding_fallback(&path).ok())
    }
}

#[cfg(feature = "workspace")]
fn build_module_rename_workspace_edits(original: &str, updated: &str) -> Vec<Value> {
    let original_lines: Vec<&str> = original.split('\n').collect();
    let updated_lines: Vec<&str> = updated.split('\n').collect();

    debug_assert_eq!(
        original_lines.len(),
        updated_lines.len(),
        "module rename planning should not change line counts"
    );

    original_lines
        .iter()
        .zip(updated_lines.iter())
        .enumerate()
        .filter_map(|(line, (old_line, new_line))| {
            if old_line == new_line {
                return None;
            }

            Some(json!({
                "range": {
                    "start": {
                        "line": line,
                        "character": 0,
                    },
                    "end": {
                        "line": line,
                        "character": old_line.len(),
                    }
                },
                "newText": new_line
            }))
        })
        .collect()
}

#[cfg(feature = "workspace")]
fn collect_delete_target_module_names(
    index: &perl_parser::workspace_index::WorkspaceIndex,
    uri: &str,
) -> std::collections::BTreeSet<String> {
    let mut module_names = std::collections::BTreeSet::new();
    let path_module_name = path_to_module_name(uri);
    if !path_module_name.is_empty() {
        module_names.insert(path_module_name);
    }

    for symbol in index.file_symbols(uri) {
        if matches!(symbol.kind, SymbolKind::Package | SymbolKind::Class | SymbolKind::Role) {
            if let Some(module_name) = symbol
                .qualified_name
                .clone()
                .or_else(|| (!symbol.name.is_empty()).then_some(symbol.name.clone()))
            {
                module_names.insert(module_name);
            }
        }
    }

    module_names
}

#[cfg(feature = "workspace")]
fn collect_cross_file_delete_dependents(
    index: &perl_parser::workspace_index::WorkspaceIndex,
    uri: &str,
    deleting_uris: &std::collections::HashSet<String>,
) -> std::collections::BTreeSet<String> {
    let normalized_uri = perl_parser::workspace_index::uri_key(uri);
    let module_names = collect_delete_target_module_names(index, uri);
    let mut dependents = std::collections::BTreeSet::new();
    for module_name in module_names {
        for dependent_uri in index.find_dependents(&module_name) {
            if dependent_uri != normalized_uri && !deleting_uris.contains(&dependent_uri) {
                dependents.insert(dependent_uri);
            }
        }
    }

    dependents
}

#[cfg(feature = "workspace")]
fn collect_open_document_delete_dependents(
    index: &perl_parser::workspace_index::WorkspaceIndex,
    uri: &str,
    deleting_uris: &std::collections::HashSet<String>,
    open_documents: &[(String, String)],
) -> std::collections::BTreeSet<String> {
    let normalized_uri = perl_parser::workspace_index::uri_key(uri);
    let module_names = collect_delete_target_module_names(index, uri);
    let mut dependents = std::collections::BTreeSet::new();

    for (doc_uri, text) in open_documents {
        let normalized_doc_uri = perl_parser::workspace_index::uri_key(doc_uri);
        if normalized_doc_uri == normalized_uri || deleting_uris.contains(&normalized_doc_uri) {
            continue;
        }

        if module_names.iter().any(|module_name| {
            !plan_module_rename_edits(text, module_name, "__PerlLspDeleteProbe__").is_empty()
        }) {
            dependents.insert(normalized_doc_uri);
        }
    }

    dependents
}

#[cfg(feature = "workspace")]
fn collect_symbol_reference_delete_dependents(
    index: &perl_parser::workspace_index::WorkspaceIndex,
    uri: &str,
    deleting_uris: &std::collections::HashSet<String>,
) -> std::collections::BTreeSet<String> {
    let normalized_uri = perl_parser::workspace_index::uri_key(uri);
    let mut dependents = std::collections::BTreeSet::new();

    for symbol in index.file_symbols(uri) {
        let mut names = std::collections::BTreeSet::new();
        if !symbol.name.is_empty() {
            names.insert(symbol.name.clone());
        }
        if let Some(qualified_name) = symbol.qualified_name {
            if !qualified_name.is_empty() {
                names.insert(qualified_name);
            }
        }

        for symbol_name in names {
            for reference in index.find_references(&symbol_name) {
                let reference_uri = perl_parser::workspace_index::uri_key(&reference.uri);
                if reference_uri != normalized_uri && !deleting_uris.contains(&reference_uri) {
                    dependents.insert(reference_uri);
                }
            }
        }
    }

    dependents
}

fn short_uri(uri: &str) -> String {
    Url::parse(uri)
        .ok()
        .and_then(|parsed| {
            parsed.path_segments().and_then(|mut s| s.next_back().map(str::to_owned))
        })
        .filter(|tail| !tail.is_empty())
        .unwrap_or_else(|| uri.to_string())
}

/// Return `true` if `module_name` appears in `text` as a whole-identifier token.
///
/// This prevents false-positive rename warnings when a short module name (e.g. `"Base"`)
/// appears as a suffix of an unrelated longer identifier (e.g. `"Database"`).  The check
/// rejects any match that is immediately preceded or followed by a word character (`\w`)
/// or a colon (`:`), both of which extend a Perl identifier.
pub(super) fn module_name_appears_in_text(text: &str, module_name: &str) -> bool {
    if module_name.is_empty() {
        return false;
    }
    let name_len = module_name.len();
    let text_len = text.len();

    let mut start = 0usize;
    while start + name_len <= text_len {
        if let Some(pos) = text[start..].find(module_name) {
            let abs = start + pos;
            // Check character before the match
            let before_ok = abs == 0 || {
                let c = text[..abs].chars().next_back();
                c.is_none_or(|c| !is_perl_identifier_continue(c))
            };
            // Check character after the match
            let after_ok = abs + name_len >= text_len || {
                let c = text[abs + name_len..].chars().next();
                c.is_none_or(|c| !is_perl_identifier_continue(c))
            };
            if before_ok && after_ok {
                return true;
            }
            start = abs + 1;
        } else {
            break;
        }
    }
    false
}

fn is_perl_identifier_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == ':'
}

/// Convert a file path to a Perl module name
pub(super) fn path_to_module_name(uri: &str) -> String {
    #[cfg(feature = "workspace")]
    let path =
        uri_to_fs_path(uri).and_then(|p| p.to_str().map(|s| s.to_string())).unwrap_or_else(|| {
            // Fallback to trim_start_matches for backward compatibility
            uri.trim_start_matches("file://").to_string()
        });
    #[cfg(not(feature = "workspace"))]
    let path = uri.trim_start_matches("file://").to_string();

    file_path_to_module_name(&path)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "workspace")]
    use super::read_text_with_encoding_fallback;
    use super::{LspServer, module_name_appears_in_text};
    use serde_json::json;
    #[cfg(feature = "workspace")]
    use std::io::Write;

    #[test]
    fn test_module_name_appears_exact_match() {
        assert!(module_name_appears_in_text("use MyBase;", "MyBase"));
    }

    #[test]
    fn test_module_name_appears_as_suffix_rejected() {
        // "Base" must NOT match inside "Database" (false-positive guard)
        assert!(!module_name_appears_in_text("use Database;", "Base"));
    }

    #[test]
    fn test_module_name_appears_as_prefix_rejected() {
        // "Foo" must NOT match inside "FooBar"
        assert!(!module_name_appears_in_text("FooBar->method()", "Foo"));
    }

    #[test]
    fn test_module_name_appears_with_colon_boundary() {
        // "Bar" must NOT match when followed by "::" (it is a namespace prefix, not a standalone name)
        assert!(!module_name_appears_in_text("Foo::Bar::Baz", "Bar"));
    }

    #[test]
    fn test_module_name_appears_qualified_name() {
        // "Foo::Bar" should match as a whole module path
        assert!(module_name_appears_in_text("use Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn test_module_name_appears_in_string_literal() {
        // Module name inside a single-quoted string counts as a reference
        assert!(module_name_appears_in_text("use parent 'MyBase';", "MyBase"));
    }

    #[test]
    fn test_module_name_empty_returns_false() {
        assert!(!module_name_appears_in_text("anything", ""));
    }

    #[test]
    fn test_module_name_unicode_letter_before_rejected() {
        // Unicode letters still extend identifiers; do not match inside "ÅBase".
        assert!(!module_name_appears_in_text("use ÅBase;", "Base"));
    }

    #[test]
    fn test_module_name_unicode_letter_after_rejected() {
        // Unicode letters still extend identifiers; do not match inside "BaseΔ".
        assert!(!module_name_appears_in_text("use BaseΔ;", "Base"));
    }

    #[test]
    fn did_change_workspace_folders_clears_pending_workspace_configuration_requests() {
        let server = LspServer::new();
        server.pending_workspace_configuration_requests.lock().insert(
            7,
            crate::runtime::PendingWorkspaceConfigurationRequest {
                folder_uris: vec!["file:///tmp/folder-a".to_string()],
                includes_global_item: true,
                created_at: std::time::Instant::now(),
            },
        );

        let result = server.handle_did_change_workspace_folders(Some(json!({
            "event": {
                "added": [
                    { "uri": "file:///tmp/folder-b", "name": "folder-b" }
                ],
                "removed": []
            }
        })));

        assert!(result.is_ok());
        assert!(server.pending_workspace_configuration_requests.lock().is_empty());
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn watched_file_deleted_clears_raw_and_normalized_uri_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("delete_variant.pm");
        let source = "package Delete::Variant;\nsub gone { 1 }\n1;\n";
        std::fs::write(&path, source)?;
        let normalized_uri = url::Url::from_file_path(&path).map_err(|_| "invalid file path")?;
        let normalized_uri = normalized_uri.to_string();
        let file_path_part = normalized_uri.strip_prefix("file:///").ok_or("expected file URI")?;
        let raw_uri = format!("file://localhost/{file_path_part}");

        assert_ne!(raw_uri, normalized_uri);
        assert_eq!(server.normalize_uri_key(&raw_uri), server.normalize_uri_key(&normalized_uri));

        server.did_open(json!({
            "textDocument": {
                "uri": normalized_uri,
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        }))?;
        let in_flight_token = server.new_parse_token(&normalized_uri);
        server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
            uri: normalized_uri.clone(),
            document_version: 1,
            line: 0,
            character: 0,
        });
        if let Some(coordinator) = server.coordinator() {
            coordinator
                .index()
                .index_file(url::Url::parse(&normalized_uri)?, source.to_string())?;
            assert!(!coordinator.index().file_symbols(&normalized_uri).is_empty());
            assert!(coordinator.index().document_store().get(&normalized_uri).is_some());
        }

        server.handle_did_change_watched_files(Some(json!({
            "changes": [
                { "uri": raw_uri, "type": 3 }
            ]
        })))?;

        assert!(in_flight_token.load(std::sync::atomic::Ordering::Relaxed));
        let after = server.memory_state_snapshot();
        assert_eq!(after.documents, 0);
        assert_eq!(after.open_text_bytes, 0);
        assert_eq!(after.parse_cancel_flags, 0);
        assert_eq!(after.stream_sessions, 0);
        if let Some(coordinator) = server.coordinator() {
            assert!(coordinator.index().file_symbols(&normalized_uri).is_empty());
            assert!(coordinator.index().file_symbols(&raw_uri).is_empty());
            assert!(coordinator.index().document_store().get(&normalized_uri).is_none());
            assert!(coordinator.index().document_store().get(&raw_uri).is_none());
        }

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn bulk_file_watcher_churn_drains_pressure_and_delete_state()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::runtime::file_watcher_debounce::FileWatcherDebouncer;
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let server = LspServer::with_io(Box::new(std::io::empty()), Box::new(Vec::<u8>::new()));
        let delivered = Arc::new(parking_lot::Mutex::new(Vec::<String>::new()));
        let delivered_for_worker = Arc::clone(&delivered);
        server.install_file_watcher_debouncer(FileWatcherDebouncer::with_interval(
            Duration::from_millis(150),
            move |uris| {
                delivered_for_worker.lock().extend(uris);
            },
        ));

        let dir = tempfile::tempdir()?;
        let mut uris = Vec::new();
        for i in 0..20 {
            let path = dir.path().join(format!("BulkWatcher{i}.pm"));
            let source = format!("package BulkWatcher{i};\nsub value {{ {i} }}\n1;\n");
            std::fs::write(&path, &source)?;
            let uri = url::Url::from_file_path(&path).map_err(|_| "invalid file path")?;
            let uri = uri.to_string();

            server.did_open(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": source
                }
            }))?;
            server.new_parse_token(&uri);
            server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
                uri: uri.clone(),
                document_version: 1,
                line: i,
                character: 0,
            });
            if let Some(coordinator) = server.coordinator() {
                coordinator.index().index_file(url::Url::parse(&uri)?, source)?;
                assert!(
                    !coordinator.index().file_symbols(&uri).is_empty(),
                    "workspace index should contain {uri} before delete"
                );
            }
            uris.push(uri);
        }

        let changes = uris.iter().map(|uri| json!({ "uri": uri, "type": 2 })).collect::<Vec<_>>();
        server.handle_did_change_watched_files(Some(json!({ "changes": changes })))?;

        let mut max_pending = 0usize;
        let pressure_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < pressure_deadline {
            let pending = server.runtime_pressure_snapshot().file_watcher_pending_uris;
            max_pending = max_pending.max(pending);
            if pending == uris.len() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            max_pending > 1,
            "bulk watched-file changes should raise file watcher pressure, max pending {max_pending}"
        );

        let drain_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < drain_deadline {
            if server.runtime_pressure_snapshot().file_watcher_pending_uris == 0
                && delivered.lock().len() == uris.len()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(server.runtime_pressure_snapshot().file_watcher_pending_uris, 0);

        let delivered_uris = delivered.lock().clone();
        assert_eq!(delivered_uris.len(), uris.len());
        server.handle_watched_file_batch(delivered_uris);

        for uri in &uris {
            if let Some(path) = perl_uri::uri_to_fs_path(uri) {
                std::fs::remove_file(path)?;
            }
        }
        let deletes = uris.iter().map(|uri| json!({ "uri": uri, "type": 3 })).collect::<Vec<_>>();
        server.handle_did_change_watched_files(Some(json!({ "changes": deletes })))?;

        for _ in 0..100 {
            let pressure = server.runtime_pressure_snapshot();
            if pressure.pending_index_tasks == 0 && pressure.file_watcher_pending_uris == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let memory = server.memory_state_snapshot();
        assert_eq!(memory.documents, 0);
        assert_eq!(memory.open_text_bytes, 0);
        assert_eq!(memory.parse_cancel_flags, 0);
        assert_eq!(memory.stream_sessions, 0);
        assert_eq!(memory.pending_index_tasks, 0);
        assert_eq!(server.runtime_pressure_snapshot().file_watcher_pending_uris, 0);

        if let Some(coordinator) = server.coordinator() {
            for uri in &uris {
                assert!(coordinator.index().file_symbols(uri).is_empty());
                assert!(coordinator.index().document_store().get(uri).is_none());
            }
        }

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn workspace_folder_removal_evicts_open_docs_under_root()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let dir = tempfile::tempdir()?;
        let removed_root = dir.path().join("removed");
        let kept_root = dir.path().join("kept");
        std::fs::create_dir_all(&removed_root)?;
        std::fs::create_dir_all(&kept_root)?;

        let removed_path = removed_root.join("inside.pm");
        let kept_path = kept_root.join("outside.pm");
        let removed_source = "package Removed::Inside;\nsub inside { 1 }\n1;\n";
        let kept_source = "package Kept::Outside;\nsub outside { 1 }\n1;\n";
        std::fs::write(&removed_path, removed_source)?;
        std::fs::write(&kept_path, kept_source)?;

        let removed_folder_uri =
            url::Url::from_directory_path(&removed_root).map_err(|_| "invalid removed root")?;
        let kept_folder_uri =
            url::Url::from_directory_path(&kept_root).map_err(|_| "invalid kept root")?;
        let removed_uri = url::Url::from_file_path(&removed_path)
            .map_err(|_| "invalid removed path")?
            .to_string();
        let kept_uri =
            url::Url::from_file_path(&kept_path).map_err(|_| "invalid kept path")?.to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(
                removed_folder_uri.to_string(),
            ),
        );
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(
                kept_folder_uri.to_string(),
            ),
        );

        server.did_open(json!({
            "textDocument": {
                "uri": removed_uri,
                "languageId": "perl",
                "version": 1,
                "text": removed_source
            }
        }))?;
        server.did_open(json!({
            "textDocument": {
                "uri": kept_uri,
                "languageId": "perl",
                "version": 1,
                "text": kept_source
            }
        }))?;
        let removed_token = server.new_parse_token(&removed_uri);
        let kept_token = server.new_parse_token(&kept_uri);
        server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
            uri: removed_uri.clone(),
            document_version: 1,
            line: 0,
            character: 0,
        });
        server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
            uri: kept_uri.clone(),
            document_version: 1,
            line: 0,
            character: 0,
        });

        server.handle_did_change_workspace_folders(Some(json!({
            "event": {
                "added": [],
                "removed": [
                    { "uri": removed_folder_uri.to_string(), "name": "removed" }
                ]
            }
        })))?;

        assert!(removed_token.load(std::sync::atomic::Ordering::Relaxed));
        assert!(!kept_token.load(std::sync::atomic::Ordering::Relaxed));
        {
            let documents = server.documents.lock();
            assert!(!documents.contains_key(&server.normalize_uri_key(&removed_uri)));
            assert!(documents.contains_key(&server.normalize_uri_key(&kept_uri)));
        }
        assert!(!server.parse_cancel_flags.lock().contains_key(&removed_uri));
        assert!(server.parse_cancel_flags.lock().contains_key(&kept_uri));
        assert_eq!(server.stream_sessions().len(), 1);
        assert!(
            server
                .workspace_folders
                .lock()
                .iter()
                .all(|folder| { folder.uri != removed_folder_uri.to_string() })
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn read_text_with_encoding_fallback_decodes_utf16le_bom()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("utf16le.pm");
        let text = "my $x = \"π\";";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::File::create(&path)?.write_all(&bytes)?;

        let read = read_text_with_encoding_fallback(&path)?;
        assert_eq!(read, text);
        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn read_text_with_encoding_fallback_strips_utf8_bom() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("utf8_bom.pm");
        std::fs::write(&path, [0xEF, 0xBB, 0xBF, b'p', b'a', b'c', b'k', b'a', b'g', b'e'])?;

        let read = read_text_with_encoding_fallback(&path)?;
        assert_eq!(read, "package");
        Ok(())
    }

    /// Regression: a UTF-16 LE BOM followed by an odd number of payload
    /// bytes must not panic or silently truncate. We fall back to lossy
    /// UTF-8 of the original bytes so the caller still gets something
    /// reasonable to index.
    #[cfg(feature = "workspace")]
    #[test]
    fn read_text_with_encoding_fallback_handles_odd_length_utf16le()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("odd_utf16le.pm");
        // BOM (2 bytes) + 3 payload bytes = odd-length UTF-16 payload.
        std::fs::write(&path, [0xFF, 0xFE, 0x6D, 0x00, 0x79])?;

        let read = read_text_with_encoding_fallback(&path)?;
        // Must return something (not panic) — the replacement string is
        // lossy but deterministic.
        assert!(!read.is_empty());
        Ok(())
    }

    /// Regression: a UTF-16 BE BOM followed by an odd number of payload
    /// bytes must not panic or silently truncate.
    #[cfg(feature = "workspace")]
    #[test]
    fn read_text_with_encoding_fallback_handles_odd_length_utf16be()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("odd_utf16be.pm");
        // BOM (2 bytes) + 3 payload bytes = odd-length UTF-16 payload.
        std::fs::write(&path, [0xFE, 0xFF, 0x00, 0x6D, 0x00])?;

        let read = read_text_with_encoding_fallback(&path)?;
        assert!(!read.is_empty());
        Ok(())
    }

    /// Edge case: empty file should decode to an empty string without panic.
    #[cfg(feature = "workspace")]
    #[test]
    fn read_text_with_encoding_fallback_handles_empty_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("empty.pm");
        std::fs::write(&path, [])?;

        let read = read_text_with_encoding_fallback(&path)?;
        assert_eq!(read, "", "Empty file should decode to empty string");
        Ok(())
    }

    /// Edge case: file with only a UTF-8 BOM and no content should decode
    /// to an empty string (BOM is stripped, nothing remains).
    #[cfg(feature = "workspace")]
    #[test]
    fn read_text_with_encoding_fallback_handles_bom_only_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("bom_only.pm");
        std::fs::write(&path, [0xEF, 0xBB, 0xBF])?;

        let read = read_text_with_encoding_fallback(&path)?;
        assert_eq!(read, "", "BOM-only file should decode to empty string after BOM strip");
        Ok(())
    }
}
