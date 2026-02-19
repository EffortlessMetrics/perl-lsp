//! LSP capabilities handling
//!
//! Handles client capability parsing and server capabilities construction.

use super::super::*;
use serde_json::{Value, json};

impl LspServer {
    /// Handle initialize request
    pub(crate) fn handle_initialize(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Check if already initialized
        if self.initialized {
            return Err(JsonRpcError {
                code: -32600, // InvalidRequest per LSP spec 3.17
                message: "initialize may only be sent once".to_string(),
                data: None,
            });
        }

        // Parse client capabilities
        if let Some(params) = &params {
            self.client_capabilities.declaration_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("declaration"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            self.client_capabilities.definition_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("definition"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            self.client_capabilities.type_definition_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("typeDefinition"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            self.client_capabilities.implementation_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("implementation"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            // Check if client supports dynamic registration for file watching
            self.client_capabilities.dynamic_registration_support = params
                .get("capabilities")
                .and_then(|c| c.get("workspace"))
                .and_then(|w| w.get("didChangeWatchedFiles"))
                .and_then(|d| d.get("dynamicRegistration"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            // Check if client supports snippet syntax in completion items
            self.client_capabilities.snippet_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("completion"))
                .and_then(|comp| comp.get("completionItem"))
                .and_then(|ci| ci.get("snippetSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            // Check if client supports markdown message content in diagnostics (LSP 3.18)
            self.client_capabilities.markup_message_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("diagnostic"))
                .and_then(|d| d.get("markupMessageSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            // Check if client supports refresh requests for various features
            if let Some(caps) = params.get("capabilities") {
                // workspace/codeLens/refresh
                self.client_capabilities.code_lens_refresh_support = caps
                    .pointer("/workspace/codeLens/refreshSupport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // workspace/semanticTokens/refresh
                self.client_capabilities.semantic_tokens_refresh_support = caps
                    .pointer("/workspace/semanticTokens/refreshSupport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // workspace/inlayHint/refresh
                self.client_capabilities.inlay_hint_refresh_support = caps
                    .pointer("/workspace/inlayHint/refreshSupport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // workspace/inlineValue/refresh
                self.client_capabilities.inline_value_refresh_support = caps
                    .pointer("/workspace/inlineValue/refreshSupport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // workspace/diagnostic/refresh
                self.client_capabilities.diagnostic_refresh_support = caps
                    .pointer("/workspace/diagnostic/refreshSupport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // workspace/foldingRange/refresh
                self.client_capabilities.folding_range_refresh_support = caps
                    .pointer("/workspace/foldingRange/refreshSupport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // window/showDocument
                self.client_capabilities.show_document_support = caps
                    .pointer("/window/showDocument/support")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // window/workDoneProgress
                self.client_capabilities.work_done_progress_support = caps
                    .pointer("/window/workDoneProgress")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
            }

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
                for folder in workspace_folders {
                    if let Some(uri) = folder["uri"].as_str() {
                        eprintln!("Initialized with workspace folder: {}", uri);
                        folders.push(uri.to_string());
                    }
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
                // Convert rootPath to URI format using proper URL encoding
                let path = std::path::Path::new(root_path);
                let root_uri =
                    url::Url::from_file_path(path).map(|u| u.to_string()).unwrap_or_else(|_| {
                        // Fallback for edge cases (e.g., relative paths, UNC paths)
                        if root_path.starts_with('/') {
                            format!("file://{}", root_path)
                        } else {
                            format!("file:///{}", root_path.replace('\\', "/"))
                        }
                    });
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
        let mut build_flags = if cfg!(feature = "lsp-ga-lock") {
            crate::protocol::capabilities::BuildFlags::ga_lock()
        } else {
            crate::protocol::capabilities::BuildFlags::production()
        };

        // Set formatting flags based on perltidy availability
        if has_perltidy {
            build_flags.formatting = true;
            build_flags.range_formatting = true;
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
