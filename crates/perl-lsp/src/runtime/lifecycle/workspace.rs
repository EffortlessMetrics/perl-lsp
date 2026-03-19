//! Workspace management
//!
//! Handles workspace folders and root URI/path management,
//! including `.perl-lsp.toml` project configuration loading.

use super::super::*;
use url::Url;

impl LspServer {
    /// Set the root path from the root URI during initialization
    pub(crate) fn set_root_uri(&self, root_uri: &str) {
        let root_path = Url::parse(root_uri).ok().and_then(|u| u.to_file_path().ok());
        *self.root_path.lock() = root_path;
    }

    /// Load `.perl-lsp.toml` from the first workspace folder (if any).
    ///
    /// This is called during initialization, after workspace folders have been
    /// resolved. The project config provides baseline settings that LSP client
    /// settings (`didChangeConfiguration`) can subsequently override.
    ///
    /// Gracefully handles missing or malformed files — a warning is logged but
    /// the server continues with its defaults.
    pub(crate) fn load_project_config(&self) {
        let workspace_root = {
            let folders = self.workspace_folders.lock();
            folders.first().and_then(|uri| Url::parse(uri).ok().and_then(|u| u.to_file_path().ok()))
        };

        let Some(root) = workspace_root else {
            return;
        };

        match ProjectConfig::load_from_workspace(&root) {
            Ok(Some(project)) => {
                eprintln!("Loaded .perl-lsp.toml from {}", root.display());
                project.apply_to_server_config(&mut self.config.lock());
                project.apply_to_workspace_config(&mut self.workspace_config.lock());
            }
            Ok(None) => {
                // No .perl-lsp.toml found — this is normal
            }
            Err(e) => {
                eprintln!("Warning: {e}");
            }
        }
    }
}
