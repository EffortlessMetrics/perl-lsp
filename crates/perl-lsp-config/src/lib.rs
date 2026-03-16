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
