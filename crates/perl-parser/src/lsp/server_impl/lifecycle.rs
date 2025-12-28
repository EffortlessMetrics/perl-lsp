//! LSP server lifecycle management
//!
//! Handles initialize/shutdown and workspace configuration.

use super::*;

impl LspServer {
    /// Handle initialize request
    pub(super) fn handle_initialize(
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
                let mut folders = self.workspace_folders.lock().unwrap();
                for folder in workspace_folders {
                    if let Some(uri) = folder["uri"].as_str() {
                        eprintln!("Initialized with workspace folder: {}", uri);
                        folders.push(uri.to_string());
                    }
                }
            } else if let Some(root_uri) = params.get("rootUri").and_then(|u| u.as_str()) {
                // Fallback to rootUri if workspaceFolders is not provided
                let mut folders = self.workspace_folders.lock().unwrap();
                eprintln!("Initialized with root URI: {}", root_uri);
                folders.push(root_uri.to_string());
                // Also set the root path for module resolution
                self.set_root_uri(root_uri);
            }
        }

        // Check for available tools quickly with a timeout
        // Use which/where command which is much faster than spawning the actual tools
        let has_perltidy = if cfg!(target_os = "windows") {
            std::process::Command::new("where")
                .arg("perltidy")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("which")
                .arg("perltidy")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };

        let has_perlcritic = if cfg!(target_os = "windows") {
            std::process::Command::new("where")
                .arg("perlcritic")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("which")
                .arg("perlcritic")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };

        eprintln!("Tool availability: perltidy={}, perlcritic={}", has_perltidy, has_perlcritic);

        // Check if incremental parsing is enabled
        let sync_kind =
            if cfg!(feature = "incremental") && std::env::var("PERL_LSP_INCREMENTAL").is_ok() {
                2 // Incremental sync
            } else {
                1 // Full document sync
            };

        // Build capabilities using catalog-driven approach
        let mut build_flags = if cfg!(feature = "lsp-ga-lock") {
            crate::capabilities::BuildFlags::ga_lock()
        } else {
            crate::capabilities::BuildFlags::production()
        };

        // Set formatting flags based on perltidy availability
        if has_perltidy {
            build_flags.formatting = true;
            build_flags.range_formatting = true;
        }

        // Persist advertised features for gating
        let features = build_flags.to_advertised_features();
        *self.advertised_features.lock().unwrap() = features.clone();

        // Generate capabilities from build flags
        let server_caps = crate::capabilities::capabilities_for(build_flags);
        let mut capabilities = serde_json::to_value(&server_caps).unwrap();

        // Add fields not yet in lsp-types 0.97
        capabilities["positionEncoding"] = json!("utf-16");
        capabilities["declarationProvider"] = json!(true);
        capabilities["documentHighlightProvider"] = json!(true);
        if features.type_hierarchy {
            capabilities["typeHierarchyProvider"] = json!(true);
        }
        if features.call_hierarchy {
            capabilities["callHierarchyProvider"] = json!(true);
        }

        // Override text document sync with more detailed options
        capabilities["textDocumentSync"] = json!({
            "openClose": true,
            "change": sync_kind,
            "willSave": true,
            "willSaveWaitUntil": false,
            "save": { "includeText": true }
        });

        Ok(Some(json!({
            "capabilities": capabilities,
            "serverInfo": {
                "name": "perl-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })))
    }

    /// Register file watchers for Perl files
    pub(super) fn register_file_watchers_async(&self) {
        use lsp_types::{
            DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, Registration,
            RegistrationParams, WatchKind,
            notification::{DidChangeWatchedFiles, Notification},
        };

        if !self.advertised_features.lock().unwrap().workspace_symbol {
            return;
        }

        let watchers = vec![
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.pl".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.pm".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.t".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
        ];

        let opts = DidChangeWatchedFilesRegistrationOptions { watchers };
        let reg = Registration {
            id: "perl-didChangeWatchedFiles".into(),
            method: <DidChangeWatchedFiles as Notification>::METHOD.to_string(),
            register_options: Some(serde_json::to_value(opts).unwrap_or(Value::Null)),
        };

        let params = RegistrationParams { registrations: vec![reg] };

        // Send the registration request without waiting for a response
        // Use a random ID since we're not tracking the response
        let request_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let request = json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Number(serde_json::Number::from(request_id)),
            "method": "client/registerCapability",
            "params": params
        });

        // Send using the proper output mechanism with explicit error logging
        // (previously silenced with .ok() which hid client disconnect issues)
        match self.output.lock() {
            Ok(mut output) => match serde_json::to_string(&request) {
                Ok(msg) => {
                    if let Err(e) = write!(output, "Content-Length: {}\r\n\r\n{}", msg.len(), msg) {
                        eprintln!("[perl-lsp] Failed to write file watcher request: {}", e);
                        return;
                    }
                    if let Err(e) = output.flush() {
                        eprintln!("[perl-lsp] Failed to flush file watcher request: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[perl-lsp] Failed to serialize file watcher request: {}", e);
                }
            },
            Err(e) => {
                eprintln!(
                    "[perl-lsp] Could not acquire output lock for file watcher registration: {}",
                    e
                );
            }
        }
    }

    /// Send a notification when the workspace index is ready
    pub(super) fn send_index_ready_notification(&self) -> io::Result<()> {
        #[cfg(feature = "workspace")]
        let has_symbols =
            self.workspace_index.as_ref().map(|idx| idx.has_symbols()).unwrap_or(false);

        #[cfg(not(feature = "workspace"))]
        let has_symbols = false;

        self.notify(
            "perl-lsp/index-ready",
            json!({
                "ready": has_symbols
            }),
        )
    }

    /// Set the root path from the root URI during initialization
    pub(super) fn set_root_uri(&self, root_uri: &str) {
        let root_path = Url::parse(root_uri).ok().and_then(|u| u.to_file_path().ok());
        *self.root_path.lock().unwrap() = root_path;
    }

    /// Enhanced module path resolver using root_path
    pub(super) fn resolve_module_path(&self, module: &str) -> Option<PathBuf> {
        let root = self.root_path.lock().unwrap().clone()?;
        let rel = module.replace("::", "/") + ".pm";
        for base in ["lib", "inc", "local/lib/perl5"] {
            let p = root.join(base).join(&rel);
            if p.exists() {
                return Some(p);
            }
        }
        // Best-effort even if not present (for test workspaces)
        Some(root.join("lib").join(rel))
    }

    /// Resolve a module name to a file path URI
    pub(super) fn resolve_module_to_path(&self, module_name: &str) -> Option<String> {
        use std::time::{Duration, Instant};

        // Convert Module::Name to Module/Name.pm
        let relative_path = format!("{}.pm", module_name.replace("::", "/"));

        // First check if we have the document already opened (fast path)
        let documents = self.documents.lock().unwrap();
        for (uri, _doc) in documents.iter() {
            if uri.ends_with(&relative_path) {
                return Some(uri.clone());
            }
        }
        drop(documents);

        // Set a timeout for file system operations
        let start_time = Instant::now();
        let timeout = Duration::from_millis(50); // Reduced timeout for faster response

        // Get workspace folders from initialization
        let workspace_folders = self.workspace_folders.lock().unwrap().clone();

        // Only check workspace-local directories to avoid blocking
        let search_dirs = ["lib", ".", "local/lib/perl5"];

        for workspace_folder in workspace_folders.iter() {
            // Early timeout check
            if start_time.elapsed() > timeout {
                eprintln!(
                    "Module resolution timeout for: {} (elapsed: {:?})",
                    module_name,
                    start_time.elapsed()
                );
                return None;
            }

            // Parse the workspace folder URI to get the file path
            let workspace_path = if workspace_folder.starts_with("file://") {
                workspace_folder.strip_prefix("file://").unwrap_or(workspace_folder)
            } else {
                workspace_folder
            };

            for dir in &search_dirs {
                let full_path = if *dir == "." {
                    format!("{}/{}", workspace_path, relative_path)
                } else {
                    format!("{}/{}/{}", workspace_path, dir, relative_path)
                };

                // Check timeout before each FS operation
                if start_time.elapsed() > timeout {
                    return None;
                }

                // Use metadata() instead of exists() as it's slightly more predictable
                // and we can potentially wrap this in a timeout later
                match std::fs::metadata(&full_path) {
                    Ok(meta) if meta.is_file() => {
                        return Some(format!("file://{}", full_path));
                    }
                    _ => {
                        // File doesn't exist or isn't a regular file, continue
                    }
                }

                // Final timeout check
                if start_time.elapsed() > timeout {
                    return None;
                }
            }
        }

        // Don't check system paths (@INC) to avoid blocking on network filesystems
        None
    }
}
