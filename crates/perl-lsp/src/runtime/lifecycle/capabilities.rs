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
                eprintln!("Client supports pull diagnostics - suppressing automatic publishing");
            }

            // Initialize workspace folders
            if let Some(workspace_folders) =
                params.get("workspaceFolders").and_then(|f| f.as_array())
            {
                let mut folders = self.workspace_folders.lock();
                for uri in extract_workspace_folder_uris(workspace_folders) {
                    eprintln!("Initialized with workspace folder: {}", uri);
                    folders.push(uri);
                }
            } else if let Some(root_uri) = params.get("rootUri").and_then(|u| u.as_str()) {
                // Fallback to rootUri if workspaceFolders is not provided
                let mut folders = self.workspace_folders.lock();
                eprintln!("Initialized with root URI: {}", root_uri);
                folders.push(root_uri.to_string());
                // Also set the root path for module resolution
                self.set_root_uri(root_uri);
            } else if let Some(root_path) = params.get("rootPath").and_then(|p| p.as_str()) {
                // Legacy fallback: rootPath is deprecated since LSP 3.0 but still sent by some clients
                eprintln!("Initialized with legacy rootPath: {}", root_path);
                let root_uri = root_path_to_file_uri(root_path);
                let mut folders = self.workspace_folders.lock();
                folders.push(root_uri.clone());
                self.set_root_uri(&root_uri);
            }
        }

        // Check for available tools quickly with a timeout
        // Use which/where command which is much faster than spawning the actual tools
        let has_perltidy = self.detect_tool("perltidy");
        let has_perlcritic = self.detect_tool("perlcritic");

        eprintln!("Tool availability: perltidy={}, perlcritic={}", has_perltidy, has_perlcritic);

        // Incremental text sync: ropey handles range-based edits correctly
        let sync_kind = 2;

        // Build capabilities using catalog-driven approach
        let profile = self.feature_profile();
        let build_flags = profile.runtime_flags(has_perltidy);

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
        capabilities["declarationProvider"] = json!(true);
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

        Ok(Some(json!({
            "capabilities": capabilities,
            "serverInfo": {
                "name": "perl-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })))
    }
}
