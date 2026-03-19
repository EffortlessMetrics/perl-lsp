//! `.perl-lsp.toml` project configuration support.
//!
//! Provides editor-agnostic, per-project settings that live in the repository.
//! Values from this file act as baseline defaults; LSP client settings override them.

use serde::Deserialize;
use std::path::Path;

/// The filename looked up in the workspace root.
pub const PROJECT_CONFIG_FILENAME: &str = ".perl-lsp.toml";

/// Top-level project configuration deserialized from `.perl-lsp.toml`.
///
/// All sections and fields are optional; missing values keep the server defaults.
///
/// # Example `.perl-lsp.toml`
///
/// ```toml
/// [perl]
/// version = "5.38"
/// include_paths = ["lib", "local/lib/perl5"]
///
/// [diagnostics]
/// perlcritic = false
/// perlcritic_severity = 3
///
/// [formatting]
/// perltidy = true
/// perltidy_profile = ".perltidyrc"
///
/// [features]
/// inlay_hints = true
/// code_lens = true
/// semantic_tokens = true
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// Perl language settings.
    pub perl: PerlSection,
    /// Diagnostics settings.
    pub diagnostics: DiagnosticsSection,
    /// Formatting settings.
    pub formatting: FormattingSection,
    /// Feature toggles.
    pub features: FeaturesSection,
}

/// `[perl]` section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PerlSection {
    /// Declared Perl version (informational, e.g. `"5.38"`).
    pub version: Option<String>,
    /// Additional include paths for module resolution.
    pub include_paths: Option<Vec<String>>,
}

/// `[diagnostics]` section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DiagnosticsSection {
    /// Enable perlcritic integration.
    pub perlcritic: Option<bool>,
    /// Minimum perlcritic severity level (1-5).
    pub perlcritic_severity: Option<u8>,
}

/// `[formatting]` section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FormattingSection {
    /// Enable perltidy formatting.
    pub perltidy: Option<bool>,
    /// Path to perltidy profile (relative to workspace root).
    pub perltidy_profile: Option<String>,
}

/// `[features]` section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FeaturesSection {
    /// Enable inlay hints.
    pub inlay_hints: Option<bool>,
    /// Enable code lens.
    pub code_lens: Option<bool>,
    /// Enable semantic tokens.
    pub semantic_tokens: Option<bool>,
}

/// Errors that can occur when loading a project config file.
#[derive(Debug)]
pub enum ProjectConfigError {
    /// File could not be read.
    Io(std::io::Error),
    /// File contents are not valid TOML.
    Parse(toml::de::Error),
}

impl std::fmt::Display for ProjectConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "failed to read .perl-lsp.toml: {e}"),
            Self::Parse(e) => write!(f, "failed to parse .perl-lsp.toml: {e}"),
        }
    }
}

impl std::error::Error for ProjectConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
        }
    }
}

impl ProjectConfig {
    /// Try to load a `.perl-lsp.toml` from the given workspace root.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    /// Returns `Ok(Some(config))` on success.
    /// Returns `Err(...)` if the file exists but cannot be read or parsed.
    pub fn load_from_workspace(workspace_root: &Path) -> Result<Option<Self>, ProjectConfigError> {
        let path = workspace_root.join(PROJECT_CONFIG_FILENAME);
        if !path.is_file() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path).map_err(ProjectConfigError::Io)?;
        let config: Self = toml::from_str(&contents).map_err(ProjectConfigError::Parse)?;
        Ok(Some(config))
    }

    /// Parse a `.perl-lsp.toml` from a string.
    pub fn from_str(s: &str) -> Result<Self, ProjectConfigError> {
        toml::from_str(s).map_err(ProjectConfigError::Parse)
    }

    /// Apply this project config as baseline to a [`ServerConfig`](super::ServerConfig).
    ///
    /// Only fields that are `Some` in the project config overwrite server defaults.
    pub fn apply_to_server_config(&self, config: &mut super::ServerConfig) {
        if let Some(enabled) = self.features.inlay_hints {
            config.inlay_hints_enabled = enabled;
        }
    }

    /// Apply this project config as baseline to a [`WorkspaceConfig`](super::WorkspaceConfig).
    ///
    /// Only fields that are `Some` in the project config overwrite workspace defaults.
    pub fn apply_to_workspace_config(&self, config: &mut super::WorkspaceConfig) {
        if let Some(ref paths) = self.perl.include_paths {
            config.include_paths = paths.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ServerConfig, WorkspaceConfig};

    #[test]
    fn parse_full_config() {
        let toml = r#"
[perl]
version = "5.38"
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
perlcritic = false
perlcritic_severity = 3

[formatting]
perltidy = true
perltidy_profile = ".perltidyrc"

[features]
inlay_hints = true
code_lens = true
semantic_tokens = true
"#;
        let config = ProjectConfig::from_str(toml);
        assert!(config.is_ok(), "full config should parse: {:?}", config.err());
        let config = config.unwrap_or_default();
        assert_eq!(config.perl.version.as_deref(), Some("5.38"));
        assert_eq!(
            config.perl.include_paths.as_deref(),
            Some(["lib".to_string(), "local/lib/perl5".to_string()].as_slice())
        );
        assert_eq!(config.diagnostics.perlcritic, Some(false));
        assert_eq!(config.diagnostics.perlcritic_severity, Some(3));
        assert_eq!(config.formatting.perltidy, Some(true));
        assert_eq!(config.formatting.perltidy_profile.as_deref(), Some(".perltidyrc"));
        assert_eq!(config.features.inlay_hints, Some(true));
        assert_eq!(config.features.code_lens, Some(true));
        assert_eq!(config.features.semantic_tokens, Some(true));
    }

    #[test]
    fn parse_empty_config() {
        let config = ProjectConfig::from_str("");
        assert!(config.is_ok());
        let config = config.unwrap_or_default();
        assert!(config.perl.version.is_none());
        assert!(config.perl.include_paths.is_none());
        assert!(config.diagnostics.perlcritic.is_none());
        assert!(config.features.inlay_hints.is_none());
    }

    #[test]
    fn parse_partial_config() {
        let toml = r#"
[features]
inlay_hints = false
"#;
        let config = ProjectConfig::from_str(toml);
        assert!(config.is_ok());
        let config = config.unwrap_or_default();
        assert_eq!(config.features.inlay_hints, Some(false));
        assert!(config.features.code_lens.is_none());
        assert!(config.perl.version.is_none());
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let result = ProjectConfig::from_str("not valid {{{{ toml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("parse"), "error message should mention parsing: {msg}");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let toml = r#"
[perl]
version = "5.40"
unknown_field = "ignored"

[unknown_section]
key = "value"
"#;
        let result = ProjectConfig::from_str(toml);
        assert!(result.is_ok(), "unknown fields should be ignored: {:?}", result.err());
    }

    #[test]
    fn apply_to_server_config_sets_inlay_hints() {
        let mut server = ServerConfig::default();
        assert!(server.inlay_hints_enabled);

        let project =
            ProjectConfig::from_str("[features]\ninlay_hints = false").unwrap_or_default();
        project.apply_to_server_config(&mut server);
        assert!(!server.inlay_hints_enabled);
    }

    #[test]
    fn apply_to_server_config_no_op_when_none() {
        let mut server = ServerConfig::default();
        let project = ProjectConfig::from_str("").unwrap_or_default();
        project.apply_to_server_config(&mut server);
        assert!(server.inlay_hints_enabled);
    }

    #[test]
    fn apply_to_workspace_config_sets_include_paths() {
        let mut workspace = WorkspaceConfig::default();
        assert_eq!(workspace.include_paths, vec!["lib", ".", "local/lib/perl5"]);

        let project = ProjectConfig::from_str("[perl]\ninclude_paths = [\"lib\", \"vendor/lib\"]")
            .unwrap_or_default();
        project.apply_to_workspace_config(&mut workspace);
        assert_eq!(workspace.include_paths, vec!["lib", "vendor/lib"]);
    }

    #[test]
    fn apply_to_workspace_config_no_op_when_none() {
        let mut workspace = WorkspaceConfig::default();
        let orig_paths = workspace.include_paths.clone();
        let project = ProjectConfig::from_str("").unwrap_or_default();
        project.apply_to_workspace_config(&mut workspace);
        assert_eq!(workspace.include_paths, orig_paths);
    }

    #[test]
    fn load_from_nonexistent_workspace_returns_none() {
        let result = ProjectConfig::load_from_workspace(Path::new("/nonexistent/path"));
        assert!(result.is_ok());
        assert!(result.unwrap_or_default().is_none());
    }

    #[test]
    fn load_from_workspace_with_file() {
        let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("failed to create temp dir"));
        let config_path = dir.path().join(PROJECT_CONFIG_FILENAME);
        std::fs::write(
            &config_path,
            "[perl]\nversion = \"5.36\"\n[features]\ninlay_hints = false\n",
        )
        .unwrap_or_else(|_| panic!("failed to write config"));

        let result = ProjectConfig::load_from_workspace(dir.path());
        assert!(result.is_ok());
        let config = result.unwrap_or_default();
        assert!(config.is_some());
        let config = config.unwrap_or_default();
        assert_eq!(config.perl.version.as_deref(), Some("5.36"));
        assert_eq!(config.features.inlay_hints, Some(false));
    }

    #[test]
    fn load_from_workspace_with_malformed_file() {
        let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("failed to create temp dir"));
        let config_path = dir.path().join(PROJECT_CONFIG_FILENAME);
        std::fs::write(&config_path, "{{invalid toml}}")
            .unwrap_or_else(|_| panic!("failed to write config"));

        let result = ProjectConfig::load_from_workspace(dir.path());
        assert!(result.is_err());
    }
}
