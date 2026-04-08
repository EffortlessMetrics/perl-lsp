//! Workspace management
//!
//! Handles workspace folders and root URI/path management.

use super::super::*;
use url::Url;

impl LspServer {
    /// Set the root path from the root URI during initialization
    pub(crate) fn set_root_uri(&self, root_uri: &str) {
        let root_path = Url::parse(root_uri).ok().and_then(|u| u.to_file_path().ok());
        *self.root_path.lock() = root_path;
    }

    /// Load `.perl-lsp.toml` from the first workspace folder and apply it as the base layer.
    ///
    /// Called once during `handle_initialize`, after workspace folders are populated and
    /// before the server returns capabilities. Subsequent `didChangeConfiguration`
    /// notifications will override these values (LSP wins over TOML).
    ///
    /// On TOML parse error, emits a `window/showMessage` Warning so the user can fix the file.
    /// In single-file mode (no workspace folders), returns early without searching.
    ///
    /// Multi-root workspaces: the first folder in `workspace_folders` wins.
    pub(crate) fn load_and_apply_project_config(&self) {
        // Determine workspace root: first workspace folder wins in multi-root workspaces.
        let root_opt = {
            let folders = self.workspace_folders.lock();
            folders.first().and_then(|uri| Url::parse(uri).ok().and_then(|u| u.to_file_path().ok()))
        };

        let Some(root) = root_opt else {
            return; // Single-file mode; no workspace root to search
        };

        match perl_lsp_config::load_project_config(&root) {
            Ok(None) => {
                // No .perl-lsp.toml found — normal, no action needed
            }
            Ok(Some(project_config)) => {
                tracing::debug!(path = %root.display(), "Loaded .perl-lsp.toml");
                {
                    let mut config = self.config.lock();
                    project_config.apply_to_server_config(&mut config);
                }
                {
                    let mut workspace_config = self.workspace_config.lock();
                    project_config.apply_to_workspace_config(&mut workspace_config);
                }
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
