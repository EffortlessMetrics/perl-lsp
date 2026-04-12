//! Workspace folder state representation.
//!
//! This module provides explicit workspace folder state to replace the current
//! "folder list + singleton config/root" assumptions as part of multi-root
//! workspace support.

#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::PathBuf;

use perl_lsp_config::{ProjectConfig, WorkspaceConfig};
use serde_json::Value;

/// State for a single workspace folder.
///
/// This struct represents a workspace folder with its metadata and configuration.
/// It will eventually support per-folder effective settings, but for now it provides
/// the foundation for multi-root workspace support.
#[derive(Debug, Clone)]
pub struct WorkspaceFolderState {
    /// The URI of the workspace folder (e.g., "file:///path/to/folder")
    pub uri: String,
    /// The filesystem path of the workspace folder (if resolvable)
    pub path: Option<PathBuf>,
    /// The name of the workspace folder (optional, from LSP client)
    pub name: Option<String>,
    /// Project configuration loaded from `.perl-lsp.toml` in this folder
    pub project_config: Option<ProjectConfig>,
    /// Client-wide workspace settings from unscoped `workspace/configuration`.
    pub client_global_settings: Option<Value>,
    /// Folder-scoped workspace settings from `workspace/configuration(scopeUri=folder)`.
    pub client_folder_settings: Option<Value>,
    /// Effective workspace configuration for this folder
    ///
    /// This will eventually be computed by merging:
    /// 1. Default workspace config
    /// 2. Project config from `.perl-lsp.toml`
    /// 3. LSP client settings
    pub effective_workspace_config: WorkspaceConfig,
}

impl WorkspaceFolderState {
    /// Create a new workspace folder state from a URI.
    #[must_use]
    pub fn new(uri: String) -> Self {
        Self {
            uri,
            path: None,
            name: None,
            project_config: None,
            client_global_settings: None,
            client_folder_settings: None,
            effective_workspace_config: WorkspaceConfig::default(),
        }
    }

    /// Set the filesystem path for this workspace folder.
    #[must_use]
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the name for this workspace folder.
    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the project configuration for this workspace folder.
    #[must_use]
    pub fn with_project_config(mut self, config: ProjectConfig) -> Self {
        self.project_config = Some(config);
        self
    }

    /// Set the effective workspace configuration for this workspace folder.
    #[must_use]
    pub fn with_effective_workspace_config(mut self, config: WorkspaceConfig) -> Self {
        self.effective_workspace_config = config;
        self
    }

    /// Recompute effective workspace config using precedence:
    /// defaults < `.perl-lsp.toml` < client-global < client-folder.
    pub fn recompute_effective_workspace_config(&mut self) {
        let mut effective = WorkspaceConfig::default();
        if let Some(project) = &self.project_config {
            project.apply_to_workspace_config(&mut effective);
        }
        if let Some(global_settings) = &self.client_global_settings {
            effective.update_from_value(global_settings);
        }
        if let Some(folder_settings) = &self.client_folder_settings {
            effective.update_from_value(folder_settings);
        }
        self.effective_workspace_config = effective;
    }

    /// Get the URI as a string reference.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Get the name, or derive it from the URI if not set.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_new_folder_state() {
        let folder = WorkspaceFolderState::new("file:///test/path".to_string());
        assert_eq!(folder.uri, "file:///test/path");
        assert!(folder.path.is_none());
        assert!(folder.name.is_none());
        assert!(folder.project_config.is_none());
        assert!(folder.client_global_settings.is_none());
        assert!(folder.client_folder_settings.is_none());
    }

    #[test]
    fn builds_with_path() {
        let folder = WorkspaceFolderState::new("file:///test/path".to_string())
            .with_path(PathBuf::from("/test/path"));
        assert_eq!(folder.path, Some(PathBuf::from("/test/path")));
    }

    #[test]
    fn builds_with_name() {
        let folder = WorkspaceFolderState::new("file:///test/path".to_string())
            .with_name("My Project".to_string());
        assert_eq!(folder.name, Some("My Project".to_string()));
    }

    #[test]
    fn display_name_uses_name_when_set() {
        let folder = WorkspaceFolderState::new("file:///test/path".to_string())
            .with_name("My Project".to_string());
        assert_eq!(folder.display_name(), "My Project");
    }

    #[test]
    fn display_name_falls_back_to_uri() {
        let folder = WorkspaceFolderState::new("file:///test/path".to_string());
        assert_eq!(folder.display_name(), "file:///test/path");
    }

    #[test]
    fn builds_with_project_config() {
        let project_config = ProjectConfig::default();
        let folder = WorkspaceFolderState::new("file:///test/path".to_string())
            .with_project_config(project_config.clone());
        assert!(folder.project_config.is_some());
    }

    #[test]
    fn builds_with_effective_workspace_config() {
        let workspace_config = WorkspaceConfig::default();
        let folder = WorkspaceFolderState::new("file:///test/path".to_string())
            .with_effective_workspace_config(workspace_config.clone());
        assert_eq!(folder.effective_workspace_config.include_paths, workspace_config.include_paths);
    }

    #[test]
    fn effective_workspace_config_has_defaults() {
        let folder = WorkspaceFolderState::new("file:///test/path".to_string());
        let config = &folder.effective_workspace_config;
        assert!(!config.include_paths.is_empty());
        assert_eq!(config.resolution_timeout_ms, 50);
        assert!(!config.use_system_inc);
    }

    #[test]
    fn recompute_effective_workspace_config_merges_project_then_client_global() {
        let mut folder = WorkspaceFolderState::new("file:///test/path".to_string());
        let mut project = ProjectConfig::default();
        project.perl.include_paths = vec!["project_lib".to_string()];
        folder.project_config = Some(project);
        folder.client_global_settings =
            Some(serde_json::json!({ "workspace": { "useSystemInc": true } }));

        folder.recompute_effective_workspace_config();

        assert_eq!(folder.effective_workspace_config.include_paths, vec!["project_lib"]);
        assert!(folder.effective_workspace_config.use_system_inc);
    }

    #[test]
    fn recompute_effective_workspace_config_allows_folder_override() {
        let mut folder = WorkspaceFolderState::new("file:///test/path".to_string());
        folder.client_global_settings =
            Some(serde_json::json!({ "workspace": { "includePaths": ["global_lib"] } }));
        folder.client_folder_settings =
            Some(serde_json::json!({ "workspace": { "includePaths": ["folder_lib"] } }));

        folder.recompute_effective_workspace_config();

        assert_eq!(folder.effective_workspace_config.include_paths, vec!["folder_lib"]);
    }
}
