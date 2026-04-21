//! Workspace folder state representation.
//!
//! This module provides explicit workspace folder state to replace the current
//! "folder list + singleton config/root" assumptions as part of multi-root
//! workspace support.

#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::PathBuf;

use perl_lsp_rs_core::config::{ProjectConfig, WorkspaceConfig};

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
}
