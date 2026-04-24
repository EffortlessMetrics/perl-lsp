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
use crate::state::workspace_symbol_cap;
use perl_module::path::file_path_to_module_name;
use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};
#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{DegradationReason, EarlyExitReason, ResourceKind, SymbolKind};
#[cfg(feature = "workspace")]
use perl_parser_core::source_file::{is_perl_source_path, is_perl_source_uri};
use perl_workspace::folder::extract_workspace_folder_change;
#[cfg(feature = "workspace")]
use perl_workspace::ignore::is_skipped_dir_name;
#[cfg(feature = "workspace")]
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "workspace")]
use std::time::Instant;
#[cfg(feature = "workspace")]
use url::Url;

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

#[cfg(feature = "workspace")]
fn send_index_ready_notification(outbound: &super::outbound::OutboundSender, ready: bool) {
    if let Err(e) = outbound.send_notification("perl-lsp/index-ready", json!({ "ready": ready })) {
        tracing::warn!(error = %e, "Failed to send index-ready notification");
    }
}

/// Token used for workspace indexing progress notifications.
#[cfg(feature = "workspace")]
const WORKSPACE_INDEX_PROGRESS_TOKEN: &str = "workspace-index";

/// Send `window/workDoneProgress/create` to register the indexing token.
///
/// This is a fire-and-forget request — the client response is not awaited.
/// Per LSP 3.15+ the server must create the token before sending `$/progress`.
#[cfg(feature = "workspace")]
fn send_progress_create(outbound: &super::outbound::OutboundSender, request_id: i64) {
    if let Err(e) = outbound.send_request(
        request_id,
        "window/workDoneProgress/create",
        json!({ "token": WORKSPACE_INDEX_PROGRESS_TOKEN }),
    ) {
        tracing::warn!(error = %e, "Failed to send workDoneProgress/create");
    }
}

/// Send a `$/progress` begin notification for workspace indexing.
#[cfg(feature = "workspace")]
fn send_progress_begin(outbound: &super::outbound::OutboundSender) {
    if let Err(e) = outbound.send_notification(
        "$/progress",
        json!({
            "token": WORKSPACE_INDEX_PROGRESS_TOKEN,
            "value": {
                "kind": "begin",
                "title": "Indexing workspace",
                "cancellable": false,
                "percentage": 0
            }
        }),
    ) {
        tracing::warn!(error = %e, "Failed to send progress begin");
    }
}

/// Send a `$/progress` report notification for workspace indexing.
#[cfg(feature = "workspace")]
fn send_progress_report(outbound: &super::outbound::OutboundSender, indexed: usize, total: usize) {
    let percentage = if total > 0 { (indexed * 100 / total).min(99) as u32 } else { 0 };
    let message = format!("Indexed {} of {} files", indexed, total);
    if let Err(e) = outbound.send_notification(
        "$/progress",
        json!({
            "token": WORKSPACE_INDEX_PROGRESS_TOKEN,
            "value": {
                "kind": "report",
                "message": message,
                "percentage": percentage
            }
        }),
    ) {
        tracing::warn!(error = %e, "Failed to send progress report");
    }
}

/// Send a `$/progress` end notification for workspace indexing.
#[cfg(feature = "workspace")]
fn send_progress_end(outbound: &super::outbound::OutboundSender, message: &str) {
    if let Err(e) = outbound.send_notification(
        "$/progress",
        json!({
            "token": WORKSPACE_INDEX_PROGRESS_TOKEN,
            "value": {
                "kind": "end",
                "message": message
            }
        }),
    ) {
        tracing::warn!(error = %e, "Failed to send progress end");
    }
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

/// Read source text from disk with basic encoding fallbacks.
///
/// Behavior:
/// - UTF-8 BOM (`EF BB BF`) is removed.
/// - UTF-16 LE/BE with BOM is decoded.
/// - Other content first tries strict UTF-8, then falls back to lossy UTF-8.
///
/// Odd-length payloads after a UTF-16 BOM fall back to lossy UTF-8 decoding
/// of the original bytes rather than silently dropping the trailing byte.
#[cfg(feature = "workspace")]
fn read_text_with_encoding_fallback(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(String::from_utf8_lossy(&bytes[3..]).into_owned());
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            // Odd-length UTF-16 payload; fall back to lossy UTF-8 of the
            // full original bytes rather than truncating the trailing byte.
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }
        let units: Vec<u16> =
            payload.chunks_exact(2).map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }
        let units: Vec<u16> =
            payload.chunks_exact(2).map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(err) => Ok(String::from_utf8_lossy(&err.into_bytes()).into_owned()),
    }
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
        let Some(id) = params.get("id").and_then(|value| value.as_i64()) else {
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
        let global_settings = if pending.includes_global_item { results.first() } else { None };
        let folder_results_start = usize::from(pending.includes_global_item);

        let mut folders = self.workspace_folders.lock();
        for (idx, folder_uri) in pending.folder_uris.iter().enumerate() {
            let Some(folder) = folders.iter_mut().find(|folder| &folder.uri == folder_uri) else {
                continue;
            };

            let mut effective_config = perl_lsp_rs_core::config::WorkspaceConfig::default();
            if let Some(project_config) = &folder.project_config {
                project_config.apply_to_workspace_config(&mut effective_config);
            }

            if let Some(global_settings) = global_settings {
                effective_config.update_from_value(global_settings);
            }
            if let Some(perl_settings) = results.get(folder_results_start + idx) {
                effective_config.update_from_value(perl_settings);
            } else {
                tracing::warn!(
                    request_id = id,
                    folder_uri = %folder_uri,
                    "workspace/configuration response missing folder item; using TOML/default config for folder"
                );
            }

            folder.effective_workspace_config = effective_config;
        }
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
        let query =
            params.as_ref().and_then(|p| p.get("query")).and_then(|q| q.as_str()).unwrap_or("");
        let cap = workspace_symbol_cap();

        tracing::debug!(query, cap, "Workspace symbol search v2");

        // Use routing helper for lifecycle-aware dispatch
        #[cfg(feature = "workspace")]
        {
            let access_mode = route_index_access(self.coordinator());

            match access_mode {
                IndexAccessMode::Full(coordinator) => {
                    // Full query path: use workspace index
                    let symbols = coordinator.index().search_symbols(query);

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

                    if !lsp_symbols.is_empty() {
                        tracing::debug!(
                            count = lsp_symbols.len(),
                            "Workspace symbol: returned results from index (Ready state)"
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
                        let symbols = coordinator.index().search_symbols(query);
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
        let mut all_symbols = Vec::new();

        // Collect lightweight snapshots without holding lock during iteration.
        // Only clone the fields needed for symbol extraction (uri, text, ast Arc),
        // avoiding expensive Rope, ParentMap, LineStartsCache, and parse_errors clones.
        let docs_snapshot: Vec<(String, String, Option<Arc<perl_parser::ast::Node>>)> = {
            let documents = self.documents.lock();
            documents.iter().map(|(k, v)| (k.clone(), v.text.clone(), v.ast.clone())).collect()
        };

        // Pre-compute lowercased query once, outside the document loop
        let query_lower = query.to_lowercase();

        for (i, (uri, text, ast)) in docs_snapshot.iter().enumerate() {
            // Cooperative yield every 8 documents
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }

            // Early exit if we've hit the result cap
            if all_symbols.len() >= cap {
                break;
            }

            if let Some(ast) = ast {
                let doc_symbols = self.extract_document_symbols(ast, text, uri);

                for sym in doc_symbols {
                    if sym.name.to_lowercase().contains(&query_lower) {
                        all_symbols.push(sym);
                        if all_symbols.len() >= cap {
                            break;
                        }
                    }
                }
            } else {
                // Text-based fallback when AST is not available
                let text_symbols = self.extract_text_based_symbols(text, uri, query);
                let remaining = cap.saturating_sub(all_symbols.len());
                all_symbols.extend(text_symbols.into_iter().take(remaining));
            }
        }

        // Truncate to cap in case we went slightly over
        all_symbols.truncate(cap);
        tracing::debug!(
            count = all_symbols.len(),
            "Workspace symbol: returned results from open documents"
        );
        Ok(Some(json!(all_symbols)))
    }

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
        let query =
            params.as_ref().and_then(|p| p.get("query")).and_then(|q| q.as_str()).unwrap_or("");

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

        let mut symbols = provider.search(query, &source_map);
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

    /// Handle workspace/didChangeConfiguration notification
    ///
    /// Updates both ServerConfig and WorkspaceConfig when the client
    /// notifies of configuration changes.
    pub(super) fn handle_did_change_configuration(&self, params: Option<Value>) {
        if let Some(params) = params {
            if let Some(settings) = params.get("settings") {
                tracing::debug!("Configuration changed, updating server settings");

                // Read perl settings once and update both configs
                if let Some(perl) = settings.get("perl") {
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
                        new_enabled != cfg.perlcritic_enabled
                            || new_severity != cfg.perlcritic_severity
                            || new_profile != cfg.perlcritic_profile
                            || new_theme != cfg.perlcritic_theme
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

                    // Remove from index
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.index().remove_file(&uri);
                    }

                    // Remove from document store
                    {
                        let mut documents = self.documents.lock();
                        documents.remove(&uri);
                    }

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

        // Re-index the file if it is a Perl source file
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            let workspace_index = coordinator.index();
            if is_perl_source_uri(uri) {
                if let Some(path) = uri_to_fs_path(uri) {
                    match read_text_with_encoding_fallback(&path) {
                        Ok(content) => {
                            if let Ok(url) = url::Url::parse(uri) {
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
                        Err(e) => {
                            tracing::debug!(
                                "Failed to read file for re-indexing ({}): {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        // Also update our internal document store if the document is open
        #[cfg(feature = "workspace")]
        {
            let mut documents = self.documents.lock();
            if let Some(doc) = self.get_document_mut(&mut documents, uri) {
                if let Some(path) = uri_to_fs_path(uri) {
                    match read_text_with_encoding_fallback(&path) {
                        Ok(content) => {
                            doc.text = content;
                            doc.version += 1;
                            // Clear cached AST so it is regenerated on next access
                            doc.ast = None;
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Failed to read file for document store update ({}): {}",
                                path.display(),
                                e
                            );
                        }
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

                    // Remove from workspace index
                    // Note: Mutation operation - use coordinator with lifecycle tracking
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_change(uri);
                        coordinator.index().remove_file(uri);
                        coordinator.notify_parse_complete(uri);
                    }

                    // Remove from document store
                    {
                        let mut documents = self.documents.lock();
                        documents.remove(uri);
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

                        // Also remove documents from the removed workspace
                        let mut documents = self.documents.lock();
                        let docs_to_remove: Vec<String> = documents
                            .keys()
                            .filter(|doc_uri| doc_uri.starts_with(uri))
                            .cloned()
                            .collect();

                        for doc_uri in docs_to_remove {
                            tracing::debug!(uri = %doc_uri, "Removing document from removed workspace");
                            documents.remove(&doc_uri);
                        }
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

                        // Remove files from removed folders
                        for removed_uri in &change.removed {
                            coordinator.index().remove_folder(removed_uri);
                        }
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
        if let Some(doc) = self.documents.lock().get(uri) {
            return Some(doc.text.clone());
        }

        if let Some(coordinator) = self.coordinator() {
            if let Some(doc) = coordinator.index().document_store().get(uri) {
                return Some(doc.text.clone());
            }
        }

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
