#![warn(missing_docs)]
//! Configuration models for perl-lsp server runtime state.
//!
//! Absorbed from `perl-lsp-config` crate into `perl-lsp-rs-core`
//! as part of Wave Final PR B (#4541). This module isolates configuration
//! parsing and defaults from the main server crate so they can evolve
//! independently and be reused by tooling.

#[cfg(not(target_arch = "wasm32"))]
use crate::platform::resolve_perl_path_with_toolchain;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

mod native_build_hints;

pub use native_build_hints::{NativeBuildHints, detect_native_build_hints};

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
    /// be installed on the system; missing binary/profile/runtime failures are
    /// surfaced as workspace warnings.
    pub perlcritic_enabled: bool,

    /// Minimum Perl::Critic severity level to report (1-5, where 5 = most severe).
    ///
    /// `perlcritic --severity N` reports violations at or above `N`.
    /// With this scale, `1` reports everything while `5` reports only the
    /// highest-severity violations. Default is 3 (Harsh).
    /// Equivalent to `perlcritic --severity`.
    pub perlcritic_severity: u8,

    /// Path to a `.perlcriticrc` profile file.
    ///
    /// When `Some`, passes `--profile=<path>` to perlcritic. When `None`,
    /// the auto-discovery logic looks for `.perlcriticrc` in the workspace root.
    pub perlcritic_profile: Option<String>,

    /// Whether perltidy formatting is enabled.
    pub perltidy_enabled: bool,

    /// Path to a `.perltidyrc` profile file.
    ///
    /// When `Some`, passes `--profile=<path>` to perltidy. When `None`,
    /// perltidy uses its default behavior or auto-discovers a profile.
    pub perltidy_profile: Option<String>,

    /// Maximum line length for perltidy.
    pub perltidy_maximum_line_length: Option<u32>,

    /// Indent size in spaces for perltidy.
    pub perltidy_indent_columns: Option<u32>,

    /// Use tabs instead of spaces for perltidy.
    pub perltidy_tabs: Option<bool>,

    /// Opening brace on new line for perltidy.
    pub perltidy_opening_brace_on_new_line: Option<bool>,

    /// Cuddled else style for perltidy.
    pub perltidy_cuddled_else: Option<bool>,

    /// Space after keyword for perltidy.
    pub perltidy_space_after_keyword: Option<bool>,

    /// Add trailing commas for perltidy.
    pub perltidy_add_trailing_commas: Option<bool>,

    /// Vertical alignment for perltidy.
    pub perltidy_vertical_alignment: Option<bool>,

    /// Block comment indentation for perltidy.
    pub perltidy_block_comment_indentation: Option<u32>,

    /// Extra perltidy arguments.
    pub perltidy_extra_args: Vec<String>,

    /// Timeout in seconds for perltidy.
    pub perltidy_timeout_secs: u64,

    /// AI-powered inline completion configuration.
    pub ai_completion: AiCompletionConfig,
}

/// Configuration for AI-powered inline completions.
///
/// Disabled by default. When enabled, the server calls an external AI provider
/// for inline completion suggestions, falling back to deterministic rules on
/// timeout, error, or when AI is disabled.
#[derive(Debug, Clone)]
pub struct AiCompletionConfig {
    /// Whether AI completions are enabled. Default: false.
    pub enabled: bool,
    /// Provider type. Currently only "openai_compat" is supported.
    pub provider: String,
    /// API endpoint URL.
    pub endpoint: String,
    /// Model identifier (e.g., "gpt-4o-mini").
    pub model: String,
    /// Environment variable name containing the API key.
    pub api_key_env: String,
    /// Request timeout in milliseconds. Default: 1800.
    pub timeout_ms: u64,
    /// Maximum output tokens per request. Default: 64.
    pub max_output_tokens: u32,
    /// Maximum requests per second. Default: 1.
    pub rate_limit_rps: f64,
    /// Maximum concurrent in-flight requests. Default: 1.
    pub max_inflight: u32,
    /// Whether to fall back to deterministic completions on AI failure. Default: true.
    pub fallback: bool,
    /// Streaming-specific configuration.
    pub streaming: AiStreamingConfig,
}

/// Streaming sub-configuration for AI completions.
#[derive(Debug, Clone)]
pub struct AiStreamingConfig {
    /// Whether streaming mode is enabled. Default: true.
    pub enabled: bool,
    /// Minimum milliseconds between emitted updates. Default: 60.
    pub update_debounce_ms: u64,
}

impl Default for AiCompletionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openai_compat".to_string(),
            endpoint: String::new(),
            model: "gpt-4o-mini".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            timeout_ms: 1800,
            max_output_tokens: 64,
            rate_limit_rps: 1.0,
            max_inflight: 1,
            fallback: true,
            streaming: AiStreamingConfig::default(),
        }
    }
}

impl Default for AiStreamingConfig {
    fn default() -> Self {
        Self { enabled: true, update_debounce_ms: 60 }
    }
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
            perltidy_enabled: true,
            perltidy_profile: None,
            perltidy_maximum_line_length: Some(80),
            perltidy_indent_columns: Some(4),
            perltidy_tabs: Some(false),
            perltidy_opening_brace_on_new_line: Some(false),
            perltidy_cuddled_else: Some(true),
            perltidy_space_after_keyword: Some(true),
            perltidy_add_trailing_commas: Some(false),
            perltidy_vertical_alignment: Some(true),
            perltidy_block_comment_indentation: Some(0),
            perltidy_extra_args: Vec::new(),
            perltidy_timeout_secs: 10,
            ai_completion: AiCompletionConfig::default(),
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
                let profile = profile.trim();
                self.perlcritic_profile = (!profile.is_empty()).then(|| profile.to_string());
            }
        }

        if let Some(formatting) = settings.get("formatting") {
            if let Some(enabled) = formatting.get("enabled").and_then(|v| v.as_bool()) {
                self.perltidy_enabled = enabled;
            }
            if let Some(profile) = formatting.get("profile").and_then(|v| v.as_str()) {
                let profile = profile.trim();
                self.perltidy_profile = (!profile.is_empty()).then(|| profile.to_string());
            }
            if let Some(len) = formatting.get("maximumLineLength").and_then(|v| v.as_u64()) {
                self.perltidy_maximum_line_length = Some(len as u32);
            }
            if let Some(indent) = formatting.get("indentColumns").and_then(|v| v.as_u64()) {
                self.perltidy_indent_columns = Some(indent as u32);
            }
            if let Some(tabs) = formatting.get("tabs").and_then(|v| v.as_bool()) {
                self.perltidy_tabs = Some(tabs);
            }
            if let Some(brace) = formatting.get("openingBraceOnNewLine").and_then(|v| v.as_bool()) {
                self.perltidy_opening_brace_on_new_line = Some(brace);
            }
            if let Some(cuddle) = formatting.get("cuddledElse").and_then(|v| v.as_bool()) {
                self.perltidy_cuddled_else = Some(cuddle);
            }
            if let Some(space) = formatting.get("spaceAfterKeyword").and_then(|v| v.as_bool()) {
                self.perltidy_space_after_keyword = Some(space);
            }
            if let Some(comma) = formatting.get("addTrailingCommas").and_then(|v| v.as_bool()) {
                self.perltidy_add_trailing_commas = Some(comma);
            }
            if let Some(align) = formatting.get("verticalAlignment").and_then(|v| v.as_bool()) {
                self.perltidy_vertical_alignment = Some(align);
            }
            if let Some(block) = formatting.get("blockCommentIndentation").and_then(|v| v.as_u64())
            {
                self.perltidy_block_comment_indentation = Some(block as u32);
            }
            if let Some(args) = formatting.get("extraArgs").and_then(|v| v.as_array()) {
                self.perltidy_extra_args =
                    args.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            }
            if let Some(timeout) = formatting.get("timeoutSecs").and_then(|v| v.as_u64()) {
                self.perltidy_timeout_secs = timeout;
            }
        }

        if let Some(ai) = settings.get("aiCompletion") {
            if let Some(enabled) = ai.get("enabled").and_then(|v| v.as_bool()) {
                self.ai_completion.enabled = enabled;
            }
            if let Some(provider) = ai.get("provider").and_then(|v| v.as_str()) {
                self.ai_completion.provider = provider.to_string();
            }
            if let Some(endpoint) = ai.get("endpoint").and_then(|v| v.as_str()) {
                self.ai_completion.endpoint = endpoint.to_string();
            }
            if let Some(model) = ai.get("model").and_then(|v| v.as_str()) {
                self.ai_completion.model = model.to_string();
            }
            if let Some(key_env) = ai.get("apiKeyEnv").and_then(|v| v.as_str()) {
                self.ai_completion.api_key_env = key_env.to_string();
            }
            if let Some(timeout) = ai.get("timeoutMs").and_then(|v| v.as_u64()) {
                self.ai_completion.timeout_ms = timeout;
            }
            if let Some(tokens) = ai.get("maxOutputTokens").and_then(|v| v.as_u64()) {
                self.ai_completion.max_output_tokens = tokens as u32;
            }
            if let Some(rps) = ai.get("rateLimitRps").and_then(|v| v.as_f64()) {
                self.ai_completion.rate_limit_rps = rps;
            }
            if let Some(inflight) = ai.get("maxInflight").and_then(|v| v.as_u64()) {
                self.ai_completion.max_inflight = inflight as u32;
            }
            if let Some(fallback) = ai.get("fallback").and_then(|v| v.as_bool()) {
                self.ai_completion.fallback = fallback;
            }
            if let Some(streaming) = ai.get("streaming") {
                if let Some(enabled) = streaming.get("enabled").and_then(|v| v.as_bool()) {
                    self.ai_completion.streaming.enabled = enabled;
                }
                if let Some(debounce) = streaming.get("updateDebounceMs").and_then(|v| v.as_u64()) {
                    self.ai_completion.streaming.update_debounce_ms = debounce;
                }
            }
        }
    }
}

/// Controls whether PERL5LIB paths are prepended or appended to `include_paths`.
///
/// `Prepend` (the default) mirrors Perl's own behaviour: paths earlier in the
/// search order shadow later ones, so PERL5LIB paths take priority over any
/// project-level `include_paths`.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Perl5LibPrecedence {
    /// PERL5LIB entries are placed *before* `include_paths` (default).
    #[default]
    Prepend,
    /// PERL5LIB entries are placed *after* `include_paths`.
    Append,
}

/// Workspace configuration for module resolution
///
/// Controls how the LSP server resolves module imports and finds
/// Perl module files across the workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Workspace-root-relative include paths for module resolution.
    ///
    /// Relative entries are resolved against the workspace root. Absolute
    /// entries are honored literally as external include roots.
    /// Default: `["lib", ".", "local/lib/perl5"]`
    pub include_paths: Vec<String>,

    /// Whether to include system @INC paths in module resolution
    /// Default: false (avoids blocking on network filesystems)
    pub use_system_inc: bool,

    /// Cached system @INC paths (populated lazily when use_system_inc is true)
    system_inc_cache: Option<Vec<PathBuf>>,

    /// Perl interpreter used for startup `@INC` probing.
    ///
    /// When unset, falls back to `perl` on `PATH`.
    pub perl_path: Option<String>,

    /// Extra arguments passed to the Perl interpreter for startup `@INC` probing.
    pub perl_args: Vec<String>,

    /// Native build hints derived from workspace-root `Makefile.PL` / `Build.PL`.
    ///
    /// These are cached once at workspace initialization and kept separate from
    /// Perl module search paths.
    pub native_build_hints: NativeBuildHints,

    /// Resolution timeout in milliseconds
    /// Default: 50ms
    pub resolution_timeout_ms: u64,

    /// Whether the `PERL5LIB` environment variable is read and merged into
    /// the module search path.  Default: `true`.
    pub use_perl5lib: bool,

    /// Controls whether PERL5LIB entries come before or after `include_paths`.
    /// Default: `Prepend` (mirrors Perl's own search order).
    pub perl5lib_precedence: Perl5LibPrecedence,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            include_paths: vec!["lib".to_string(), ".".to_string(), "local/lib/perl5".to_string()],
            use_system_inc: false,
            system_inc_cache: None,
            perl_path: None,
            perl_args: Vec::new(),
            native_build_hints: NativeBuildHints::default(),
            resolution_timeout_ms: 50,
            use_perl5lib: true,
            perl5lib_precedence: Perl5LibPrecedence::Prepend,
        }
    }
}

fn dedupe_preserve_order<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut result = Vec::new();
    for path in paths {
        if !result.iter().any(|existing| existing == path) {
            result.push(path.to_string());
        }
    }
    result
}

impl WorkspaceConfig {
    /// Parse a `PERL5LIB` environment variable value into a list of paths.
    ///
    /// Uses `:` as the separator on Unix and `;` on Windows, matching Perl's
    /// own behaviour.  Empty components (produced by leading, trailing, or
    /// consecutive separators) are silently dropped.
    pub fn parse_perl5lib(value: &str) -> Vec<String> {
        #[cfg(windows)]
        const SEP: char = ';';
        #[cfg(not(windows))]
        const SEP: char = ':';
        dedupe_preserve_order(value.split(SEP).map(str::trim).filter(|s| !s.is_empty()))
    }

    /// Return the effective module-search-path, merging `PERL5LIB` paths with
    /// `self.include_paths` according to `self.perl5lib_precedence`.
    ///
    /// If `self.use_perl5lib` is `false`, or `perl5lib_paths` is empty, the
    /// returned list contains only `self.include_paths` entries (trimmed and deduplicated).
    pub fn effective_include_paths(&self, perl5lib_paths: &[String]) -> Vec<String> {
        if !self.use_perl5lib || perl5lib_paths.is_empty() {
            return dedupe_preserve_order(
                self.include_paths
                    .iter()
                    .map(String::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
            );
        }
        match self.perl5lib_precedence {
            Perl5LibPrecedence::Prepend => dedupe_preserve_order(
                perl5lib_paths
                    .iter()
                    .map(String::as_str)
                    .chain(self.include_paths.iter().map(String::as_str))
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
            ),
            Perl5LibPrecedence::Append => dedupe_preserve_order(
                self.include_paths
                    .iter()
                    .map(String::as_str)
                    .chain(perl5lib_paths.iter().map(String::as_str))
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
            ),
        }
    }

    /// Refresh workspace-native build hints from the selected workspace root.
    ///
    /// This is a workspace-initialization cache step only; it does not mutate
    /// module-resolution include paths.
    pub fn refresh_native_build_hints(&mut self, workspace_root: &Path) {
        self.native_build_hints = detect_native_build_hints(workspace_root);
    }

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
            if let Some(perl_path) = workspace.get("perlPath").and_then(|v| v.as_str()) {
                let next = Some(perl_path.to_string());
                if next != self.perl_path {
                    self.system_inc_cache = None;
                }
                self.perl_path = next;
            }
            if let Some(perl_args) = workspace.get("perlArgs").and_then(|v| v.as_array()) {
                let next: Vec<String> =
                    perl_args.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
                if next != self.perl_args {
                    self.system_inc_cache = None;
                }
                self.perl_args = next;
            }
            if let Some(timeout) = workspace.get("resolutionTimeout").and_then(|v| v.as_u64()) {
                self.resolution_timeout_ms = timeout;
            }
            if let Some(use_p5l) = workspace.get("usePerl5lib").and_then(|v| v.as_bool()) {
                self.use_perl5lib = use_p5l;
            }
            if let Some(prec) = workspace.get("perl5libPrecedence").and_then(|v| v.as_str()) {
                // Only update on recognised values; leave the current setting unchanged for
                // unknown strings so a typo does not silently reset an explicitly-set Append.
                match prec {
                    "append" => self.perl5lib_precedence = Perl5LibPrecedence::Append,
                    "prepend" => self.perl5lib_precedence = Perl5LibPrecedence::Prepend,
                    _ => {} // unknown value — leave current setting intact
                }
            }
        }
    }

    /// Get system @INC paths (lazily populated).
    pub fn get_system_inc(&mut self) -> &[PathBuf] {
        if !self.use_system_inc {
            return &[];
        }

        if self.system_inc_cache.is_none() {
            self.system_inc_cache =
                Some(Self::fetch_perl_inc(self.perl_path.as_deref(), &self.perl_args));
        }

        self.system_inc_cache.as_deref().unwrap_or(&[])
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fetch_perl_inc(perl_path: Option<&str>, perl_args: &[String]) -> Vec<PathBuf> {
        let perl_path = match perl_path.filter(|path| !path.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => match resolve_perl_path_with_toolchain() {
                Ok(path) => path,
                Err(_) => return Vec::new(),
            },
        };
        let mut command = Command::new(perl_path);
        command.args(perl_args);
        let output = command.args(["-e", "print join(\"\\n\", @INC)"]).output();

        match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != ".")
                .map(PathBuf::from)
                .fold(Vec::new(), |mut acc, path| {
                    if !acc.contains(&path) {
                        acc.push(path);
                    }
                    acc
                }),
            _ => Vec::new(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn fetch_perl_inc(_: Option<&str>, _: &[String]) -> Vec<PathBuf> {
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
    /// `[ai_completion]` section: AI completion settings.
    pub ai_completion: ProjectAiCompletionConfig,
    /// `[formatting]` section: perltidy configuration.
    pub formatting: ProjectFormattingConfig,
}

/// `[perl]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectPerlConfig {
    /// Additional include paths for module resolution.
    ///
    /// Relative entries are resolved against the workspace root. Absolute
    /// entries are honored literally as external include roots.
    pub include_paths: Vec<String>,
    /// Perl version string (e.g. "5.38") — parsed but not yet wired to diagnostics.
    /// Reserved for future use; ignored in this implementation.
    pub version: Option<String>,
    /// Whether to read `PERL5LIB` from the environment and include it in the
    /// module search path.  Unset means "leave the server default unchanged".
    pub use_perl5lib: Option<bool>,
    /// Whether PERL5LIB paths come before or after `include_paths`.
    /// Unset means "leave the server default unchanged".
    pub perl5lib_precedence: Option<Perl5LibPrecedence>,
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

/// `[ai_completion]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectAiCompletionConfig {
    /// Whether AI completions are enabled.
    pub enabled: Option<bool>,
    /// Provider type.
    pub provider: Option<String>,
    /// API endpoint URL.
    pub endpoint: Option<String>,
    /// Model identifier.
    pub model: Option<String>,
    /// Environment variable name for API key.
    pub api_key_env: Option<String>,
}

/// `[formatting]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectFormattingConfig {
    /// Whether perltidy formatting is enabled.
    pub enabled: Option<bool>,
    /// Path to a `.perltidyrc` profile file.
    pub perltidy_profile: Option<String>,
    /// Maximum line length.
    pub perltidy_maximum_line_length: Option<u32>,
    /// Indent size in spaces.
    pub perltidy_indent_columns: Option<u32>,
    /// Use tabs instead of spaces.
    pub perltidy_tabs: Option<bool>,
    /// Opening brace on new line.
    pub perltidy_opening_brace_on_new_line: Option<bool>,
    /// Cuddled else style.
    pub perltidy_cuddled_else: Option<bool>,
    /// Space after keyword.
    pub perltidy_space_after_keyword: Option<bool>,
    /// Add trailing commas.
    pub perltidy_add_trailing_commas: Option<bool>,
    /// Vertical alignment.
    pub perltidy_vertical_alignment: Option<bool>,
    /// Block comment indentation.
    pub perltidy_block_comment_indentation: Option<u32>,
    /// Extra perltidy arguments.
    pub perltidy_extra_args: Vec<String>,
    /// Timeout in seconds.
    pub perltidy_timeout_secs: Option<u64>,
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
        Err(e) => Err(format!(
            "Could not read .perl-lsp.toml: {}. \
             Check that the file is readable and not locked by another process.",
            e
        )),
        Ok(content) => toml::from_str::<ProjectConfig>(&content)
            .map(Some)
            .map_err(|e| format!(".perl-lsp.toml has a syntax error: {}", e)),
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
        if let Some(enabled) = self.ai_completion.enabled {
            config.ai_completion.enabled = enabled;
        }
        if let Some(ref provider) = self.ai_completion.provider {
            config.ai_completion.provider = provider.clone();
        }
        if let Some(ref endpoint) = self.ai_completion.endpoint {
            config.ai_completion.endpoint = endpoint.clone();
        }
        if let Some(ref model) = self.ai_completion.model {
            config.ai_completion.model = model.clone();
        }
        if let Some(ref key_env) = self.ai_completion.api_key_env {
            config.ai_completion.api_key_env = key_env.clone();
        }

        // Apply formatting configuration
        if let Some(enabled) = self.formatting.enabled {
            config.perltidy_enabled = enabled;
        }
        if let Some(ref profile) = self.formatting.perltidy_profile {
            config.perltidy_profile = Some(profile.clone());
        }
        if let Some(len) = self.formatting.perltidy_maximum_line_length {
            config.perltidy_maximum_line_length = Some(len);
        }
        if let Some(indent) = self.formatting.perltidy_indent_columns {
            config.perltidy_indent_columns = Some(indent);
        }
        if let Some(tabs) = self.formatting.perltidy_tabs {
            config.perltidy_tabs = Some(tabs);
        }
        if let Some(brace) = self.formatting.perltidy_opening_brace_on_new_line {
            config.perltidy_opening_brace_on_new_line = Some(brace);
        }
        if let Some(cuddle) = self.formatting.perltidy_cuddled_else {
            config.perltidy_cuddled_else = Some(cuddle);
        }
        if let Some(space) = self.formatting.perltidy_space_after_keyword {
            config.perltidy_space_after_keyword = Some(space);
        }
        if let Some(comma) = self.formatting.perltidy_add_trailing_commas {
            config.perltidy_add_trailing_commas = Some(comma);
        }
        if let Some(align) = self.formatting.perltidy_vertical_alignment {
            config.perltidy_vertical_alignment = Some(align);
        }
        if let Some(block) = self.formatting.perltidy_block_comment_indentation {
            config.perltidy_block_comment_indentation = Some(block);
        }
        if !self.formatting.perltidy_extra_args.is_empty() {
            config.perltidy_extra_args = self.formatting.perltidy_extra_args.clone();
        }
        if let Some(timeout) = self.formatting.perltidy_timeout_secs {
            config.perltidy_timeout_secs = timeout;
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
        if let Some(use_p5l) = self.perl.use_perl5lib {
            config.use_perl5lib = use_p5l;
        }
        if let Some(ref prec) = self.perl.perl5lib_precedence {
            config.perl5lib_precedence = prec.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn load_project_config_returns_none_when_missing() -> TestResult {
        let temp = tempfile::tempdir()?;
        let config = load_project_config(temp.path())?;
        assert!(config.is_none());
        Ok(())
    }

    #[test]
    fn load_project_config_returns_parse_error_for_invalid_toml() -> TestResult {
        let temp = tempfile::tempdir()?;
        std::fs::write(temp.path().join(".perl-lsp.toml"), "[perl\ninclude_paths = [\"lib\"]")?;

        let err = load_project_config(temp.path())
            .err()
            .ok_or("expected invalid TOML to return an error")?;
        assert!(err.contains("syntax error"));
        Ok(())
    }

    #[test]
    fn load_project_config_parses_known_sections() -> TestResult {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join(".perl-lsp.toml"),
            r#"
[perl]
include_paths = ["lib", "t/lib"]
use_perl5lib = true
perl5lib_precedence = "prepend"

[diagnostics]
perlcritic = true
perlcritic_severity = 4

[features]
inlay_hints = false

[formatting]
enabled = true
perltidy_maximum_line_length = 100
perltidy_extra_args = ["-noll"]
"#,
        )?;

        let config = load_project_config(temp.path())?.ok_or("expected parsed project config")?;

        assert_eq!(config.perl.include_paths, vec!["lib", "t/lib"]);
        assert_eq!(config.perl.use_perl5lib, Some(true));
        assert!(matches!(config.perl.perl5lib_precedence, Some(Perl5LibPrecedence::Prepend)));
        assert_eq!(config.diagnostics.perlcritic, Some(true));
        assert_eq!(config.diagnostics.perlcritic_severity, Some(4));
        assert_eq!(config.features.inlay_hints, Some(false));
        assert_eq!(config.formatting.enabled, Some(true));
        assert_eq!(config.formatting.perltidy_maximum_line_length, Some(100));
        assert_eq!(config.formatting.perltidy_extra_args, vec!["-noll"]);
        Ok(())
    }

    #[test]
    fn apply_to_server_config_clamps_perlcritic_severity() {
        let mut config = ServerConfig::default();
        let mut project = ProjectConfig::default();
        project.diagnostics.perlcritic_severity = Some(99);

        project.apply_to_server_config(&mut config);

        assert_eq!(config.perlcritic_severity, 5);
    }

    #[test]
    fn apply_to_server_config_does_not_overwrite_unset_values() {
        let mut config = ServerConfig {
            perlcritic_enabled: true,
            inlay_hints_enabled: true,
            ..ServerConfig::default()
        };
        let project = ProjectConfig::default();

        project.apply_to_server_config(&mut config);

        assert!(config.perlcritic_enabled);
        assert!(config.inlay_hints_enabled);
    }

    #[test]
    fn apply_to_workspace_config_only_overrides_non_empty_include_paths() {
        let mut workspace = WorkspaceConfig::default();
        let baseline_include_paths = workspace.include_paths.clone();

        let mut project = ProjectConfig::default();
        project.apply_to_workspace_config(&mut workspace);
        assert_eq!(workspace.include_paths, baseline_include_paths);

        project.perl.include_paths = vec!["custom/lib".to_string()];
        project.apply_to_workspace_config(&mut workspace);
        assert_eq!(workspace.include_paths, vec!["custom/lib"]);
    }

    #[test]
    fn apply_to_workspace_config_sets_perl5lib_toggles() {
        let mut workspace = WorkspaceConfig::default();
        let mut project = ProjectConfig::default();
        project.perl.use_perl5lib = Some(false);
        project.perl.perl5lib_precedence = Some(Perl5LibPrecedence::Append);

        project.apply_to_workspace_config(&mut workspace);

        assert!(!workspace.use_perl5lib);
        assert!(matches!(workspace.perl5lib_precedence, Perl5LibPrecedence::Append));
    }

    #[test]
    fn parse_perl5lib_trims_and_dedupes_entries() {
        // Use the platform separator so the test works on both Unix and Windows.
        #[cfg(windows)]
        let input = " lib ;local/lib;;lib; ";
        #[cfg(not(windows))]
        let input = " lib :local/lib::lib: ";
        let parsed = WorkspaceConfig::parse_perl5lib(input);
        assert_eq!(parsed, vec!["lib", "local/lib"]);
    }

    #[test]
    fn effective_include_paths_dedupes_with_prepend_precedence() {
        let config = WorkspaceConfig {
            include_paths: vec!["lib".to_string(), "local/lib".to_string(), "lib".to_string()],
            perl5lib_precedence: Perl5LibPrecedence::Prepend,
            ..WorkspaceConfig::default()
        };

        let paths = config.effective_include_paths(&[
            "local/lib".to_string(),
            "vendor/lib".to_string(),
            "vendor/lib".to_string(),
        ]);

        assert_eq!(paths, vec!["local/lib", "vendor/lib", "lib"]);
    }

    #[test]
    fn effective_include_paths_dedupes_with_append_precedence() {
        let config = WorkspaceConfig {
            include_paths: vec!["lib".to_string(), "local/lib".to_string()],
            perl5lib_precedence: Perl5LibPrecedence::Append,
            ..WorkspaceConfig::default()
        };

        let paths = config.effective_include_paths(&[
            "local/lib".to_string(),
            "vendor/lib".to_string(),
            "lib".to_string(),
        ]);

        assert_eq!(paths, vec!["lib", "local/lib", "vendor/lib"]);
    }

    #[test]
    fn effective_include_paths_filters_whitespace_only_entries() {
        // Whitespace-only entries in include_paths must be silently dropped.
        let config = WorkspaceConfig {
            include_paths: vec![
                "lib".to_string(),
                "  ".to_string(),
                "".to_string(),
                "lib".to_string(),
            ],
            perl5lib_precedence: Perl5LibPrecedence::Prepend,
            ..WorkspaceConfig::default()
        };
        // use_perl5lib is true by default but perl5lib_paths is empty → takes the
        // early-return branch that also dedupes and trims include_paths.
        let paths = config.effective_include_paths(&[]);
        assert_eq!(paths, vec!["lib"]);
    }

    #[test]
    fn update_from_value_keeps_existing_perl5lib_precedence_on_unknown_value() {
        let mut config = WorkspaceConfig {
            perl5lib_precedence: Perl5LibPrecedence::Append,
            ..WorkspaceConfig::default()
        };

        let settings = serde_json::json!({
            "workspace": {
                "perl5libPrecedence": "sideways"
            }
        });

        config.update_from_value(&settings);

        assert!(matches!(config.perl5lib_precedence, Perl5LibPrecedence::Append));
    }

    #[test]
    fn update_from_value_clears_system_inc_cache_when_perl_runtime_inputs_change() {
        let mut config = WorkspaceConfig::default();
        config.use_system_inc = true;
        config.system_inc_cache = Some(vec![PathBuf::from("/cached/inc")]);

        let perl_path_change = serde_json::json!({
            "workspace": {
                "perlPath": "/usr/bin/perl"
            }
        });
        config.update_from_value(&perl_path_change);
        assert!(config.system_inc_cache.is_none());

        config.system_inc_cache = Some(vec![PathBuf::from("/cached/inc")]);
        let perl_args_change = serde_json::json!({
            "workspace": {
                "perlArgs": ["-Ilib"]
            }
        });
        config.update_from_value(&perl_args_change);
        assert!(config.system_inc_cache.is_none());

        config.system_inc_cache = Some(vec![PathBuf::from("/cached/inc")]);
        let use_system_inc_change = serde_json::json!({
            "workspace": {
                "useSystemInc": false
            }
        });
        config.update_from_value(&use_system_inc_change);
        assert!(config.system_inc_cache.is_none());
    }
}
