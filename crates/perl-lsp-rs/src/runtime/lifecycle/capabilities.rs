//! LSP capabilities handling
//!
//! Handles client capability parsing and server capabilities construction.

use super::super::*;
use perl_workspace::folder::{extract_workspace_folder_uris, root_path_to_file_uri};
use serde_json::{Value, json};

fn is_opencode_client(params: &Value) -> bool {
    params
        .get("clientInfo")
        .and_then(|info| info.get("name"))
        .and_then(|name| name.as_str())
        .map(|name| name.to_ascii_lowercase().contains("opencode"))
        .unwrap_or(false)
}

fn is_jetbrains_client(params: &Value) -> bool {
    params
        .get("clientInfo")
        .and_then(|info| info.get("name"))
        .and_then(|name| name.as_str())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("jetbrains") || lower.contains("intellij") || lower.contains("idea")
        })
        .unwrap_or(false)
}

fn merge_experimental_capability(capabilities: &mut Value, key: &str, value: Value) {
    if !capabilities.get("experimental").is_some_and(Value::is_object) {
        let Some(capabilities_object) = capabilities.as_object_mut() else {
            tracing::warn!("Failed to merge experimental capability into non-object capabilities");
            return;
        };
        capabilities_object.insert("experimental".to_string(), json!({}));
    }

    let Some(experimental) = capabilities.get_mut("experimental").and_then(Value::as_object_mut)
    else {
        tracing::warn!("Failed to merge experimental capability into non-object value");
        return;
    };

    experimental.insert(key.to_string(), value);
}

fn code_action_documentation_entries() -> Value {
    json!([
        {
            "kind": "quickfix",
            "command": {
                "title": "Explain Perl quick fixes",
                "command": "perl.explainProviderDecision",
                "arguments": [{
                    "provider": "diagnostics",
                    "receipt_id": "docs/specs/PLSP-SPEC-0029-lsp-318-conformance-boundary.md#code-action-documentation",
                    "scenario": "lsp_318_code_action_documentation_quickfix"
                }]
            }
        },
        {
            "kind": "refactor",
            "command": {
                "title": "Explain Perl refactors",
                "command": "perl.explainProviderDecision",
                "arguments": [{
                    "provider": "rename",
                    "receipt_id": "docs/specs/PLSP-SPEC-0029-lsp-318-conformance-boundary.md#code-action-documentation",
                    "scenario": "lsp_318_code_action_documentation_refactor"
                }]
            }
        },
        {
            "kind": "source.fixAll",
            "command": {
                "title": "Explain Perl fix-all actions",
                "command": "perl.explainProviderDecision",
                "arguments": [{
                    "provider": "diagnostics",
                    "receipt_id": "docs/specs/PLSP-SPEC-0029-lsp-318-conformance-boundary.md#code-action-documentation",
                    "scenario": "lsp_318_code_action_documentation_fix_all"
                }]
            }
        }
    ])
}

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
                caps.file_watcher_relative_pattern_support = params
                    .pointer("/capabilities/workspace/didChangeWatchedFiles/relativePatternSupport")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                caps.inline_completion_support =
                    params.pointer("/capabilities/textDocument/inlineCompletion").is_some();
                caps.inline_completion_dynamic_registration_support = params
                    .pointer("/capabilities/textDocument/inlineCompletion/dynamicRegistration")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                // JetBrains-family IDEs (IntelliJ IDEA, etc.) advertise dynamic watcher
                // registration but their registration flow is unreliable and can degrade LSP
                // startup behavior. Force-disable for these clients regardless of what the
                // capabilities object claims.
                if is_jetbrains_client(params) {
                    caps.dynamic_registration_support = false;
                }

                caps.workspace_configuration_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("workspace"))
                    .and_then(|w| w.get("configuration"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                caps.workspace_folders_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("workspace"))
                    .and_then(|w| w.get("workspaceFolders"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                // Check if client supports snippet syntax in completion items.
                //
                // Spec-compliant clients send this under
                // textDocument.completion.completionItem.*, but some generic
                // clients flatten these booleans directly onto
                // textDocument.completion. Support both shapes.
                caps.snippet_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("completion"))
                    .and_then(|comp| {
                        comp.get("completionItem")
                            .and_then(|ci| ci.get("snippetSupport"))
                            .or_else(|| comp.get("snippetSupport"))
                    })
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                caps.completion_commit_characters_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("completion"))
                    .and_then(|comp| {
                        comp.get("completionItem")
                            .and_then(|ci| ci.get("commitCharactersSupport"))
                            .or_else(|| comp.get("commitCharactersSupport"))
                    })
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                caps.label_details_support = params
                    .get("capabilities")
                    .and_then(|c| c.get("textDocument"))
                    .and_then(|td| td.get("completion"))
                    .and_then(|comp| comp.get("completionItem"))
                    .and_then(|ci| ci.get("labelDetailsSupport"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                caps.completion_list_item_defaults_data_support = params
                    .pointer("/capabilities/textDocument/completion/completionList/itemDefaults")
                    .and_then(Value::as_array)
                    .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("data")));
                caps.completion_list_apply_kind_support = params
                    .pointer(
                        "/capabilities/textDocument/completion/completionList/applyKindSupport",
                    )
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                caps.workspace_edit_document_changes_support = params
                    .pointer("/capabilities/workspace/workspaceEdit/documentChanges")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                caps.workspace_edit_snippet_edit_support = params
                    .pointer("/capabilities/workspace/workspaceEdit/snippetEditSupport")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                caps.code_action_documentation_support = params
                    .pointer("/capabilities/textDocument/codeAction/documentationSupport")
                    .and_then(Value::as_bool)
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

                    // textDocument/codeLens resolveSupport.properties
                    if let Some(properties) =
                        cap_val.pointer("/textDocument/codeLens/resolveSupport/properties")
                    {
                        let props: std::collections::HashSet<String> = properties
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        caps.code_lens_resolve_support = Some(props);
                    }

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

                    caps.inline_completion_support =
                        cap_val.pointer("/textDocument/inlineCompletion").is_some();

                    caps.inline_completion_dynamic_registration_support = cap_val
                        .pointer("/textDocument/inlineCompletion/dynamicRegistration")
                        .and_then(Value::as_bool)
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

            // Check if client supports pull diagnostics.
            //
            // OpenCode currently relies on push diagnostics (publishDiagnostics)
            // even when it advertises textDocument.diagnostic capability.
            // Treat it as push-only to avoid suppressing diagnostics.
            let is_opencode = is_opencode_client(params);
            let supports_pull = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("diagnostic"))
                .is_some();

            if supports_pull && !is_opencode {
                self.client_supports_pull_diags.store(true, Ordering::Relaxed);
                tracing::debug!(
                    "Client supports pull diagnostics - suppressing automatic publishing"
                );
            } else if supports_pull && is_opencode {
                tracing::debug!(
                    "OpenCode client detected - keeping push diagnostics enabled despite \
                     textDocument.diagnostic capability"
                );
            }

            // Initialize workspace folders
            if let Some(workspace_folders) =
                params.get("workspaceFolders").and_then(|f| f.as_array())
            {
                let uris = extract_workspace_folder_uris(workspace_folders);
                if let Some(first_uri) = uris.first() {
                    self.set_root_uri(first_uri);
                }

                let mut folders = self.workspace_folders.lock();
                for uri in uris {
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
                // (including older JetBrains LSP clients).
                tracing::debug!(root_path, "Initialized with legacy rootPath");
                let root_uri = root_path_to_file_uri(root_path);
                let mut folder =
                    super::super::workspace_folder::WorkspaceFolderState::new(root_uri.clone());
                // Preserve filesystem path metadata so project-config loading and other
                // path-based workflows behave the same as rootUri/workspaceFolders initialization.
                folder = folder.with_path(std::path::PathBuf::from(root_path));
                let mut folders = self.workspace_folders.lock();
                folders.push(folder);
                self.set_root_uri(&root_uri);
            } else if let Some(init_options) = params.get("initializationOptions") {
                // Compatibility fallback for clients that place workspace roots in
                // initializationOptions instead of top-level initialize params.
                if let Some(workspace_folders) =
                    init_options.get("workspaceFolders").and_then(|f| f.as_array())
                {
                    let uris = extract_workspace_folder_uris(workspace_folders);
                    // Mirror top-level workspaceFolders: set root URI from first folder.
                    if let Some(first_uri) = uris.first() {
                        self.set_root_uri(first_uri);
                    }
                    let mut folders = self.workspace_folders.lock();
                    for uri in uris {
                        tracing::debug!(
                            uri,
                            "Initialized with workspace folder from initializationOptions"
                        );
                        let mut folder =
                            super::super::workspace_folder::WorkspaceFolderState::new(uri.clone());
                        if let Some(path) = super::super::source_path_from_uri(&uri) {
                            folder = folder.with_path(path);
                        }
                        folders.push(folder);
                    }
                } else if let Some(root_uri) = init_options.get("rootUri").and_then(|u| u.as_str())
                {
                    let mut folders = self.workspace_folders.lock();
                    tracing::debug!(
                        root_uri,
                        "Initialized with root URI from initializationOptions"
                    );
                    let mut folder = super::super::workspace_folder::WorkspaceFolderState::new(
                        root_uri.to_string(),
                    );
                    if let Some(path) = super::super::source_path_from_uri(root_uri) {
                        folder = folder.with_path(path);
                    }
                    folders.push(folder);
                    self.set_root_uri(root_uri);
                } else if let Some(root_path) =
                    init_options.get("rootPath").and_then(|p| p.as_str())
                {
                    tracing::debug!(
                        root_path,
                        "Initialized with legacy rootPath from initializationOptions"
                    );
                    let root_uri = root_path_to_file_uri(root_path);
                    let mut folders = self.workspace_folders.lock();
                    folders.push(super::super::workspace_folder::WorkspaceFolderState::new(
                        root_uri.clone(),
                    ));
                    self.set_root_uri(&root_uri);
                }
            } else if let Ok(cwd) = std::env::current_dir() {
                // Compatibility fallback for lightweight clients (for example Aider)
                // that initialize without workspaceFolders/rootUri/rootPath.
                let cwd_uri = root_path_to_file_uri(&cwd.to_string_lossy());
                let mut folders = self.workspace_folders.lock();
                folders.push(super::super::workspace_folder::WorkspaceFolderState::new(
                    cwd_uri.clone(),
                ));
                self.set_root_uri(&cwd_uri);
                tracing::debug!(cwd_uri, "Initialized with process current directory fallback");
            }
        }

        // Load .perl-lsp.toml from workspace root (base layer; LSP config overrides later)
        self.load_and_apply_project_config();

        // Detect Perl interpreter and surface an actionable error if not found.
        // Runs after config load so that perl-lsp.perl.path is already applied.
        self.check_perl_interpreter();

        // Construct the AI inline-completion backend if enabled in config
        self.refresh_ai_backend();

        // Check for available tools quickly with a timeout
        // Use which/where command which is much faster than spawning the actual tools
        let has_perltidy = self.detect_tool("perltidy");
        let has_perlcritic = self.detect_tool("perlcritic");

        tracing::debug!(perltidy = has_perltidy, perlcritic = has_perlcritic, "Tool availability");

        // TextDocumentSyncKind::Full (1): the server always reparses the full
        // document on every didChange notification.  Advertising Incremental (2)
        // would be inaccurate â€” we do not maintain incremental AST state between
        // edits; we rebuild the entire AST from the complete document text each time.
        let sync_kind = 1;

        // Build capabilities using catalog-driven approach
        let profile = self.feature_profile();
        let mut build_flags = profile.runtime_flags(has_perltidy);

        // Read user-disabled features from initializationOptions.
        //
        // Supported shapes:
        //   1) { "disabledFeatures": [...] }
        //   2) { "perl-lsp": { "disabledFeatures": [...] } }
        //   3) { "perl_lsp": { "disabledFeatures": [...] } }
        //
        // Some generic LSP clients namespace server settings under the server id,
        // while others pass options at the top level.
        if let Some(init_opts) = params.as_ref().and_then(|p| p.get("initializationOptions")) {
            for id in disabled_feature_ids_from_init_options(init_opts) {
                apply_disabled_feature_id(&mut build_flags, id);
            }
        }

        // Persist advertised features for gating
        let features = build_flags.to_advertised_features();
        *self.advertised_features.lock() = features.clone();

        // Generate capabilities from build flags
        //
        // `capabilities_json()` is the static/default capability surface and
        // has no client context. Runtime initialize may remove standard LSP
        // fields such as inlineCompletionProvider when the client asks for
        // dynamic registration for the same selector.
        let mut capabilities =
            crate::protocol::capabilities::capabilities_json(build_flags.clone());
        let inline_completion_dynamic_registration_support =
            self.client_capabilities.lock().inline_completion_dynamic_registration_support;

        match (features.inline_completion, inline_completion_dynamic_registration_support) {
            // LSP 3.18 dynamic registration is an alternate registration mode,
            // not an addition to static registration for the same selector.
            (true, true) => {
                if let Some(capabilities) = capabilities.as_object_mut() {
                    capabilities.remove("inlineCompletionProvider");
                }
            }
            (true, false) => {
                if let Some(capabilities) = capabilities.as_object_mut() {
                    capabilities.insert("inlineCompletionProvider".to_string(), json!({}));
                }
            }
            (false, _) => {
                if let Some(capabilities) = capabilities.as_object_mut() {
                    capabilities.remove("inlineCompletionProvider");
                }
            }
        }

        // Add fields not yet in lsp-types 0.97
        capabilities["positionEncoding"] = json!("utf-16");
        if features.declaration {
            capabilities["declarationProvider"] = json!(true);
        }
        let code_action_documentation_support =
            self.client_capabilities.lock().code_action_documentation_support;
        if features.code_action && code_action_documentation_support {
            if let Some(code_action_provider) =
                capabilities.get_mut("codeActionProvider").and_then(Value::as_object_mut)
            {
                code_action_provider
                    .insert("documentation".to_string(), code_action_documentation_entries());
            } else {
                tracing::warn!(
                    "Cannot advertise CodeAction.documentation; codeActionProvider is not an object"
                );
            }
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
        let workspace_folders_support = self.client_capabilities.lock().workspace_folders_support;
        capabilities["workspace"] = json!({
            "workspaceFolders": {
                "supported": workspace_folders_support,
                "changeNotifications": workspace_folders_support
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
        if features.inline_completion {
            merge_experimental_capability(
                &mut capabilities,
                "perlInlineCompletionStream",
                json!(true),
            );
        }

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

fn disabled_feature_ids_from_init_options(init_opts: &Value) -> Vec<&str> {
    let top_level = init_opts.get("disabledFeatures").and_then(Value::as_array);
    let namespaced_hyphen =
        init_opts.get("perl-lsp").and_then(|v| v.get("disabledFeatures")).and_then(Value::as_array);
    let namespaced_underscore =
        init_opts.get("perl_lsp").and_then(|v| v.get("disabledFeatures")).and_then(Value::as_array);

    top_level
        .into_iter()
        .chain(namespaced_hyphen)
        .chain(namespaced_underscore)
        .flat_map(|entries| entries.iter())
        .filter_map(Value::as_str)
        .collect()
}

#[cfg(test)]
mod init_options_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn disabled_feature_ids_reads_top_level_and_namespaced_options() {
        let init_opts = json!({
            "disabledFeatures": ["lsp.hover", true, 42],
            "perl-lsp": {
                "disabledFeatures": ["lsp.completion", null]
            },
            "perl_lsp": {
                "disabledFeatures": ["lsp.semantic_tokens"]
            }
        });

        let ids = disabled_feature_ids_from_init_options(&init_opts);
        assert_eq!(ids, vec!["lsp.hover", "lsp.completion", "lsp.semantic_tokens"]);
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_disabled_feature_id, is_jetbrains_client, is_opencode_client};
    use crate::LspServer;
    use crate::protocol::capabilities::BuildFlags;
    use perl_workspace::folder::root_path_to_file_uri;
    use serde_json::{Value, json};
    use std::sync::atomic::Ordering;

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
                 apply_disabled_feature_id â€” add one to keep the two in sync"
            );
        }
    }

    #[test]
    fn initialize_with_workspace_folders_sets_root_path_from_first_folder() {
        use std::path::Path;

        let server = LspServer::new();

        // Create platform-appropriate URIs for workspace folders using Url::from_file_path
        #[cfg(windows)]
        let (primary_uri, secondary_uri) = {
            let primary = Path::new("C:\\tmp\\primary");
            let secondary = Path::new("C:\\tmp\\secondary");
            (
                url::Url::from_file_path(primary).unwrap().to_string(),
                url::Url::from_file_path(secondary).unwrap().to_string(),
            )
        };

        #[cfg(not(windows))]
        let (primary_uri, secondary_uri) = {
            let primary_path = Path::new("/tmp/primary");
            let secondary_path = Path::new("/tmp/secondary");
            (
                url::Url::from_file_path(primary_path).unwrap().to_string(),
                url::Url::from_file_path(secondary_path).unwrap().to_string(),
            )
        };

        let params = json!({
            "workspaceFolders": [
                { "uri": primary_uri, "name": "primary" },
                { "uri": secondary_uri, "name": "secondary" }
            ],
            "capabilities": {}
        });

        let result = server.handle_initialize(Some(params));
        assert!(result.is_ok(), "initialize should succeed");

        let root_path = server.root_path.lock();
        assert!(
            root_path.as_ref().is_some_and(|path| path.ends_with("primary")),
            "root path should come from first workspace folder. Got: {:?}",
            root_path
        );
    }

    #[test]
    fn initialize_parses_workspace_configuration_capability() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "workspace": {
                    "configuration": true,
                    "workspaceFolders": true
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(server.client_capabilities.lock().workspace_configuration_support);
        assert!(server.client_capabilities.lock().workspace_folders_support);
    }

    #[test]
    fn initialize_parses_file_watcher_relative_pattern_support() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "workspace": {
                    "didChangeWatchedFiles": {
                        "dynamicRegistration": true,
                        "relativePatternSupport": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        let caps = server.client_capabilities.lock();
        assert!(caps.dynamic_registration_support);
        assert!(caps.file_watcher_relative_pattern_support);
    }

    #[test]
    fn initialize_disables_workspace_folder_server_capability_when_client_lacks_support()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "workspace": {
                    "workspaceFolders": false
                }
            }
        });

        let response =
            server.handle_initialize(Some(params))?.ok_or("initialize should return payload")?;

        let workspace_folders = response
            .pointer("/capabilities/workspace/workspaceFolders/supported")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let change_notifications = response
            .pointer("/capabilities/workspace/workspaceFolders/changeNotifications")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        assert!(!workspace_folders, "server must not advertise unsupported workspace folders");
        assert!(
            !change_notifications,
            "server must not advertise workspace folder change notifications when unsupported"
        );
        Ok(())
    }

    #[test]
    fn initialize_parses_completion_item_capabilities_from_spec_shape() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true,
                            "commitCharactersSupport": true
                        }
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        let caps = server.client_capabilities.lock();
        assert!(caps.snippet_support);
        assert!(caps.completion_commit_characters_support);
    }

    #[test]
    fn initialize_parses_completion_item_capabilities_from_flattened_shape() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "snippetSupport": true,
                        "commitCharactersSupport": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        let caps = server.client_capabilities.lock();
        assert!(caps.snippet_support);
        assert!(caps.completion_commit_characters_support);
    }

    #[test]
    fn initialize_parses_completion_list_item_defaults_data_support() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionList": {
                            "itemDefaults": [
                                "commitCharacters",
                                "editRange",
                                "insertTextFormat",
                                "insertTextMode",
                                "data"
                            ]
                        }
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(server.client_capabilities.lock().completion_list_item_defaults_data_support);
    }

    #[test]
    fn initialize_leaves_completion_list_item_defaults_data_disabled_when_absent() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionList": {
                            "itemDefaults": ["commitCharacters", "insertTextFormat"]
                        }
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(!server.client_capabilities.lock().completion_list_item_defaults_data_support);
    }

    #[test]
    fn initialize_parses_completion_list_apply_kind_support() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionList": {
                            "applyKindSupport": true
                        }
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(server.client_capabilities.lock().completion_list_apply_kind_support);
    }

    #[test]
    fn initialize_leaves_completion_list_apply_kind_disabled_when_absent() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionList": {
                            "itemDefaults": ["data"]
                        }
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(!server.client_capabilities.lock().completion_list_apply_kind_support);
    }

    #[test]
    fn initialize_parses_workspace_edit_snippet_text_edit_support() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "workspace": {
                    "workspaceEdit": {
                        "documentChanges": true,
                        "snippetEditSupport": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));
        let caps = server.client_capabilities.lock();

        assert!(caps.workspace_edit_document_changes_support);
        assert!(caps.workspace_edit_snippet_edit_support);
    }

    #[test]
    fn initialize_leaves_workspace_edit_snippet_text_edit_disabled_when_absent() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "workspace": {
                    "workspaceEdit": {
                        "documentChanges": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));
        let caps = server.client_capabilities.lock();

        assert!(caps.workspace_edit_document_changes_support);
        assert!(!caps.workspace_edit_snippet_edit_support);
    }

    #[test]
    fn initialize_parses_code_action_documentation_support() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "codeAction": {
                        "documentationSupport": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(server.client_capabilities.lock().code_action_documentation_support);
    }

    #[test]
    fn initialize_advertises_code_action_documentation_only_when_supported()
    -> Result<(), Box<dyn std::error::Error>> {
        let unsupported = LspServer::new()
            .handle_initialize(Some(json!({ "capabilities": {} })))?
            .ok_or("initialize should return unsupported-client payload")?;
        assert!(
            unsupported.pointer("/capabilities/codeActionProvider/documentation").is_none(),
            "default clients must not receive CodeAction.documentation: {unsupported}"
        );

        let supported = LspServer::new()
            .handle_initialize(Some(json!({
                "capabilities": {
                    "textDocument": {
                        "codeAction": {
                            "documentationSupport": true
                        }
                    }
                }
            })))?
            .ok_or("initialize should return supported-client payload")?;
        let docs = supported
            .pointer("/capabilities/codeActionProvider/documentation")
            .and_then(Value::as_array)
            .ok_or("supported clients should receive CodeActionOptions.documentation")?;
        assert_eq!(docs.len(), 3, "expected quickfix, refactor, and source.fixAll docs");
        Ok(())
    }

    #[test]
    fn lsp4ij_inline_completion_dynamic_registration_shape_is_parsed() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {
                "textDocument": {
                    "inlineCompletion": {
                        "dynamicRegistration": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));
        let caps = server.client_capabilities.lock();
        assert!(caps.inline_completion_support);
        assert!(caps.inline_completion_dynamic_registration_support);
    }

    #[test]
    fn initialize_uses_current_directory_when_root_is_missing() {
        let server = LspServer::new();
        let params = json!({
            "capabilities": {}
        });

        let _ = server.handle_initialize(Some(params));

        let folders = server.workspace_folders.lock();
        assert_eq!(folders.len(), 1, "missing roots should fall back to current directory");

        let expected_uri =
            std::env::current_dir().ok().map(|cwd| root_path_to_file_uri(&cwd.to_string_lossy()));
        assert_eq!(
            folders[0].uri,
            expected_uri.unwrap_or_default(),
            "workspace folder should match current directory fallback URI"
        );
    }

    /// Guard: cwd fallback must NOT fire when a top-level rootUri is present.
    #[test]
    fn initialize_cwd_fallback_not_used_when_root_uri_present() {
        let server = LspServer::new();
        let params = json!({
            "rootUri": "file:///explicit-workspace"
        });

        let _ = server.handle_initialize(Some(params));

        let folders = server.workspace_folders.lock();
        assert_eq!(folders.len(), 1, "must create exactly one workspace folder from rootUri");
        assert_eq!(
            folders[0].uri, "file:///explicit-workspace",
            "cwd fallback must not override an explicitly provided rootUri"
        );
    }

    #[test]
    fn opencode_client_detection_is_case_insensitive() {
        let params = json!({
            "clientInfo": {
                "name": "OpenCode"
            }
        });
        assert!(is_opencode_client(&params));
    }

    #[test]
    fn jetbrains_client_detection_matches_jetbrains_intellij_idea_names() {
        for name in &["JetBrains", "IntelliJ IDEA", "idea", "JetBrains Client"] {
            let params = json!({ "clientInfo": { "name": name } });
            assert!(is_jetbrains_client(&params), "should detect JetBrains client: {name}");
        }
        let non_jetbrains = json!({ "clientInfo": { "name": "vscode" } });
        assert!(!is_jetbrains_client(&non_jetbrains));
        assert!(!is_jetbrains_client(&json!({})));
    }

    #[test]
    fn initialize_disables_dynamic_registration_for_jetbrains_clients() {
        let server = LspServer::new();
        let params = json!({
            "clientInfo": {
                "name": "JetBrains"
            },
            "capabilities": {
                "workspace": {
                    "didChangeWatchedFiles": {
                        "dynamicRegistration": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(
            !server.client_capabilities.lock().dynamic_registration_support,
            "JetBrains clients must have dynamic_registration_support forced to false \
             even when the capabilities object claims support"
        );
    }

    #[test]
    fn initialize_preserves_dynamic_registration_for_non_jetbrains_clients() {
        let server = LspServer::new();
        let params = json!({
            "clientInfo": {
                "name": "vscode"
            },
            "capabilities": {
                "workspace": {
                    "didChangeWatchedFiles": {
                        "dynamicRegistration": true
                    }
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(
            server.client_capabilities.lock().dynamic_registration_support,
            "non-JetBrains clients that advertise dynamic registration must have it enabled"
        );
    }

    #[test]
    fn initialize_keeps_push_diagnostics_for_opencode() {
        let server = LspServer::new();
        let params = json!({
            "clientInfo": {
                "name": "opencode"
            },
            "capabilities": {
                "textDocument": {
                    "diagnostic": {}
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(
            !server.client_supports_pull_diags.load(Ordering::Relaxed),
            "opencode should keep push diagnostics enabled"
        );
    }

    #[test]
    fn initialize_enables_pull_diagnostics_for_non_opencode_clients() {
        let server = LspServer::new();
        let params = json!({
            "clientInfo": {
                "name": "vscode"
            },
            "capabilities": {
                "textDocument": {
                    "diagnostic": {}
                }
            }
        });

        let _ = server.handle_initialize(Some(params));

        assert!(
            server.client_supports_pull_diags.load(Ordering::Relaxed),
            "non-opencode clients with textDocument.diagnostic should enable pull diagnostics"
        );
    }

    #[test]
    fn initialize_reads_root_uri_from_initialization_options() {
        let server = LspServer::new();
        let params = json!({
            "initializationOptions": {
                "rootUri": "file:///tmp/claude-workspace"
            }
        });

        let _ = server.handle_initialize(Some(params));

        let folders = server.workspace_folders.lock();
        assert_eq!(
            folders.len(),
            1,
            "must create one workspace folder from initializationOptions.rootUri"
        );
        assert_eq!(folders[0].uri, "file:///tmp/claude-workspace");
    }

    /// Precedence guard: top-level `rootUri` must take priority over
    /// `initializationOptions.rootUri` when both are present.
    #[test]
    fn initialize_top_level_root_uri_takes_precedence_over_initialization_options() {
        let server = LspServer::new();
        let params = json!({
            "rootUri": "file:///top-level-workspace",
            "initializationOptions": {
                "rootUri": "file:///init-options-workspace"
            }
        });

        let _ = server.handle_initialize(Some(params));

        let folders = server.workspace_folders.lock();
        assert_eq!(folders.len(), 1, "must create exactly one workspace folder");
        assert_eq!(
            folders[0].uri, "file:///top-level-workspace",
            "top-level rootUri must take precedence over initializationOptions.rootUri"
        );
    }

    /// Parity guard: initializationOptions.workspaceFolders must also call set_root_uri
    /// for the first folder, matching the behavior of the top-level workspaceFolders branch.
    #[test]
    fn initialize_init_options_workspace_folders_sets_root_path() {
        use std::path::Path;

        let server = LspServer::new();

        // Use platform-appropriate real file URIs so source_path_from_uri can convert them.
        #[cfg(windows)]
        let (primary_uri, secondary_uri) = {
            let primary = Path::new("C:\\tmp\\init-opts-primary");
            let secondary = Path::new("C:\\tmp\\init-opts-secondary");
            (
                url::Url::from_file_path(primary).unwrap().to_string(),
                url::Url::from_file_path(secondary).unwrap().to_string(),
            )
        };
        #[cfg(not(windows))]
        let (primary_uri, secondary_uri) = {
            let primary = Path::new("/tmp/init-opts-primary");
            let secondary = Path::new("/tmp/init-opts-secondary");
            (
                url::Url::from_file_path(primary).unwrap().to_string(),
                url::Url::from_file_path(secondary).unwrap().to_string(),
            )
        };

        let params = json!({
            "initializationOptions": {
                "workspaceFolders": [
                    { "uri": primary_uri, "name": "primary" },
                    { "uri": secondary_uri, "name": "secondary" }
                ]
            }
        });

        let _ = server.handle_initialize(Some(params));

        // Workspace folders must be populated
        let folders = server.workspace_folders.lock();
        assert_eq!(
            folders.len(),
            2,
            "both workspace folders from initializationOptions must be registered"
        );
        drop(folders);

        // root_path must be set from the first folder (module resolution depends on this).
        // This is the parity check — the top-level workspaceFolders branch calls
        // set_root_uri; the initializationOptions branch must do the same.
        let root_path = server.root_path.lock();
        assert!(
            root_path.as_ref().is_some_and(|p| p.ends_with("init-opts-primary")),
            "root_path must be set from first initializationOptions.workspaceFolders entry. Got: {:?}",
            root_path
        );
    }
}
