//! LSP capabilities handling
//!
//! Handles client capability parsing and server capabilities construction.

use super::super::*;
use perl_workspace_folder::{extract_workspace_folder_uris, root_path_to_file_uri};
use serde_json::{Value, json};

impl LspServer {
    /// Handle initialize request
    pub(crate) fn handle_initialize(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Atomically check and set initialize_requested
        if self
            .initialize_requested
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(JsonRpcError {
                code: -32600, // InvalidRequest per LSP spec 3.17
                message: "initialize may only be sent once".to_string(),
                data: None,
            });
        }

        // Parse client capabilities
        if let Some(params) = &params {
            // Take lock once to write all capabilities
            {
                let mut caps = self.client_capabilities.lock();

                caps.declaration_link_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("declaration"))
                    .and_then(|d| d.get("linkSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                caps.definition_link_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("definition"))
                    .and_then(|d| d.get("linkSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                caps.type_definition_link_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("typeDefinition"))
                    .and_then(|d| d.get("linkSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                caps.implementation_link_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("implementation"))
                    .and_then(|d| d.get("linkSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                // Check if client supports dynamic registration for file watching
                caps.dynamic_registration_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("workspace"))
                    .and_then(|w| w.get("didChangeWatchedFiles"))
                    .and_then(|d| d.get("dynamicRegistration"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                // Check if client supports server-to-client workspace/configuration requests
                caps.workspace_configuration_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("workspace"))
                    .and_then(|w| w.get("configuration"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                // Check if client supports snippet syntax in completion items
                caps.snippet_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("completion"))
                    .and_then(|comp| comp.get("completionItem"))
                    .and_then(|ci| ci.get("snippetSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                // Check if client supports markdown message content in diagnostics (LSP 3.18)
                caps.markup_message_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("diagnostic"))
                    .and_then(|d| d.get("markupMessageSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                // Check if client supports refresh requests for various features
                if let Some(cap_val) = params.get("capabilities") {
                    // workspace/codeLens/refresh
                    caps.code_lens_refresh_support = cap_val
                        .pointer("/workspace/codeLens/refreshSupport")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // workspace/semanticTokens/refresh
                    caps.semantic_tokens_refresh_support = cap_val
                        .pointer("/workspace/semanticTokens/refreshSupport")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // workspace/inlayHint/refresh
                    caps.inlay_hint_refresh_support = cap_val
                        .pointer("/workspace/inlayHint/refreshSupport")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // textDocument/inlayHint
                    caps.inlay_hint_support = cap_val
                        .pointer("/textDocument/inlayHint/staticRegistration")
                        .or_else(|| cap_val.pointer("/textDocument/inlayHint"))
                        .is_some();

                    // workspace/inlineValue/refresh
                    caps.inline_value_refresh_support = cap_val
                        .pointer("/workspace/inlineValue/refreshSupport")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // workspace/diagnostic/refresh
                    caps.diagnostic_refresh_support = cap_val
                        .pointer("/workspace/diagnostic/refreshSupport")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // workspace/foldingRange/refresh
                    caps.folding_range_refresh_support = cap_val
                        .pointer("/workspace/foldingRange/refreshSupport")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // window/showDocument
                    caps.show_document_support = cap_val
                        .pointer("/window/showDocument/support")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // window/workDoneProgress
                    caps.work_done_progress_support = cap_val
                        .pointer("/window/workDoneProgress")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // textDocument/inlayHint resolveSupport.properties
                    // Collect the property names the client can resolve (e.g. "label.location")
                    if let Some(properties) =
                        cap_val.pointer("/textDocument/inlayHint/resolveSupport/properties")
                    {
                        let props: std::collections::HashSet<String> = properties
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        caps.inlay_hint_resolve_support = Some(props);
                    }
                }
            } // caps lock released here

            // Check if client supports pull diagnostics
            let supports_pull = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("diagnostic"))
                .is_some();

            if supports_pull {
                self.client_supports_pull_diags.store(true, Ordering::Relaxed);
                tracing::debug!(
                    "Client supports pull diagnostics - suppressing automatic publishing"
                );
            }

            // Initialize workspace folders
            if let Some(workspace_folders) =
                params.get("workspaceFolders").and_then(|f| f.as_array())
            {
                let mut folders = self.workspace_folders.lock();
                for uri in extract_workspace_folder_uris(workspace_folders) {
                    tracing::debug!(uri, "Initialized with workspace folder");
                    let mut folder =
                        super::super::workspace_folder::WorkspaceFolderState::new(uri.clone());
                    if let Some(path) = super::super::source_path_from_uri(&uri) {
                        folder = folder.with_path(path);
                    }
                    folders.push(folder);
                }
            } else if let Some(root_uri) = params.get("rootUri").and_then(|u| u.as_str()) {
                // Fallback to rootUri if workspaceFolders is not provided
                let mut folders = self.workspace_folders.lock();
                tracing::debug!(root_uri, "Initialized with root URI");
                let mut folder =
                    super::super::workspace_folder::WorkspaceFolderState::new(root_uri.to_string());
                if let Some(path) = super::super::source_path_from_uri(root_uri) {
                    folder = folder.with_path(path);
                }
                folders.push(folder);
                // Also set the root path for module resolution
                self.set_root_uri(root_uri);
            } else if let Some(root_path) = params.get("rootPath").and_then(|p| p.as_str()) {
                // Legacy fallback: rootPath is deprecated since LSP 3.0 but still sent by some clients
                tracing::debug!(root_path, "Initialized with legacy rootPath");
                let root_uri = root_path_to_file_uri(root_path);
                let mut folders = self.workspace_folders.lock();
                folders.push(super::super::workspace_folder::WorkspaceFolderState::new(
                    root_uri.clone(),
                ));
                self.set_root_uri(&root_uri);
            }
        }

        // Load .perl-lsp.toml from workspace root (base layer; LSP config overrides later)
        self.load_and_apply_project_config();
        self.request_workspace_configuration();

        // Construct the AI inline-completion backend if enabled in config
        self.refresh_ai_backend();

        // Check for available tools quickly with a timeout
        // Use which/where command which is much faster than spawning the actual tools
        let has_perltidy = self.detect_tool("perltidy");
        let has_perlcritic = self.detect_tool("perlcritic");

        tracing::debug!(perltidy = has_perltidy, perlcritic = has_perlcritic, "Tool availability");

        // TextDocumentSyncKind::Full (1): the server always reparses the full
        // document on every didChange notification.  Advertising Incremental (2)
        // would be inaccurate — we do not maintain incremental AST state between
        // edits; we rebuild the entire AST from the complete document text each time.
        let sync_kind = 1;

        // Build capabilities using catalog-driven approach
        let profile = self.feature_profile();
        let mut build_flags = profile.runtime_flags(has_perltidy);

        // Read user-disabled features from initializationOptions
        if let Some(init_opts) = params.as_ref().and_then(|p| p.get("initializationOptions")) {
            if let Some(disabled) = init_opts.get("disabledFeatures").and_then(|v| v.as_array()) {
                for id in disabled.iter().filter_map(|v| v.as_str()) {
                    apply_disabled_feature_id(&mut build_flags, id);
                }
            }
        }

        // Persist advertised features for gating
        let features = build_flags.to_advertised_features();
        *self.advertised_features.lock() = features.clone();

        // Generate capabilities from build flags
        let server_caps = crate::protocol::capabilities::capabilities_for(build_flags);
        let mut capabilities = serde_json::to_value(&server_caps).map_err(|e| {
            crate::protocol::internal_error(&format!(
                "Failed to serialize server capabilities: {}",
                e
            ))
        })?;

        // Add fields not yet in lsp-types 0.97
        capabilities["positionEncoding"] = json!("utf-16");
        if features.declaration {
            capabilities["declarationProvider"] = json!(true);
        }
        if features.type_hierarchy {
            capabilities["typeHierarchyProvider"] = json!(true);
        }

        // Override text document sync with more detailed options
        capabilities["textDocumentSync"] = json!({
            "openClose": true,
            "change": sync_kind,
            "willSave": true,
            "willSaveWaitUntil": true,
            "save": { "includeText": true }
        });

        // Workspace capabilities: folders, file operations, and content schemes
        capabilities["workspace"] = json!({
            "workspaceFolders": {
                "supported": true,
                "changeNotifications": true
            },
            "fileOperations": {
                "willCreate": { "filters": [
                    { "pattern": { "glob": "**/*.pl" } },
                    { "pattern": { "glob": "**/*.pm" } },
                    { "pattern": { "glob": "**/*.t" } },
                    { "pattern": { "glob": "**/*.psgi" } }
                ]},
                "didCreate": { "filters": [
                    { "pattern": { "glob": "**/*.pl" } },
                    { "pattern": { "glob": "**/*.pm" } },
                    { "pattern": { "glob": "**/*.t" } },
                    { "pattern": { "glob": "**/*.psgi" } }
                ]},
                "willRename": { "filters": [
                    { "pattern": { "glob": "**/*.pl" } },
                    { "pattern": { "glob": "**/*.pm" } },
                    { "pattern": { "glob": "**/*.t" } },
                    { "pattern": { "glob": "**/*.psgi" } }
                ]},
                "didRename": { "filters": [
                    { "pattern": { "glob": "**/*.pl" } },
                    { "pattern": { "glob": "**/*.pm" } },
                    { "pattern": { "glob": "**/*.t" } },
                    { "pattern": { "glob": "**/*.psgi" } }
                ]},
                "willDelete": { "filters": [
                    { "pattern": { "glob": "**/*.pl" } },
                    { "pattern": { "glob": "**/*.pm" } },
                    { "pattern": { "glob": "**/*.t" } },
                    { "pattern": { "glob": "**/*.psgi" } }
                ]},
                "didDelete": { "filters": [
                    { "pattern": { "glob": "**/*.pl" } },
                    { "pattern": { "glob": "**/*.pm" } },
                    { "pattern": { "glob": "**/*.t" } },
                    { "pattern": { "glob": "**/*.psgi" } }
                ]}
            },
            "textDocumentContent": {
                "schemes": ["perldoc"]
            }
        });

        // Advertise experimental custom requests
        capabilities["experimental"] = json!({
            "perlInlineCompletionStream": true
        });

        Ok(Some(json!({
            "capabilities": capabilities,
            "serverInfo": {
                "name": "perl-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })))
    }
}

/// Zero the `BuildFlags` field corresponding to the given feature ID.
///
/// Feature IDs use the canonical `lsp.*` format from `perl-lsp-feature-ids`
/// (e.g. `"lsp.semantic_tokens"`). Unknown IDs are logged and ignored.
pub(crate) fn apply_disabled_feature_id(
    flags: &mut crate::protocol::capabilities::BuildFlags,
    id: &str,
) {
    match id {
        "lsp.completion" => flags.completion = false,
        "lsp.hover" => flags.hover = false,
        "lsp.definition" => flags.definition = false,
        "lsp.declaration" => flags.declaration = false,
        "lsp.references" => flags.references = false,
        "lsp.document_symbol" => flags.document_symbol = false,
        "lsp.workspace_symbol" => flags.workspace_symbol = false,
        "lsp.code_action" => flags.code_actions = false,
        "lsp.code_lens" => flags.code_lens = false,
        "lsp.rename" => flags.rename = false,
        "lsp.folding_range" => flags.folding_range = false,
        "lsp.selection_range" => flags.selection_ranges = false,
        "lsp.linked_editing_range" => flags.linked_editing = false,
        "lsp.inlay_hint" => flags.inlay_hints = false,
        "lsp.semantic_tokens" => flags.semantic_tokens = false,
        "lsp.call_hierarchy" => flags.call_hierarchy = false,
        "lsp.type_hierarchy" => flags.type_hierarchy = false,
        "lsp.pull_diagnostics" => flags.pull_diagnostics = false,
        "lsp.document_color" => flags.document_color = false,
        "lsp.signature_help" => flags.signature_help = false,
        "lsp.document_highlight" => flags.document_highlight = false,
        "lsp.formatting" => flags.formatting = false,
        "lsp.range_formatting" | "lsp.ranges_formatting" => flags.range_formatting = false,
        "lsp.on_type_formatting" => flags.on_type_formatting = false,
        "lsp.document_link" => flags.document_links = false,
        "lsp.inline_completion" => flags.inline_completion = false,
        "lsp.inline_value" => flags.inline_values = false,
        "lsp.notebook_document_sync" => flags.notebook_document_sync = false,
        "lsp.notebook_cell_execution" => flags.notebook_cell_execution = false,
        "lsp.implementation" => flags.implementation = false,
        "lsp.type_definition" => flags.type_definition = false,
        "lsp.execute_command" => flags.execute_command = false,
        "lsp.moniker" => flags.moniker = false,
        unknown => tracing::warn!(id = unknown, "Unknown disabledFeatures ID ignored"),
    }
}

#[cfg(test)]
mod tests {
    use super::apply_disabled_feature_id;
    use crate::protocol::capabilities::BuildFlags;

    #[test]
    fn apply_disabled_feature_id_zeros_correct_field() {
        let mut flags = BuildFlags::all();
        apply_disabled_feature_id(&mut flags, "lsp.semantic_tokens");
        assert!(!flags.semantic_tokens);
        assert!(flags.completion, "other flags must be unchanged");
    }

    #[test]
    fn apply_disabled_feature_id_unknown_is_noop() {
        let mut flags = BuildFlags::all();
        let before = flags.clone();
        apply_disabled_feature_id(&mut flags, "lsp.does_not_exist");
        assert_eq!(flags, before, "unknown ID must not mutate flags");
    }

    #[test]
    fn apply_disabled_feature_id_execute_command_zeros_field() {
        let mut flags = BuildFlags::all();
        apply_disabled_feature_id(&mut flags, "lsp.execute_command");
        assert!(!flags.execute_command, "lsp.execute_command must zero execute_command field");
        assert!(flags.completion, "other flags must be unchanged");
    }

    #[test]
    fn apply_disabled_feature_id_moniker_zeros_field() {
        let mut flags = BuildFlags::all();
        apply_disabled_feature_id(&mut flags, "lsp.moniker");
        assert!(!flags.moniker, "lsp.moniker must zero moniker field");
    }

    #[test]
    fn apply_disabled_feature_id_notebook_cell_execution_zeros_field() {
        let mut flags = BuildFlags::all();
        apply_disabled_feature_id(&mut flags, "lsp.notebook_cell_execution");
        assert!(
            !flags.notebook_cell_execution,
            "lsp.notebook_cell_execution must zero notebook_cell_execution field"
        );
    }

    /// All feature IDs emitted by BuildFlags::to_feature_ids() must have a match arm.
    /// This test will fail if a new field is added to BuildFlags with a feature ID
    /// but no corresponding arm in apply_disabled_feature_id.
    #[test]
    fn all_feature_ids_have_match_arm() {
        let all_ids = BuildFlags::all().to_feature_ids();
        for id in &all_ids {
            let mut before = BuildFlags::all();
            apply_disabled_feature_id(&mut before, id);
            let still_all = before == BuildFlags::all();
            assert!(
                !still_all,
                "feature ID '{id}' emitted by to_feature_ids() has no match arm in \
                 apply_disabled_feature_id — add one to keep the two in sync"
            );
        }
    }
}
