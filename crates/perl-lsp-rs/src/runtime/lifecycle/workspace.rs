//! Workspace management
//!
//! Handles workspace folders and root URI/path management.

use super::super::*;
use perl_dap::platform::{PerlInterpreterResult, find_perl_interpreter};
use perl_lsp_rs_core::config::WorkspaceConfig;
use std::sync::Once;

/// Fires at most once per LSP session, when Perl is not found anywhere.
static PERL_NOT_FOUND_WARNED: Once = Once::new();

fn perl_not_found_message(configured_path: Option<&str>, searched: &[String]) -> String {
    let searched_str = searched.join(", ");
    if configured_path.is_some_and(|p| !p.is_empty()) {
        format!(
            "perl-lsp: The configured Perl interpreter was not found. \
             Searched: {searched_str}. \
             Check `perl-lsp.perl.path` in your settings and reload the window \
             (Ctrl+Shift+P \u{2192} Developer: Reload Window). \
             If Perl is not installed yet, install it (https://strawberryperl.com on Windows, \
             `brew install perl` on macOS, or your distro package manager on Linux)."
        )
    } else {
        format!(
            "perl-lsp: Perl interpreter not found on PATH. \
             Searched: {searched_str}. \
             Install Perl (https://strawberryperl.com on Windows, \
             `brew install perl` on macOS, or your distro package manager on Linux) \
             and reload the window, or set `perl-lsp.perl.path` in settings."
        )
    }
}

impl LspServer {
    /// Set the root path from the root URI during initialization
    pub(crate) fn set_root_uri(&self, root_uri: &str) {
        let root_path = super::super::source_path_from_uri(root_uri);
        *self.root_path.lock() = root_path;
    }

    /// Detect the Perl interpreter and surface an actionable message if not found.
    ///
    /// Called once during `handle_initialize`. Reads `perl-lsp.perl.path` from the
    /// workspace config (if set), then falls back to full OS-aware detection. Emits:
    ///
    /// - `window/logMessage` (Info) if Perl was found via an OS fallback path so the
    ///   user knows which interpreter will be used.
    /// - `window/showMessage` (Error) **once per session** if no Perl interpreter is found
    ///   anywhere, with actionable remediation text.
    /// - `window/showMessage` (Error) **once per session** if `perl-lsp.perl.path` is set
    ///   but the configured path does not exist.
    ///
    /// Does not alter any server state. Tracing fallback is preserved alongside user messages.
    pub(crate) fn check_perl_interpreter(&self) {
        let configured_path = self.workspace_config.lock().perl_path.clone();
        let result = find_perl_interpreter(configured_path.as_deref());

        match result {
            PerlInterpreterResult::ConfiguredPath(ref path) => {
                tracing::debug!(path = %path.display(), "Perl interpreter: using configured path");
            }
            PerlInterpreterResult::FoundOnPath(ref path) => {
                tracing::debug!(path = %path.display(), "Perl interpreter: found on PATH");
            }
            PerlInterpreterResult::FoundViaFallback { ref path, ref label } => {
                let msg = format!(
                    "perl-lsp: Perl not found on PATH; using {label} at {}. \
                     Add Perl to PATH or set `perl-lsp.perl.path` to suppress this message.",
                    path.display()
                );
                tracing::info!(path = %path.display(), label = %label, "Perl interpreter found via fallback");
                if let Err(e) = self.log_message(MessageType::Info, &msg) {
                    tracing::warn!(error = %e, "Failed to send logMessage for perl fallback");
                }
            }
            PerlInterpreterResult::NotFound { ref searched } => {
                tracing::warn!(searched = ?searched, "Perl interpreter not found");
                PERL_NOT_FOUND_WARNED.call_once(|| {
                    let msg = perl_not_found_message(configured_path.as_deref(), searched);
                    if let Err(e) = self.show_message(MessageType::Error, &msg) {
                        tracing::warn!(error = %e, "Failed to send showMessage for perl not found");
                    }
                });
            }
        }
    }

    /// Load `.perl-lsp.toml` from each workspace folder and compute per-folder effective config.
    ///
    /// Called once during `handle_initialize`, after workspace folders are populated and
    /// before the server returns capabilities. Subsequent `didChangeConfiguration`
    /// notifications will override these values (LSP wins over TOML).
    ///
    /// On TOML parse error, emits a `window/showMessage` Warning so the user can fix the file.
    /// In single-file mode (no workspace folders), returns early without searching.
    ///
    /// Multi-root workspaces: each folder loads its own `.perl-lsp.toml` independently.
    pub(crate) fn load_and_apply_project_config(&self) {
        let mut folders = self.workspace_folders.lock();

        if folders.is_empty() {
            return; // Single-file mode; no workspace root to search
        }

        for folder in folders.iter_mut() {
            // Try to load .perl-lsp.toml from this folder
            if let Some(folder_path) = &folder.path {
                match perl_lsp_rs_core::config::load_project_config(folder_path) {
                    Ok(None) => {
                        // No .perl-lsp.toml found — normal, no action needed
                    }
                    Ok(Some(project_config)) => {
                        tracing::debug!(path = %folder_path.display(), "Loaded .perl-lsp.toml for folder");

                        // Store project config in the folder state
                        folder.project_config = Some(project_config.clone());

                        // Apply global settings to server config (editor preferences, etc.)
                        {
                            let mut config = self.config.lock();
                            project_config.apply_to_server_config(&mut config);
                        }

                        // Compute effective workspace config for this folder
                        let mut effective_config = WorkspaceConfig::default();
                        project_config.apply_to_workspace_config(&mut effective_config);
                        folder.effective_workspace_config = effective_config;
                    }
                    Err(msg) => {
                        let user_msg = format!(
                            "perl-lsp: {msg} \
                             Fix the error in .perl-lsp.toml and reload the window \
                             (Ctrl+Shift+P \u{2192} Developer: Reload Window) to apply your settings.",
                        );
                        tracing::warn!(message = %user_msg, "Project config warning");
                        // Emit user-visible warning so devs can fix a broken .perl-lsp.toml
                        if let Err(e) = self.notify(
                            "window/showMessage",
                            serde_json::json!({
                                "type": 2, // Warning
                                "message": user_msg
                            }),
                        ) {
                            tracing::warn!(error = %e, "Failed to send showMessage warning");
                        }
                    }
                }
            }
        }

        // Pull client-scoped workspace settings (if supported) and merge them
        // as the highest-precedence layer over TOML-derived folder config.
        drop(folders);
        self.request_workspace_configuration_for_folders();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn check_perl_interpreter_does_not_panic() {
        let server = LspServer::new();
        // Should complete without panicking regardless of Perl install state
        server.check_perl_interpreter();
    }

    #[test]
    fn check_perl_interpreter_broken_config_path_does_not_panic() {
        let server = LspServer::new();
        // Set a broken perl path in the workspace config
        server.workspace_config.lock().perl_path =
            Some("/nonexistent/path/to/perl/that/does/not/exist".to_string());
        // Should not panic — message will fail to send silently (stdout in test mode)
        server.check_perl_interpreter();
    }

    #[test]
    fn check_perl_interpreter_valid_config_path_does_not_panic() {
        use std::fs;
        let tempdir = tempfile::tempdir().expect("tempdir");
        #[cfg(windows)]
        let fake_perl = tempdir.path().join("perl.exe");
        #[cfg(not(windows))]
        let fake_perl = tempdir.path().join("perl");
        fs::write(&fake_perl, "").expect("write fake perl");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_perl).expect("metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_perl, perms).expect("chmod");
        }
        let server = LspServer::new();
        server.workspace_config.lock().perl_path = Some(fake_perl.to_string_lossy().to_string());
        // Should not panic
        server.check_perl_interpreter();
    }

    #[test]
    fn load_and_apply_project_config_handles_empty_workspace_folders() {
        let server = LspServer::new();
        // Should not panic with empty workspace folders
        server.load_and_apply_project_config();
    }

    #[test]
    fn set_root_uri_ignores_non_file_scheme() {
        let server = LspServer::new();
        server.set_root_uri("untitled:Untitled-1");
        assert!(server.root_path.lock().is_none());
    }

    #[test]
    fn load_and_apply_project_config_loads_per_folder_config() {
        let server = LspServer::new();
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let folder1 = temp.path().join("folder1");
        let folder2 = temp.path().join("folder2");
        std::fs::create_dir_all(&folder1).expect("failed to create folder1");
        std::fs::create_dir_all(&folder2).expect("failed to create folder2");

        // Create .perl-lsp.toml in folder1
        let config1 = folder1.join(".perl-lsp.toml");
        std::fs::write(
            &config1,
            r#"
[perl]
include_paths = ["custom_lib"]
"#,
        )
        .expect("failed to write config1");

        // Create .perl-lsp.toml in folder2
        let config2 = folder2.join(".perl-lsp.toml");
        std::fs::write(
            &config2,
            r#"
[perl]
include_paths = ["other_lib"]
"#,
        )
        .expect("failed to write config2");

        // Add workspace folders
        let uri1 =
            url::Url::from_directory_path(&folder1).expect("failed to create uri1").to_string();
        let uri2 =
            url::Url::from_directory_path(&folder2).expect("failed to create uri2").to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri1.clone())
                .with_path(folder1.clone()),
        );
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri2.clone())
                .with_path(folder2.clone()),
        );

        // Load configs
        server.load_and_apply_project_config();

        // Verify each folder has its own config
        let folders = server.workspace_folders.lock();
        assert_eq!(folders.len(), 2);

        let folder1_state = folders.iter().find(|f| f.uri == uri1).unwrap();
        assert!(folder1_state.project_config.is_some());
        assert!(
            folder1_state
                .effective_workspace_config
                .include_paths
                .contains(&"custom_lib".to_string())
        );

        let folder2_state = folders.iter().find(|f| f.uri == uri2).unwrap();
        assert!(folder2_state.project_config.is_some());
        assert!(
            folder2_state
                .effective_workspace_config
                .include_paths
                .contains(&"other_lib".to_string())
        );
    }

    #[test]
    fn load_and_apply_project_config_handles_missing_config() {
        let server = LspServer::new();
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let folder = temp.path().join("folder");
        std::fs::create_dir_all(&folder).expect("failed to create folder");

        // Add workspace folder without config
        let uri = url::Url::from_directory_path(&folder).expect("failed to create uri").to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri.clone())
                .with_path(folder.clone()),
        );

        // Load configs
        server.load_and_apply_project_config();

        // Verify folder has no project config but has default effective config
        let folders = server.workspace_folders.lock();
        assert_eq!(folders.len(), 1);

        let folder_state = folders.iter().find(|f| f.uri == uri).unwrap();
        assert!(folder_state.project_config.is_none());
        // Should have default include paths
        assert!(!folder_state.effective_workspace_config.include_paths.is_empty());
    }

    #[test]
    fn handle_client_response_applies_per_folder_workspace_config() {
        let server = LspServer::new();
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let folder1 = temp.path().join("folder1");
        let folder2 = temp.path().join("folder2");
        std::fs::create_dir_all(&folder1).expect("failed to create folder1");
        std::fs::create_dir_all(&folder2).expect("failed to create folder2");

        let uri1 =
            url::Url::from_directory_path(&folder1).expect("failed to create uri1").to_string();
        let uri2 =
            url::Url::from_directory_path(&folder2).expect("failed to create uri2").to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri1.clone())
                .with_path(folder1.clone()),
        );
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri2.clone())
                .with_path(folder2.clone()),
        );

        server.pending_workspace_configuration_requests.lock().insert(
            11,
            crate::runtime::PendingWorkspaceConfigurationRequest {
                folder_uris: vec![uri1.clone(), uri2.clone()],
                includes_global_item: true,
                created_at: std::time::Instant::now(),
            },
        );

        server.handle_client_response(Some(serde_json::json!({
            "id": 11,
            "result": [
                { "workspace": { "useSystemInc": true } },
                { "workspace": { "includePaths": ["api_lib"] } },
                { "workspace": { "includePaths": ["ui_lib"] } }
            ]
        })));

        let folders = server.workspace_folders.lock();
        let folder1_state = folders.iter().find(|f| f.uri == uri1).expect("missing folder1");
        let folder2_state = folders.iter().find(|f| f.uri == uri2).expect("missing folder2");

        assert!(
            folder1_state.effective_workspace_config.include_paths.contains(&"api_lib".to_string())
        );
        assert!(
            folder2_state.effective_workspace_config.include_paths.contains(&"ui_lib".to_string())
        );
        assert!(folder1_state.effective_workspace_config.use_system_inc);
        assert!(folder2_state.effective_workspace_config.use_system_inc);
    }

    #[test]
    fn handle_client_response_ignores_non_array_result() {
        let server = LspServer::new();
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let folder = temp.path().join("folder");
        std::fs::create_dir_all(&folder).expect("failed to create folder");
        let uri = url::Url::from_directory_path(&folder).expect("failed to create uri").to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri.clone())
                .with_path(folder.clone()),
        );
        server.pending_workspace_configuration_requests.lock().insert(
            99,
            crate::runtime::PendingWorkspaceConfigurationRequest {
                folder_uris: vec![uri.clone()],
                includes_global_item: true,
                created_at: std::time::Instant::now(),
            },
        );

        server.handle_client_response(Some(serde_json::json!({
            "id": 99,
            "result": { "workspace": { "includePaths": ["oops"] } }
        })));

        let folders = server.workspace_folders.lock();
        let folder_state = folders.iter().find(|f| f.uri == uri).expect("missing folder");
        assert!(
            !folder_state.effective_workspace_config.include_paths.contains(&"oops".to_string())
        );
    }

    #[test]
    fn did_change_configuration_updates_folder_effective_configs() {
        let server = LspServer::new();
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let folder1 = temp.path().join("folder1");
        let folder2 = temp.path().join("folder2");
        std::fs::create_dir_all(&folder1).expect("failed to create folder1");
        std::fs::create_dir_all(&folder2).expect("failed to create folder2");

        let uri1 =
            url::Url::from_directory_path(&folder1).expect("failed to create uri1").to_string();
        let uri2 =
            url::Url::from_directory_path(&folder2).expect("failed to create uri2").to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri1)
                .with_path(folder1.clone()),
        );
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri2)
                .with_path(folder2.clone()),
        );

        server.handle_did_change_configuration(Some(serde_json::json!({
            "settings": {
                "perl": {
                    "workspace": {
                        "includePaths": ["client_lib"],
                        "useSystemInc": true
                    }
                }
            }
        })));

        let folders = server.workspace_folders.lock();
        assert_eq!(folders.len(), 2);
        for folder in folders.iter() {
            assert!(
                folder.effective_workspace_config.include_paths.contains(&"client_lib".to_string()),
                "folder {} missing client_lib in effective include_paths",
                folder.uri
            );
            assert!(
                folder.effective_workspace_config.use_system_inc,
                "folder {} should have use_system_inc=true from didChangeConfiguration",
                folder.uri
            );
        }
    }

    #[test]
    fn did_change_configuration_accepts_unwrapped_perl_settings() {
        let server = LspServer::new();
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let folder = temp.path().join("folder");
        std::fs::create_dir_all(&folder).expect("failed to create folder");

        let uri = url::Url::from_directory_path(&folder).expect("failed to create uri").to_string();

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(uri.clone())
                .with_path(folder),
        );

        server.handle_did_change_configuration(Some(serde_json::json!({
            "settings": {
                "workspace": {
                    "includePaths": ["sublime_lib"],
                    "useSystemInc": true
                }
            }
        })));

        let folders = server.workspace_folders.lock();
        let folder_state = folders.iter().find(|f| f.uri == uri).expect("missing folder");
        assert!(
            folder_state.effective_workspace_config.include_paths.contains(&"sublime_lib".to_string())
        );
        assert!(folder_state.effective_workspace_config.use_system_inc);
    }

    #[test]
    fn perl_not_found_message_without_config_is_actionable() -> Result<(), Box<dyn Error>> {
        let searched = vec!["PATH".to_string(), "/usr/bin/perl".to_string()];
        let msg = perl_not_found_message(None, &searched);
        assert!(msg.contains("Perl interpreter not found on PATH"));
        assert!(msg.contains("strawberryperl.com"));
        assert!(msg.contains("brew install perl"));
        assert!(msg.contains("distro package manager on Linux"));
        Ok(())
    }

    #[test]
    fn perl_not_found_message_with_config_mentions_fix_and_install() -> Result<(), Box<dyn Error>> {
        let searched = vec!["configured path: /missing/perl".to_string()];
        let msg = perl_not_found_message(Some("/missing/perl"), &searched);
        assert!(msg.contains("configured Perl interpreter was not found"));
        assert!(msg.contains("perl-lsp.perl.path"));
        assert!(msg.contains("strawberryperl.com"));
        assert!(msg.contains("distro package manager on Linux"));
        Ok(())
    }
}
