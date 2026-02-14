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
#[cfg(feature = "workspace")]
use parking_lot::Mutex;
#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{DegradationReason, EarlyExitReason, ResourceKind};
#[cfg(feature = "workspace")]
use std::io::Write;
#[cfg(feature = "workspace")]
use std::path::Path;
#[cfg(feature = "workspace")]
use std::sync::Arc;
#[cfg(feature = "workspace")]
use std::time::Instant;
#[cfg(feature = "workspace")]
use url::Url;
#[cfg(feature = "workspace")]
use walkdir::WalkDir;

#[cfg(feature = "workspace")]
fn is_perl_source_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "pl" | "pm" | "t" | "psgi")
}

#[cfg(feature = "workspace")]
fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    matches!(name.as_ref(), ".git" | ".hg" | ".svn" | "target" | "node_modules" | ".cache")
}

#[cfg(feature = "workspace")]
fn send_index_ready_notification(output: &Arc<Mutex<Box<dyn Write + Send>>>, ready: bool) {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "perl-lsp/index-ready",
        "params": { "ready": ready }
    });

    if let Ok(notification_str) = serde_json::to_string(&notification) {
        let mut out = output.lock();
        if write!(out, "Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str)
            .is_ok()
        {
            let _ = out.flush();
        }
    }
}

impl LspServer {
    /// Handle workspace/symbol request (v2 implementation with lifecycle-aware dispatch)
    ///
    /// Uses routing helper for state-aware behavior:
    /// - **Ready state**: Full workspace index search with cooperative yielding
    /// - **Building/Degraded state**: Open document search only (partial results)
    pub(super) fn handle_workspace_symbols_v2(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query =
            params.as_ref().and_then(|p| p.get("query")).and_then(|q| q.as_str()).unwrap_or("");
        let cap = workspace_symbol_cap();

        eprintln!("Workspace symbol search v2: '{}' (cap: {})", query, cap);

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
                        eprintln!(
                            "Workspace symbol: returned {} results from index (Ready state)",
                            lsp_symbols.len()
                        );
                        return Ok(Some(json!(lsp_symbols)));
                    }
                    // If index is empty, fall through to open-doc search
                }
                IndexAccessMode::Partial(reason) => {
                    eprintln!("Workspace symbol: {}, using open-doc fallback", reason);
                }
                IndexAccessMode::None => {
                    eprintln!("Workspace symbol: no workspace feature, using open-doc fallback");
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

        // Collect document snapshots without holding lock
        let docs_snapshot: Vec<(String, DocumentState)> = {
            let documents = self.documents.lock();
            documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (i, (uri, doc)) in docs_snapshot.iter().enumerate() {
            // Cooperative yield every 8 documents
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }

            // Early exit if we've hit the result cap
            if all_symbols.len() >= cap {
                break;
            }

            if let Some(ref ast) = doc.ast {
                let doc_symbols = self.extract_document_symbols(ast, &doc.text, uri);
                let query_lower = query.to_lowercase();

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
                let text_symbols = self.extract_text_based_symbols(&doc.text, uri, query);
                let remaining = cap.saturating_sub(all_symbols.len());
                all_symbols.extend(text_symbols.into_iter().take(remaining));
            }
        }

        // Truncate to cap in case we went slightly over
        all_symbols.truncate(cap);
        eprintln!("Workspace symbol: returned {} results from open documents", all_symbols.len());
        Ok(Some(json!(all_symbols)))
    }

    /// Search open documents for symbols (non-workspace stub)
    #[cfg(not(feature = "workspace"))]
    fn search_open_documents_for_symbols(
        &self,
        query: &str,
        _cap: usize,
    ) -> Result<Option<Value>, JsonRpcError> {
        eprintln!("Workspace symbol: no workspace feature, returning empty for query '{}'", query);
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

        eprintln!("Workspace symbol search: '{}'", query);

        // Snapshot documents without holding lock during iteration
        // (Follows same pattern as handle_workspace_symbols_v2)
        let docs_snapshot: Vec<(String, DocumentState)> = {
            let documents = self.documents.lock();
            documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        // Simple synchronous extraction (legacy non-workspace path)
        let mut all_symbols = Vec::new();
        for (uri, doc) in docs_snapshot.iter() {
            if let Some(ref ast) = doc.ast {
                // Extract symbols using document symbol provider
                self.extract_simple_symbols(ast, &doc.text, uri, query, &mut all_symbols);
            }
        }

        eprintln!("Found {} symbols total", all_symbols.len());

        // Convert to JSON for LSP response
        let result = serde_json::to_value(&all_symbols).unwrap_or_else(|_| json!([]));

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

                                // Add scope information if available
                                if let Some(scope) = symbol_table.scopes.get(&sym.scope_id) {
                                    if scope.parent.is_some() {
                                        // Find parent scope's package name
                                        for parent_symbols in symbol_table.symbols.values() {
                                            for parent_sym in parent_symbols {
                                                if parent_sym.scope_id == scope.parent.unwrap_or(0)
                                                    && parent_sym.kind
                                                        == crate::symbol::SymbolKind::Package
                                                {
                                                    resolved["containerName"] =
                                                        json!(parent_sym.name);
                                                    break;
                                                }
                                            }
                                        }
                                    }
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
                        eprintln!("Configuration requested for section: {}", section);

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
                eprintln!("Configuration changed, updating server settings");

                // Read perl settings once and update both configs
                // Support both "perl" (legacy/manual) and "perl-lsp" (extension default) sections
                let perl_settings = settings.get("perl").or_else(|| settings.get("perl-lsp"));

                if let Some(perl) = perl_settings {
                    // Update server config (inlay hints, test runner, perltidy)
                    {
                        let mut config = self.config.lock();
                        config.update_from_value(perl);
                        eprintln!("Updated server config from settings");
                    }

                    // Update workspace config (include paths, @INC)
                    {
                        let mut workspace_config = self.workspace_config.lock();
                        workspace_config.update_from_value(perl);
                        eprintln!("Updated workspace config from settings");
                    }

                    // Trigger client refresh for configuration-dependent features
                    let _ = self.refresh_controller.refresh_all(self);
                }
            }
        }
    }

    /// Handle workspace/didChangeWatchedFiles notification
    ///
    /// Deterministic state transitions:
    /// - File changes notify coordinator (never block handler threads)
    /// - Index updates happen synchronously but quickly
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
            eprintln!("Failed to parse didChangeWatchedFiles params");
            return Ok(None);
        };

        let total_changes = params.changes.len();
        #[cfg(feature = "workspace")]
        let mut processed_changes = 0usize;
        #[cfg(feature = "workspace")]
        let (incremental_budget_ms, budget_start) = match self.coordinator() {
            Some(coord) => (coord.performance_caps().incremental_budget_ms, Instant::now()),
            None => (0, Instant::now()),
        };

        for change in params.changes {
            let uri = change.uri.to_string();
            let change_type = change.typ;

            eprintln!("File change detected: {} (type: {:?})", uri, change_type);

            // Notify coordinator of pending change (never block handler threads)
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_change(&uri);
            }

            match change_type {
                FileChangeType::CREATED => {
                    // Created
                    // Re-index the file if it's a Perl file
                    // Note: Mutation operation - use coordinator.index() directly
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        let workspace_index = coordinator.index();
                        if uri.ends_with(".pl") || uri.ends_with(".pm") || uri.ends_with(".t") {
                            if let Some(path) = uri_to_fs_path(&uri) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(url) = url::Url::parse(&uri) {
                                        let _ = workspace_index.index_file(url, content);
                                        eprintln!("Indexed new file: {}", uri);
                                    }
                                }
                            }
                        }
                    }
                }
                FileChangeType::CHANGED => {
                    // Changed
                    // Re-index the file
                    // Note: Mutation operation - use coordinator.index() directly
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        let workspace_index = coordinator.index();
                        if let Some(path) = uri_to_fs_path(&uri) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(url) = url::Url::parse(&uri) {
                                    // Clear old index data
                                    workspace_index.clear_file(&uri);
                                    // Re-index with new content
                                    let _ = workspace_index.index_file(url, content.clone());
                                }
                            }
                        }
                    }

                    // Also update our internal document store if it exists
                    #[cfg(feature = "workspace")]
                    {
                        let mut documents = self.documents.lock();
                        if let Some(doc) = self.get_document_mut(&mut documents, &uri) {
                            // Note: content variable is only available inside the cfg block above
                            // We'll need to re-read the file or restructure this
                            if let Some(path) = uri_to_fs_path(&uri) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    doc.text = content;
                                    doc.version += 1;
                                    // Clear cached AST
                                    doc.ast = None;
                                }
                            }
                        }
                    }

                    eprintln!("Re-indexed changed file: {}", uri);
                }
                FileChangeType::DELETED => {
                    // Deleted
                    // Remove from index
                    // Note: Mutation operation - use coordinator.index() directly
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.index().remove_file(&uri);
                    }

                    // Remove from document store
                    {
                        let mut documents = self.documents.lock();
                        documents.remove(&uri);
                    }

                    eprintln!("Removed deleted file from index: {}", uri);
                }
                _ => {}
            }

            // Notify coordinator that file processing is complete
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(&uri);
            }

            #[cfg(feature = "workspace")]
            {
                processed_changes += 1;
                if incremental_budget_ms > 0 {
                    let elapsed_ms = budget_start.elapsed().as_millis() as u64;
                    if elapsed_ms > incremental_budget_ms {
                        if let Some(coordinator) = self.coordinator() {
                            coordinator.record_early_exit(
                                EarlyExitReason::IncrementalTimeBudget,
                                elapsed_ms,
                                processed_changes,
                                total_changes,
                            );
                            coordinator.transition_to_degraded(DegradationReason::ScanTimeout {
                                elapsed_ms,
                            });
                        }
                        break;
                    }
                }
            }
        }

        // This is a notification, no response needed
        Ok(None)
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

                for file in files {
                    let Some(old_uri) = file["oldUri"].as_str() else {
                        continue;
                    };
                    let Some(new_uri) = file["newUri"].as_str() else {
                        continue;
                    };

                    eprintln!("File rename: {} -> {}", old_uri, new_uri);

                    // Extract module names from file paths
                    let old_module = path_to_module_name(old_uri);
                    let new_module = path_to_module_name(new_uri);

                    if !old_module.is_empty() && !new_module.is_empty() {
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
                            // Get the document content
                            let documents = self.documents.lock();
                            if let Some(doc) = documents.get(&dependent_uri) {
                                let mut edits = Vec::new();

                                // Find and replace use statements
                                for (line_num, line) in doc.text.lines().enumerate() {
                                    if line.contains(&format!("use {}", old_module))
                                        || line.contains(&format!("require {}", old_module))
                                        || line.contains(&format!("use parent '{}'", old_module))
                                        || line.contains(&format!("use base '{}'", old_module))
                                    {
                                        let new_line = line.replace(&old_module, &new_module);
                                        edits.push(json!({
                                            "range": {
                                                "start": {"line": line_num, "character": 0},
                                                "end": {"line": line_num, "character": line.len()}
                                            },
                                            "newText": new_line
                                        }));
                                    }
                                }

                                if !edits.is_empty() {
                                    workspace_edit["changes"][dependent_uri] = json!(edits);
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
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(url) = url::Url::parse(new_uri) {
                                    let _ = workspace_index.index_file(url, content.clone());
                                }
                            }
                        }
                        coordinator.notify_parse_complete(old_uri);
                        coordinator.notify_parse_complete(new_uri);
                    }
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

                    eprintln!("File deleted: {}", uri);

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
                let _ = self.refresh_controller.refresh_all(self);
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
                for file in files {
                    let Some(uri) = file["uri"].as_str() else {
                        continue;
                    };

                    eprintln!("File will be deleted: {}", uri);
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

                    eprintln!("File will be created: {}", uri);
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

                    eprintln!("File created: {}", uri);

                    // Index the new file if it's a Perl file
                    // Note: Mutation operation - use coordinator with lifecycle tracking
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        if uri.ends_with(".pl") || uri.ends_with(".pm") || uri.ends_with(".t") {
                            if let Some(path) = uri_to_fs_path(uri) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    coordinator.notify_change(uri);
                                    if let Ok(url) = url::Url::parse(uri) {
                                        let _ = coordinator.index().index_file(url, content);
                                        eprintln!("Indexed new file: {}", uri);
                                    }
                                    coordinator.notify_parse_complete(uri);
                                }
                            }
                        }
                    }
                }

                // Trigger client refresh after file creations
                let _ = self.refresh_controller.refresh_all(self);
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

                    eprintln!("File renamed: {} -> {}", old_uri, new_uri);

                    // Update the index for the renamed file
                    // Note: Mutation operation - use coordinator with lifecycle tracking
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        coordinator.notify_change(old_uri);
                        coordinator.notify_change(new_uri);

                        // Remove old file from index
                        coordinator.index().remove_file(old_uri);

                        // Index new file if it's a Perl file
                        if new_uri.ends_with(".pl")
                            || new_uri.ends_with(".pm")
                            || new_uri.ends_with(".t")
                        {
                            if let Some(path) = uri_to_fs_path(new_uri) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(url) = url::Url::parse(new_uri) {
                                        let _ = coordinator.index().index_file(url, content);
                                        eprintln!("Indexed renamed file: {}", new_uri);
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
                let _ = self.refresh_controller.refresh_all(self);
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
                // Handle added folders
                if let Some(added) = event["added"].as_array() {
                    let mut workspace_folders = self.workspace_folders.lock();
                    for folder in added {
                        if let Some(uri) = folder["uri"].as_str() {
                            eprintln!("Added workspace folder: {}", uri);
                            workspace_folders.push(uri.to_string());
                        }
                    }
                }

                // Handle removed folders
                if let Some(removed) = event["removed"].as_array() {
                    let mut workspace_folders = self.workspace_folders.lock();
                    for folder in removed {
                        if let Some(uri) = folder["uri"].as_str() {
                            eprintln!("Removed workspace folder: {}", uri);
                            workspace_folders.retain(|f| f.as_str() != uri);

                            // Also remove documents from the removed workspace
                            let mut documents = self.documents.lock();
                            let docs_to_remove: Vec<String> = documents
                                .keys()
                                .filter(|doc_uri| doc_uri.starts_with(uri))
                                .cloned()
                                .collect();

                            for doc_uri in docs_to_remove {
                                eprintln!("Removing document from removed workspace: {}", doc_uri);
                                documents.remove(&doc_uri);
                            }
                        }
                    }
                }

                // Trigger client refresh after workspace folder changes
                let _ = self.refresh_controller.refresh_all(self);

                // Rebuild workspace index after folder changes
                #[cfg(feature = "workspace")]
                self.start_workspace_indexing();
            }
        }

        Ok(())
    }

    /// Start a background workspace indexing scan
    #[cfg(feature = "workspace")]
    pub(super) fn start_workspace_indexing(&self) {
        let Some(coordinator) = self.coordinator().map(Arc::clone) else {
            return;
        };
        let workspace_folders = self.workspace_folders.lock().clone();
        if workspace_folders.is_empty() {
            return;
        }

        let output = self.output.clone();
        let limits = coordinator.limits().clone();
        let caps = coordinator.performance_caps().clone();

        std::thread::spawn(move || {
            let budget_start = Instant::now();
            coordinator.transition_to_scanning();

            let mut files: Vec<std::path::PathBuf> = Vec::new();
            let mut early_exit: Option<(EarlyExitReason, u64, usize, usize)> = None;

            'scan: for folder_uri in workspace_folders {
                let Some(root) = uri_to_fs_path(&folder_uri) else {
                    continue;
                };

                for entry in WalkDir::new(root)
                    .follow_links(false)
                    .into_iter()
                    .filter_entry(|e| !should_skip_dir(e))
                {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_) => continue,
                    };

                    if entry.file_type().is_file() && is_perl_source_file(entry.path()) {
                        files.push(entry.path().to_path_buf());
                        let total_files = files.len();

                        if total_files.is_multiple_of(64) {
                            coordinator.update_scan_progress(total_files);
                        }

                        let elapsed_ms = budget_start.elapsed().as_millis() as u64;
                        if total_files >= limits.max_files {
                            early_exit =
                                Some((EarlyExitReason::FileLimit, elapsed_ms, 0, total_files));
                            break 'scan;
                        }

                        if elapsed_ms > caps.initial_scan_budget_ms {
                            early_exit = Some((
                                EarlyExitReason::InitialTimeBudget,
                                elapsed_ms,
                                0,
                                total_files,
                            ));
                            break 'scan;
                        }
                    }
                }
            }

            coordinator.update_scan_progress(files.len());
            coordinator.transition_to_indexing(files.len());

            let mut indexed_files = 0usize;
            let total_files = files.len();

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

                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(url) = Url::from_file_path(&path) else {
                    continue;
                };
                if coordinator.index().index_file(url, content).is_ok() {
                    indexed_files += 1;
                    coordinator.update_building_progress(indexed_files);
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
                send_index_ready_notification(&output, false);
            } else {
                let file_count = coordinator.index().file_count();
                let symbol_count = coordinator.index().symbol_count();
                coordinator.transition_to_ready(file_count, symbol_count);
                send_index_ready_notification(&output, true);
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

            eprintln!("Applying workspace edit");

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
                                    let _ = coordinator.index().index_file(url, doc.text.clone());
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
    let path = path.as_str();
    let path = path.trim_end_matches(".pm").trim_end_matches(".pl");

    // Find the lib directory and extract module path
    if let Some(lib_index) = path.rfind("/lib/") {
        let module_path = &path[lib_index + 5..];
        return module_path.replace('/', "::");
    }

    // Fallback: use filename as module name
    if let Some(last_slash) = path.rfind('/') {
        return path[last_slash + 1..].to_string();
    }

    path.to_string()
}
