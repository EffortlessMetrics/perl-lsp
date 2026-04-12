#![warn(missing_docs)]
//! Configuration models for perl-lsp server runtime state.
//!
//! This microcrate isolates configuration parsing and defaults from the main
//! server crate so they can evolve independently and be reused by tooling.

use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

/// Server configuration
///
/// Runtime configuration for the LSP server features including inlay hints
/// and test runner integration. Updated dynamically via `didChangeConfiguration`.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Whether inlay hints are globally enabled.
    pub inlay_hints_enabled: bool,
    /// Show parameter name hints at call sites.
    pub inlay_hints_parameter_hints: bool,
    /// Show inferred type hints for variables.
    pub inlay_hints_type_hints: bool,
    /// Show hints for method chains.
    pub inlay_hints_chained_hints: bool,
    /// Maximum character length for hint labels before truncation.
    pub inlay_hints_max_length: usize,

    /// Whether the integrated test runner is enabled.
    pub test_runner_enabled: bool,
    /// Command to execute tests (e.g., "perl", "prove").
    pub test_runner_command: String,
    /// Additional arguments passed to the test command.
    pub test_runner_args: Vec<String>,
    /// Test execution timeout in milliseconds.
    pub test_runner_timeout: u64,

    /// Whether telemetry events are enabled.
    pub telemetry_enabled: bool,

    /// Whether external perlcritic diagnostics are enabled (opt-in).
    ///
    /// When enabled, the server will run `perlcritic` on open documents and
    /// merge violations into the diagnostic stream. Requires `perlcritic` to
    /// be installed on the system; silently skipped if not available.
    pub perlcritic_enabled: bool,

    /// Minimum severity level to report (1-5, where 1 = most severe).
    ///
    /// Violations below this threshold are suppressed. Default is 3 (Harsh).
    /// Equivalent to `perlcritic --severity`.
    pub perlcritic_severity: u8,

    /// Path to a `.perlcriticrc` profile file.
    ///
    /// When `Some`, passes `--profile=<path>` to perlcritic. When `None`,
    /// the auto-discovery logic looks for `.perlcriticrc` in the workspace root.
    pub perlcritic_profile: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            inlay_hints_enabled: true,
            inlay_hints_parameter_hints: true,
            inlay_hints_type_hints: true,
            inlay_hints_chained_hints: false,
            inlay_hints_max_length: 30,
            test_runner_enabled: true,
            test_runner_command: "perl".to_string(),
            test_runner_args: vec![],
            test_runner_timeout: 60000,
            telemetry_enabled: false,
            perlcritic_enabled: false,
            perlcritic_severity: 3,
            perlcritic_profile: None,
        }
    }
}

impl ServerConfig {
    /// Update configuration from LSP settings
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(inlay) = settings.get("inlayHints") {
            if let Some(enabled) = inlay.get("enabled").and_then(|v| v.as_bool()) {
                self.inlay_hints_enabled = enabled;
            }
            if let Some(param) = inlay.get("parameterHints").and_then(|v| v.as_bool()) {
                self.inlay_hints_parameter_hints = param;
            }
            if let Some(type_hints) = inlay.get("typeHints").and_then(|v| v.as_bool()) {
                self.inlay_hints_type_hints = type_hints;
            }
            if let Some(chained) = inlay.get("chainedHints").and_then(|v| v.as_bool()) {
                self.inlay_hints_chained_hints = chained;
            }
            if let Some(max_len) = inlay.get("maxLength").and_then(|v| v.as_u64()) {
                self.inlay_hints_max_length = max_len as usize;
            }
        }

        if let Some(test) = settings.get("testRunner") {
            if let Some(enabled) = test.get("enabled").and_then(|v| v.as_bool()) {
                self.test_runner_enabled = enabled;
            }
            if let Some(cmd) = test.get("command").and_then(|v| v.as_str()) {
                self.test_runner_command = cmd.to_string();
            }
            if let Some(args) = test.get("args").and_then(|v| v.as_array()) {
                self.test_runner_args =
                    args.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            }
            if let Some(timeout) = test.get("timeout").and_then(|v| v.as_u64()) {
                self.test_runner_timeout = timeout;
            }
        }

        if let Some(telemetry) = settings.get("telemetry")
            && let Some(enabled) = telemetry.get("enabled").and_then(|v| v.as_bool())
        {
            self.telemetry_enabled = enabled;
        }

        if let Some(critic) = settings.get("perlcritic") {
            if let Some(enabled) = critic.get("enabled").and_then(|v| v.as_bool()) {
                self.perlcritic_enabled = enabled;
            }
            if let Some(severity) = critic.get("severity").and_then(|v| v.as_u64()) {
                self.perlcritic_severity = severity.clamp(1, 5) as u8;
            }
            if let Some(profile) = critic.get("profile").and_then(|v| v.as_str()) {
                self.perlcritic_profile = Some(profile.to_string());
            }
        }
    }
}

/// Workspace configuration for module resolution
///
/// Controls how the LSP server resolves module imports and finds
/// Perl module files across the workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Custom include paths for module resolution (relative to workspace root)
    /// Default: `["lib", ".", "local/lib/perl5"]`
    pub include_paths: Vec<String>,

    /// Whether to include system @INC paths in module resolution
    /// Default: false (avoids blocking on network filesystems)
    pub use_system_inc: bool,

    /// Cached system @INC paths (populated lazily when use_system_inc is true)
    system_inc_cache: Option<Vec<PathBuf>>,

    /// Resolution timeout in milliseconds
    /// Default: 50ms
    pub resolution_timeout_ms: u64,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            include_paths: vec!["lib".to_string(), ".".to_string(), "local/lib/perl5".to_string()],
            use_system_inc: false,
            system_inc_cache: None,
            resolution_timeout_ms: 50,
        }
    }
}

impl WorkspaceConfig {
    /// Update workspace configuration from LSP settings.
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(workspace) = settings.get("workspace") {
            if let Some(paths) = workspace.get("includePaths").and_then(|v| v.as_array()) {
                self.include_paths =
                    paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            }
            if let Some(use_inc) = workspace.get("useSystemInc").and_then(|v| v.as_bool()) {
                if use_inc != self.use_system_inc {
                    self.system_inc_cache = None;
                }
                self.use_system_inc = use_inc;
            }
            if let Some(timeout) = workspace.get("resolutionTimeout").and_then(|v| v.as_u64()) {
                self.resolution_timeout_ms = timeout;
            }
        }
    }

    /// Parse PERL5LIB environment variable into a list of paths.
    ///
    /// PERL5LIB uses the same path separator as Perl's @INC:
    /// - Colon (`:`) on Unix
    /// - Semicolon (`;`) on Windows
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_lsp_config::WorkspaceConfig;
    ///
    /// let paths = WorkspaceConfig::parse_perl5lib("/custom/lib:/other/lib");
    /// assert_eq!(paths, vec!["/custom/lib", "/other/lib"]);
    /// ```
    pub fn parse_perl5lib(perl5lib: &str) -> Vec<String> {
        #[cfg(target_os = "windows")]
        const SEP: char = ';';
        #[cfg(not(target_os = "windows"))]
        const SEP: char = ':';

        perl5lib
            .split(SEP)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Get system @INC paths (lazily populated).
    pub fn get_system_inc(&mut self) -> &[PathBuf] {
        if !self.use_system_inc {
            return &[];
        }

        if self.system_inc_cache.is_none() {
            self.system_inc_cache = Some(Self::fetch_perl_inc());
        }

        self.system_inc_cache.as_deref().unwrap_or(&[])
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fetch_perl_inc() -> Vec<PathBuf> {
        let output = Command::new("perl").args(["-e", "print join(\"\\n\", @INC)"]).output();

        match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.is_empty() && *line != ".")
                .map(PathBuf::from)
                .collect(),
            _ => Vec::new(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn fetch_perl_inc() -> Vec<PathBuf> {
        Vec::new()
    }
}

// ── ProjectConfig ─────────────────────────────────────────────────────────────

/// Project configuration loaded from `.perl-lsp.toml` in the workspace root.
///
/// Committed to the repo; provides editor-agnostic, team-wide defaults.
/// LSP `initializationOptions` / `didChangeConfiguration` always win over this file.
///
/// Unknown TOML keys are silently ignored for forward compatibility.
///
/// `[formatting]` is reserved for future perltidy configuration (not yet wired).
#[non_exhaustive]
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// `[perl]` section: module resolution settings.
    pub perl: ProjectPerlConfig,
    /// `[diagnostics]` section: linting settings.
    pub diagnostics: ProjectDiagnosticsConfig,
    /// `[features]` section: LSP feature toggles.
    pub features: ProjectFeaturesConfig,
}

/// `[perl]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectPerlConfig {
    /// Additional include paths for module resolution (relative to workspace root).
    pub include_paths: Vec<String>,
    /// Perl version string (e.g. "5.38") — parsed but not yet wired to diagnostics.
    /// Reserved for future use; ignored in this implementation.
    pub version: Option<String>,
}

/// `[diagnostics]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectDiagnosticsConfig {
    /// Whether perlcritic is enabled. Maps to `ServerConfig.perlcritic_enabled`.
    pub perlcritic: Option<bool>,
    /// Minimum perlcritic severity (1-5). Maps to `ServerConfig.perlcritic_severity`.
    pub perlcritic_severity: Option<u8>,
}

/// `[features]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectFeaturesConfig {
    /// Whether inlay hints are enabled globally. Maps to `ServerConfig.inlay_hints_enabled`.
    pub inlay_hints: Option<bool>,
}

/// Load project config from `<workspace_root>/.perl-lsp.toml`.
///
/// Returns `None` if the file does not exist (normal case — most projects won't have one).
/// Returns `Err` only on TOML parse failure; caller should emit a `window/showMessage` warning.
pub fn load_project_config(
    workspace_root: &std::path::Path,
) -> Result<Option<ProjectConfig>, String> {
    let path = workspace_root.join(".perl-lsp.toml");
    match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Failed to read .perl-lsp.toml: {}", e)),
        Ok(content) => toml::from_str::<ProjectConfig>(&content)
            .map(Some)
            .map_err(|e| format!(".perl-lsp.toml parse error: {}", e)),
    }
}

impl ProjectConfig {
    /// Apply project config to `ServerConfig` as the base layer.
    ///
    /// Only fields explicitly set in the TOML override defaults; unset fields are untouched.
    /// LSP `didChangeConfiguration` is expected to run after this, overriding any values here.
    pub fn apply_to_server_config(&self, config: &mut ServerConfig) {
        if let Some(enabled) = self.diagnostics.perlcritic {
            config.perlcritic_enabled = enabled;
        }
        if let Some(severity) = self.diagnostics.perlcritic_severity {
            config.perlcritic_severity = severity.clamp(1, 5);
        }
        if let Some(hints) = self.features.inlay_hints {
            config.inlay_hints_enabled = hints;
        }
    }

    /// Apply project config to `WorkspaceConfig` as the base layer.
    ///
    /// Only applies `include_paths` when the TOML list is non-empty, so that
    /// an absent key leaves the defaults unchanged (distinct from an explicit `[]`).
    pub fn apply_to_workspace_config(&self, config: &mut WorkspaceConfig) {
        if !self.perl.include_paths.is_empty() {
            config.include_paths = self.perl.include_paths.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ServerConfig, WorkspaceConfig};
    use serde_json::json;

    // ── ServerConfig defaults ─────────────────────────────────

    #[test]
    fn server_config_default_inlay_hints_enabled() {
        let config = ServerConfig::default();
        assert!(config.inlay_hints_enabled, "inlay hints enabled by default");
        assert!(config.inlay_hints_parameter_hints, "parameter hints enabled by default");
        assert!(config.inlay_hints_type_hints, "type hints enabled by default");
        assert!(!config.inlay_hints_chained_hints, "chained hints disabled by default");
        assert_eq!(config.inlay_hints_max_length, 30);
    }

    #[test]
    fn server_config_default_test_runner() {
        let config = ServerConfig::default();
        assert!(config.test_runner_enabled, "test runner enabled by default");
        assert_eq!(config.test_runner_command, "perl");
        assert!(config.test_runner_args.is_empty(), "no default test runner args");
        assert_eq!(config.test_runner_timeout, 60000);
    }

    #[test]
    fn server_config_default_telemetry_disabled() {
        let config = ServerConfig::default();
        assert!(!config.telemetry_enabled, "telemetry disabled by default");
    }

    #[test]
    fn server_config_default_perlcritic_disabled() {
        let config = ServerConfig::default();
        assert!(!config.perlcritic_enabled, "perlcritic disabled by default (opt-in)");
    }

    #[test]
    fn server_config_perlcritic_enabled_via_update() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({
            "perlcritic": { "enabled": true }
        }));
        assert!(config.perlcritic_enabled);
    }

    // ── ServerConfig::update_from_value ──────────────────────

    #[test]
    fn server_config_updates_selected_fields() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({
            "inlayHints": { "enabled": false, "maxLength": 42 },
            "testRunner": { "enabled": false, "command": "prove", "args": ["-l"] },
            "telemetry": { "enabled": true }
        }));

        assert!(!config.inlay_hints_enabled);
        assert_eq!(config.inlay_hints_max_length, 42);
        assert!(!config.test_runner_enabled);
        assert_eq!(config.test_runner_command, "prove");
        assert_eq!(config.test_runner_args, vec!["-l"]);
        assert!(config.telemetry_enabled);
    }

    #[test]
    fn server_config_partial_update_leaves_unspecified_fields_unchanged() {
        let mut config = ServerConfig::default();
        // Only update one inlay hint field
        config.update_from_value(&json!({
            "inlayHints": { "enabled": false }
        }));
        assert!(!config.inlay_hints_enabled, "updated field changes");
        assert!(config.inlay_hints_parameter_hints, "unspecified field unchanged");
        assert_eq!(config.inlay_hints_max_length, 30, "unspecified field unchanged");
        assert_eq!(config.test_runner_command, "perl", "unrelated section unchanged");
    }

    #[test]
    fn server_config_empty_update_leaves_all_defaults_unchanged() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({}));
        assert!(config.inlay_hints_enabled);
        assert_eq!(config.test_runner_command, "perl");
        assert!(!config.telemetry_enabled);
    }

    #[test]
    fn server_config_test_runner_timeout_updated() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({
            "testRunner": { "timeout": 30000 }
        }));
        assert_eq!(config.test_runner_timeout, 30000);
    }

    // ── Perlcritic extended config ────────────────────────────

    #[test]
    fn server_config_default_perlcritic_severity_is_three() {
        let config = ServerConfig::default();
        assert_eq!(config.perlcritic_severity, 3, "default severity should be 3 (Harsh)");
    }

    #[test]
    fn server_config_default_perlcritic_profile_is_none() {
        let config = ServerConfig::default();
        assert!(config.perlcritic_profile.is_none(), "profile is None by default");
    }

    #[test]
    fn server_config_perlcritic_severity_updated_via_settings() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({ "perlcritic": { "severity": 1 } }));
        assert_eq!(config.perlcritic_severity, 1);
    }

    #[test]
    fn server_config_perlcritic_severity_clamped_to_five() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({ "perlcritic": { "severity": 99 } }));
        assert_eq!(config.perlcritic_severity, 5, "severity clamped to max 5");
    }

    #[test]
    fn server_config_perlcritic_severity_clamped_to_one() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({ "perlcritic": { "severity": 0 } }));
        assert_eq!(config.perlcritic_severity, 1, "severity clamped to min 1");
    }

    #[test]
    fn server_config_perlcritic_profile_updated_via_settings() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({ "perlcritic": { "profile": "/path/to/.perlcriticrc" } }));
        assert_eq!(config.perlcritic_profile, Some("/path/to/.perlcriticrc".to_string()));
    }

    #[test]
    fn server_config_perlcritic_all_fields_together() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({
            "perlcritic": {
                "enabled": true,
                "severity": 2,
                "profile": "/workspace/.perlcriticrc"
            }
        }));
        assert!(config.perlcritic_enabled);
        assert_eq!(config.perlcritic_severity, 2);
        assert_eq!(config.perlcritic_profile, Some("/workspace/.perlcriticrc".to_string()));
    }

    #[test]
    fn server_config_perlcritic_partial_update_preserves_other_fields() {
        let mut config = ServerConfig::default();
        config.update_from_value(&json!({ "perlcritic": { "enabled": true } }));
        // severity and profile should still be at defaults
        assert_eq!(config.perlcritic_severity, 3);
        assert!(config.perlcritic_profile.is_none());
    }

    // ── WorkspaceConfig defaults ──────────────────────────────

    #[test]
    fn workspace_config_defaults_include_common_paths() {
        let config = WorkspaceConfig::default();
        assert_eq!(config.include_paths, vec!["lib", ".", "local/lib/perl5"]);
        assert!(!config.use_system_inc);
        assert_eq!(config.resolution_timeout_ms, 50);
    }

    // ── WorkspaceConfig::update_from_value ───────────────────

    #[test]
    fn workspace_config_updates_include_paths() {
        let mut config = WorkspaceConfig::default();
        config.update_from_value(&json!({
            "workspace": { "includePaths": ["/custom/lib", "/other/lib"] }
        }));
        assert_eq!(config.include_paths, vec!["/custom/lib", "/other/lib"]);
    }

    #[test]
    fn workspace_config_updates_resolution_timeout() {
        let mut config = WorkspaceConfig::default();
        config.update_from_value(&json!({
            "workspace": { "resolutionTimeout": 100 }
        }));
        assert_eq!(config.resolution_timeout_ms, 100);
    }

    #[test]
    fn workspace_config_empty_update_leaves_defaults() {
        let mut config = WorkspaceConfig::default();
        config.update_from_value(&json!({}));
        assert_eq!(config.include_paths, vec!["lib", ".", "local/lib/perl5"]);
        assert!(!config.use_system_inc);
    }

    // ── WorkspaceConfig::get_system_inc ──────────────────────

    #[test]
    fn workspace_config_get_system_inc_returns_empty_when_disabled() {
        let mut config = WorkspaceConfig::default();
        // use_system_inc = false (default)
        let inc = config.get_system_inc();
        assert!(inc.is_empty(), "system inc is empty when use_system_inc=false");
    }
}
