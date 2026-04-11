//! Workspace management
//!
//! Handles workspace folders and root URI/path management.

use super::super::*;
use perl_lsp_config::WorkspaceConfig;

impl LspServer {
    /// Set the root path from the root URI during initialization
    pub(crate) fn set_root_uri(&self, root_uri: &str) {
        let root_path = perl_uri::uri_to_fs_path(root_uri);
        *self.root_path.lock() = root_path;
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
                match perl_lsp_config::load_project_config(folder_path) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_and_apply_project_config_handles_empty_workspace_folders() {
        let server = LspServer::new();
        // Should not panic with empty workspace folders
        server.load_and_apply_project_config();
    }

    #[test]
    fn set_root_uri_handles_non_file_scheme() {
        let server = LspServer::new();
        server.set_root_uri("vscode-remote://ssh-remote+dev/home/project");
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
}
