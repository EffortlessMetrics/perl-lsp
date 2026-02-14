//! Server configuration management
//!
//! Runtime configuration for the LSP server, including inlay hints,
//! test runner settings, and workspace module resolution configuration.

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

    /// Path to perltidy configuration file.
    pub perltidy_config: Option<String>,
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
            perltidy_config: None,
        }
    }
}

impl ServerConfig {
    /// Update configuration from LSP settings
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(config) = settings.get("perltidyConfig").and_then(|v| v.as_str()) {
            // Treat empty string as None
            self.perltidy_config = if config.is_empty() {
                None
            } else {
                Some(config.to_string())
            };
        }

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

        if let Some(telemetry) = settings.get("telemetry") {
            if let Some(enabled) = telemetry.get("enabled").and_then(|v| v.as_bool()) {
                self.telemetry_enabled = enabled;
            }
        }
    }
}

/// Workspace configuration for module resolution
///
/// Controls how the LSP server resolves module imports and finds
/// Perl module files across the workspace. Implements a deterministic
/// precedence order for reliable cross-workspace navigation.
///
/// ## Resolution Precedence Order
///
/// 1. **Open Documents** - Already-opened documents take highest priority
/// 2. **Workspace Folders** - Searched in initialization order (multi-root aware)
/// 3. **Configured Include Paths** - User-specified directories per folder
/// 4. **System @INC** - Opt-in only, filtered for security
///
/// ## Performance
///
/// - Resolution timeout prevents blocking on slow/network filesystems
/// - System @INC is lazily populated only when enabled
/// - Default configuration matches typical Perl project layouts
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
    /// Update workspace configuration from LSP settings
    ///
    /// Reads from the `workspace` section of settings:
    /// - `workspace.includePaths`: Array of relative paths
    /// - `workspace.useSystemInc`: Boolean to enable @INC lookup
    /// - `workspace.resolutionTimeout`: Timeout in milliseconds
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(workspace) = settings.get("workspace") {
            if let Some(paths) = workspace.get("includePaths").and_then(|v| v.as_array()) {
                self.include_paths =
                    paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            }
            if let Some(use_inc) = workspace.get("useSystemInc").and_then(|v| v.as_bool()) {
                // Clear cache if setting changed
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

    /// Get system @INC paths (lazily populated)
    ///
    /// Returns empty slice if `use_system_inc` is false.
    /// Otherwise, fetches @INC from perl interpreter on first call.
    ///
    /// # Security Notes
    ///
    /// - **Opt-in only**: System @INC is disabled by default (`use_system_inc: false`)
    /// - **Trusted source**: Paths come from user's configured `perl` executable/environment
    /// - **Filtered**: Excludes `.` (current directory) to prevent injection attacks
    /// - **Expectation**: Returned paths should be absolute; relative paths from exotic
    ///   @INC configurations may behave unexpectedly
    pub fn get_system_inc(&mut self) -> &[PathBuf] {
        if !self.use_system_inc {
            return &[];
        }

        if self.system_inc_cache.is_none() {
            self.system_inc_cache = Some(Self::fetch_perl_inc());
        }

        self.system_inc_cache.as_deref().unwrap_or(&[])
    }

    /// Fetch @INC from perl interpreter
    ///
    /// Filters out "." for security (prevents current directory injection).
    #[cfg(not(target_arch = "wasm32"))]
    fn fetch_perl_inc() -> Vec<PathBuf> {
        let output = Command::new("perl").args(["-e", "print join(\"\\n\", @INC)"]).output();

        match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.is_empty() && *l != ".")
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
