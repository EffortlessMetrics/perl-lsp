#![warn(missing_docs)]
//! Configuration models for perl-lsp server runtime state.
//!
//! Absorbed from `perl-lsp-config` crate into `perl-lsp-rs-core`
//! as part of Wave Final PR B (#4541). This module isolates configuration
//! parsing and defaults from the main server crate so they can evolve
//! independently and be reused by tooling.

#[cfg(all(test, not(target_arch = "wasm32")))]
use crate::platform::resolve_perl_path_with_toolchain;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Output, Stdio};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use std::{fs::File, io::Read};

mod native_build_hints;
pub mod perl_oracle_env;

pub use native_build_hints::{NativeBuildHints, detect_native_build_hints};
pub use perl_lsp_perltidy::FormatterMode;
#[cfg(not(target_arch = "wasm32"))]
pub use perl_oracle_env::PerlOracleEnv;

/// Critic diagnostic engine used for LSP policy diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CriticEngine {
    /// Existing built-in/external Perl::Critic-compatible path.
    #[default]
    Legacy,
    /// Rust-native critic rule registry.
    Native,
}

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

    /// Whether critic diagnostics are enabled.
    ///
    /// The default engine is native and does not require `perlcritic`. Projects
    /// can select the legacy/external engine when exact Perl::Critic behavior is
    /// required.
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

    /// Optional Perl::Critic theme expression.
    ///
    /// When `Some`, passes `--theme=<expr>` to perlcritic.
    pub perlcritic_theme: Option<String>,

    /// Critic engine used for LSP policy diagnostics.
    pub critic_engine: CriticEngine,

    /// Native critic profile used when `critic_engine` is [`CriticEngine::Native`].
    ///
    /// Defaults to `recommended` for the lower-noise native rule bundle. Projects
    /// can select `strict` for the full native rule surface.
    pub native_critic_profile: String,

    /// Native critic rule IDs to include. Empty means use the selected profile.
    pub native_critic_include: Vec<String>,

    /// Native critic rule IDs to exclude from the selected profile.
    pub native_critic_exclude: Vec<String>,

    /// Whether LSP formatting is enabled.
    ///
    /// Kept as `perltidy_enabled` for compatibility with older internal call sites
    /// and configuration names; the actual engine is selected by `formatting_engine`.
    pub perltidy_enabled: bool,

    /// Formatter engine used for LSP formatting requests.
    pub formatting_engine: FormatterMode,

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
            perlcritic_enabled: true,
            perlcritic_severity: 3,
            perlcritic_profile: None,
            perlcritic_theme: None,
            critic_engine: CriticEngine::Native,
            native_critic_profile: "recommended".to_string(),
            native_critic_include: Vec::new(),
            native_critic_exclude: Vec::new(),
            perltidy_enabled: true,
            formatting_engine: FormatterMode::Native,
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
            if let Some(theme) = critic.get("theme").and_then(|v| v.as_str()) {
                let theme = theme.trim();
                self.perlcritic_theme = (!theme.is_empty()).then(|| theme.to_string());
            }
        }

        if let Some(critic) = settings.get("critic")
            && let Some(engine) = critic.get("engine").and_then(|v| v.as_str())
            && let Some(engine) = parse_critic_engine(engine)
        {
            self.critic_engine = engine;
        }
        if let Some(critic) = settings.get("critic")
            && let Some(profile) = critic.get("profile").and_then(|v| v.as_str())
            && let Some(profile) = parse_native_critic_profile(profile)
        {
            self.native_critic_profile = profile.to_string();
        }
        if let Some(critic) = settings.get("critic") {
            if let Some(include) = string_array(critic.get("include")) {
                self.native_critic_include = include;
            }
            if let Some(exclude) = string_array(critic.get("exclude")) {
                self.native_critic_exclude = exclude;
            }
        }

        if let Some(formatting) = settings.get("formatting") {
            if let Some(enabled) = formatting.get("enabled").and_then(|v| v.as_bool()) {
                self.perltidy_enabled = enabled;
            }
            if let Some(engine) = formatting.get("engine").and_then(|v| v.as_str())
                && let Some(mode) = parse_formatter_mode(engine)
            {
                self.formatting_engine = mode;
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

fn parse_formatter_mode(value: &str) -> Option<FormatterMode> {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "native" => Some(FormatterMode::Native),
        "compat" | "perltidy-compat" => Some(FormatterMode::Compat),
        "external-legacy" | "external-perltidy" | "perltidy" => Some(FormatterMode::ExternalLegacy),
        "off" | "disabled" | "none" => Some(FormatterMode::Off),
        _ => None,
    }
}

fn parse_critic_engine(value: &str) -> Option<CriticEngine> {
    match value.trim().to_ascii_lowercase().as_str() {
        "legacy" | "external" | "perlcritic" => Some(CriticEngine::Legacy),
        "native" => Some(CriticEngine::Native),
        _ => None,
    }
}

fn parse_native_critic_profile(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "recommended" => Some("recommended"),
        "strict" => Some("strict"),
        _ => None,
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

fn normalize_include_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = Path::new(trimmed).components().fold(PathBuf::new(), |mut acc, comp| {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::RootDir
            | std::path::Component::Prefix(_)
            | std::path::Component::ParentDir
            | std::path::Component::Normal(_) => acc.push(comp.as_os_str()),
        }
        acc
    });

    if normalized.as_os_str().is_empty() {
        return Some(".".to_string());
    }

    Some(normalized.to_string_lossy().into_owned())
}

fn dedupe_preserve_order<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut result = Vec::new();
    for path in paths {
        let Some(normalized) = normalize_include_path(path) else {
            continue;
        };
        if !result.iter().any(|existing| existing == &normalized) {
            result.push(normalized);
        }
    }
    result
}

fn normalize_string_list(values: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || result.iter().any(|existing| existing == trimmed) {
            continue;
        }
        result.push(trimmed.to_string());
    }
    result
}

fn string_array(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value.and_then(|value| value.as_array()).map(|values| {
        normalize_string_list(
            &values
                .iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>(),
        )
    })
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
        dedupe_preserve_order(value.split(SEP))
    }

    /// Return the effective module-search-path, merging `PERL5LIB` paths with
    /// `self.include_paths` according to `self.perl5lib_precedence`.
    ///
    /// If `self.use_perl5lib` is `false`, or `perl5lib_paths` is empty, the
    /// returned list contains only `self.include_paths` entries (trimmed and deduplicated).
    pub fn effective_include_paths(&self, perl5lib_paths: &[String]) -> Vec<String> {
        if !self.use_perl5lib || perl5lib_paths.is_empty() {
            return dedupe_preserve_order(self.include_paths.iter().map(String::as_str));
        }
        match self.perl5lib_precedence {
            Perl5LibPrecedence::Prepend => dedupe_preserve_order(
                perl5lib_paths
                    .iter()
                    .map(String::as_str)
                    .chain(self.include_paths.iter().map(String::as_str)),
            ),
            Perl5LibPrecedence::Append => dedupe_preserve_order(
                self.include_paths
                    .iter()
                    .map(String::as_str)
                    .chain(perl5lib_paths.iter().map(String::as_str)),
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
            // Security: do NOT honour workspace-supplied perlPath / perlArgs.
            // Allowing arbitrary Perl interpreter / argv from workspace settings
            // would let a hostile project execute arbitrary code via the @INC
            // probe (issue #3729). The interpreter / args remain whatever the
            // user (not the workspace) configured globally.
            if let Some(timeout) = workspace.get("resolutionTimeout").and_then(|v| v.as_u64()) {
                self.resolution_timeout_ms = timeout;
            }
            if let Some(use_p5l) = workspace.get("usePerl5lib").and_then(|v| v.as_bool()) {
                // Invalidate the lazy startup-@INC cache when usePerl5lib toggles, because
                // `fetch_perl_inc` strips PERL5LIB from the probe environment when
                // usePerl5lib is false (and inherits it when true). A cache built under
                // the old setting may include or exclude PERL5LIB paths incorrectly.
                if use_p5l != self.use_perl5lib {
                    self.system_inc_cache = None;
                }
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
    ///
    /// The probe (`perl -e 'print join("\n", @INC)'`) is bounded by
    /// `SYSTEM_INC_PROBE_TIMEOUT`. If it times out — common when the
    /// interpreter is on a slow filesystem, hangs, or is a perlbrew shim
    /// that takes a long time on first run — an empty vector is cached
    /// so subsequent requests don't re-probe. The user can re-trigger
    /// probing by toggling `useSystemInc`, which invalidates the cache.
    ///
    /// The PERL5LIB environment variable is stripped from the probe subprocess
    /// when `use_perl5lib` is false, so interpreter startup `@INC` does not
    /// silently reintroduce PERL5LIB paths that the user has disabled. The two
    /// settings remain independent: PERL5LIB visibility is controlled by
    /// `use_perl5lib`, and startup `@INC` (everything except PERL5LIB) is
    /// controlled by `use_system_inc`.
    pub fn get_system_inc(&mut self) -> &[PathBuf] {
        if !self.use_system_inc {
            return &[];
        }

        if self.system_inc_cache.is_none() {
            // Snapshot the fields needed by the oracle constructor before the
            // mutable borrow below.
            let perl_args = self.perl_args.clone();
            let result = Self::fetch_perl_inc(self, &perl_args);
            self.system_inc_cache = Some(result);
        }

        self.system_inc_cache.as_deref().unwrap_or(&[])
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fetch_perl_inc(config: &WorkspaceConfig, perl_args: &[String]) -> Vec<PathBuf> {
        let oracle = match PerlOracleEnv::for_module_resolution(config) {
            Some(o) => o,
            None => return Vec::new(),
        };
        let timeout = oracle.timeout;
        let mut command = oracle.into_command();
        command.args(perl_args);
        command.args(["-e", "print join(\"\\n\", @INC)"]);
        let output = output_with_timeout(command, timeout);

        match output {
            Ok(out) if out.status.success() => {
                Self::parse_perl_inc_output(&String::from_utf8_lossy(&out.stdout))
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::warn!(
                    target: "perl_lsp::config::system_inc",
                    status = ?out.status,
                    stderr = %stderr.trim(),
                    "startup @INC probe exited non-zero; caching empty result"
                );
                Vec::new()
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {
                tracing::warn!(
                    target: "perl_lsp::config::system_inc",
                    timeout_ms = timeout.as_millis() as u64,
                    "startup @INC probe timed out; caching empty result. \
                     Set perl.workspace.useSystemInc=false to disable probing, \
                     or pin a faster perl interpreter."
                );
                Vec::new()
            }
            Err(err) => {
                tracing::warn!(
                    target: "perl_lsp::config::system_inc",
                    error = %err,
                    "startup @INC probe failed to spawn perl; caching empty result"
                );
                Vec::new()
            }
        }
    }

    fn parse_perl_inc_output(stdout: &str) -> Vec<PathBuf> {
        stdout
            .lines()
            .map(str::trim)
            .filter(|line| {
                !line.is_empty()
                    && *line != "."
                    && !line.starts_with("CODE(")
                    && !line.starts_with("ARRAY(")
                    && !line.starts_with("HASH(")
                    && !line.starts_with("SCALAR(")
                    && !line.starts_with("REF(")
                    && !line.starts_with("GLOB(")
                    && !line.starts_with("IO::")
            })
            .map(PathBuf::from)
            .fold(Vec::new(), |mut acc, path| {
                if !acc.contains(&path) {
                    acc.push(path);
                }
                acc
            })
    }

    #[cfg(target_arch = "wasm32")]
    fn fetch_perl_inc(_config: &WorkspaceConfig, _perl_args: &[String]) -> Vec<PathBuf> {
        Vec::new()
    }
}

/// Bounded interpreter startup `@INC` probe.
///
/// The probe is intentionally a separate constant from
/// `WorkspaceConfig::resolution_timeout_ms` (50 ms default). 50 ms is well
/// under Perl interpreter startup on most platforms — a perlbrew shim,
/// remote filesystem, or even a cold cache can comfortably exceed it.
/// 1000 ms is short enough that a stalled probe does not noticeably block
/// the LSP and long enough that healthy probes succeed reliably.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SYSTEM_INC_PROBE_TIMEOUT: Duration = Duration::from_secs(1);

/// Run `command` with a wall-clock timeout, killing the child if it exceeds
/// `timeout`. Returns `io::Error` with kind `TimedOut` on timeout. Used by
/// `fetch_perl_inc` so a hanging or slow `perl` interpreter cannot stall
/// the LSP indefinitely.
///
/// Polls `try_wait` every 20 ms. The 20 ms granularity is acceptable for the
/// startup-`@INC` probe — total overhead at the bound is at most one extra
/// poll tick.
#[cfg(not(target_arch = "wasm32"))]
fn output_with_timeout(mut command: Command, timeout: Duration) -> std::io::Result<Output> {
    let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let start = Instant::now();
    let poll_interval = Duration::from_millis(20);

    loop {
        match child.try_wait()? {
            Some(_) => return child.wait_with_output(),
            None => {
                if start.elapsed() >= timeout {
                    // Best-effort kill; ignore errors from already-exited processes.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("subprocess exceeded {timeout:?}"),
                    ));
                }
                std::thread::sleep(poll_interval);
            }
        }
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
    /// `[formatting]` section: native formatter and legacy adapter configuration.
    pub formatting: ProjectFormattingConfig,
    /// `[critic]` section: native critic and legacy adapter configuration.
    pub critic: ProjectCriticConfig,
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

/// `[critic]` section of `.perl-lsp.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ProjectCriticConfig {
    /// Critic engine (`legacy`, `perlcritic`, or `native`).
    pub engine: Option<String>,
    /// Native critic profile (`recommended` or `strict`).
    pub profile: Option<String>,
    /// Native critic rule IDs to include. Empty means use the selected profile.
    pub include: Option<Vec<String>>,
    /// Native critic rule IDs to exclude from the selected profile.
    pub exclude: Option<Vec<String>>,
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
    /// Whether LSP formatting is enabled.
    pub enabled: Option<bool>,
    /// Formatter engine (`native`, `compat`, `external-perltidy`, or `off`).
    pub engine: Option<String>,
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
/// Returns `Err` on TOML parse failure, I/O errors, oversized files, or non-regular paths;
/// caller should emit a `window/showMessage` warning and continue with defaults.
pub fn load_project_config(
    workspace_root: &std::path::Path,
) -> Result<Option<ProjectConfig>, String> {
    const MAX_PROJECT_CONFIG_BYTES: u64 = 1024 * 1024; // 1 MiB

    let path = workspace_root.join(".perl-lsp.toml");
    let metadata = match std::fs::metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Could not read .perl-lsp.toml: {}. \
                 Check that the file is readable and not locked by another process.",
                e
            ));
        }
        Ok(metadata) => metadata,
    };

    if !metadata.file_type().is_file() {
        return Err(
            "Could not read .perl-lsp.toml: path must be a regular file (not a directory, pipe, \
             or device)."
                .to_string(),
        );
    }

    if metadata.len() > MAX_PROJECT_CONFIG_BYTES {
        return Err(format!(
            "Could not read .perl-lsp.toml: file is too large ({} bytes, max {} bytes).",
            metadata.len(),
            MAX_PROJECT_CONFIG_BYTES
        ));
    }

    // Open the file; guard against a TOCTOU race where the file is removed after
    // the metadata check succeeds — treat a vanished file the same as not-found.
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Could not read .perl-lsp.toml: {}. \
                 Check that the file is readable and not locked by another process.",
                e
            ));
        }
    };
    let mut content = String::new();
    file.take(MAX_PROJECT_CONFIG_BYTES + 1)
        .read_to_string(&mut content)
        .map_err(|e| format!("Could not read .perl-lsp.toml: {}", e))?;

    if content.len() as u64 > MAX_PROJECT_CONFIG_BYTES {
        return Err(format!(
            "Could not read .perl-lsp.toml: file is too large ({} bytes, max {} bytes). \
             The file may have grown between the size check and the read.",
            content.len(),
            MAX_PROJECT_CONFIG_BYTES
        ));
    }

    toml::from_str::<ProjectConfig>(&content)
        .map(Some)
        .map_err(|e| format!(".perl-lsp.toml has a syntax error: {}", e))
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
        if let Some(ref engine) = self.formatting.engine
            && let Some(mode) = parse_formatter_mode(engine)
        {
            config.formatting_engine = mode;
        }
        if let Some(ref engine) = self.critic.engine
            && let Some(engine) = parse_critic_engine(engine)
        {
            config.critic_engine = engine;
        }
        if let Some(ref profile) = self.critic.profile
            && let Some(profile) = parse_native_critic_profile(profile)
        {
            config.native_critic_profile = profile.to_string();
        }
        if let Some(ref include) = self.critic.include {
            config.native_critic_include = normalize_string_list(include);
        }
        if let Some(ref exclude) = self.critic.exclude {
            config.native_critic_exclude = normalize_string_list(exclude);
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
engine = "external-perltidy"
perltidy_maximum_line_length = 100
perltidy_opening_brace_on_new_line = true
perltidy_cuddled_else = false
perltidy_space_after_keyword = false
perltidy_add_trailing_commas = true
perltidy_extra_args = ["-noll"]

[critic]
engine = "native"
profile = "recommended"
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
        assert_eq!(config.formatting.engine.as_deref(), Some("external-perltidy"));
        assert_eq!(config.formatting.perltidy_maximum_line_length, Some(100));
        assert_eq!(config.formatting.perltidy_opening_brace_on_new_line, Some(true));
        assert_eq!(config.formatting.perltidy_cuddled_else, Some(false));
        assert_eq!(config.formatting.perltidy_space_after_keyword, Some(false));
        assert_eq!(config.formatting.perltidy_add_trailing_commas, Some(true));
        assert_eq!(config.formatting.perltidy_extra_args, vec!["-noll"]);
        assert_eq!(config.critic.engine.as_deref(), Some("native"));
        assert_eq!(config.critic.profile.as_deref(), Some("recommended"));
        Ok(())
    }

    #[test]
    fn load_project_config_rejects_non_regular_file() -> TestResult {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir(temp.path().join(".perl-lsp.toml"))?;

        let err = load_project_config(temp.path())
            .err()
            .ok_or("expected non-regular config path to return an error")?;
        assert!(err.contains("regular file"));
        Ok(())
    }

    #[test]
    fn load_project_config_rejects_oversized_file() -> TestResult {
        let temp = tempfile::tempdir()?;
        let oversized = vec![b'a'; (1024 * 1024) + 1];
        std::fs::write(temp.path().join(".perl-lsp.toml"), oversized)?;

        let err = load_project_config(temp.path())
            .err()
            .ok_or("expected oversized config file to return an error")?;
        assert!(err.contains("too large"));
        Ok(())
    }

    /// A 1 MiB file (exactly at the cap) must be accepted without error.
    #[test]
    fn load_project_config_accepts_file_at_size_limit() -> TestResult {
        let temp = tempfile::tempdir()?;
        // Write a 1 MiB file containing only '#' comment chars — valid TOML, empty config.
        let exactly_at_limit = vec![b'#'; 1024 * 1024];
        std::fs::write(temp.path().join(".perl-lsp.toml"), exactly_at_limit)?;
        // Should parse successfully and yield a default ProjectConfig (no sections set).
        let config = load_project_config(temp.path())?;
        assert!(config.is_some(), "1 MiB file at the limit must be accepted");
        Ok(())
    }

    /// An error from File::open when the file is not found (TOCTOU: file removed after metadata
    /// check) must be treated as absent, not as a hard error.
    #[test]
    fn load_project_config_returns_none_for_vanished_file() -> TestResult {
        // We can't easily reproduce a true TOCTOU race, but we can verify the code path by
        // directly exercising the condition: call load_project_config on a path where no file
        // exists (metadata returns NotFound at the first check, so this also confirms the
        // original not-found path still works under the restructured code).
        let temp = tempfile::tempdir()?;
        let result = load_project_config(temp.path())?;
        assert!(result.is_none(), "missing file must yield None, not Err");
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
    fn server_config_update_from_value_applies_lsp_settings() -> TestResult {
        let mut config = ServerConfig::default();

        config.update_from_value(&serde_json::json!({
            "inlayHints": {
                "enabled": false,
                "parameterHints": false,
                "typeHints": false,
                "chainedHints": true,
                "maxLength": 12
            },
            "testRunner": {
                "enabled": false,
                "command": "prove",
                "args": ["-lv", 42, "t/unit.t"],
                "timeout": 12_345
            },
            "telemetry": {
                "enabled": true
            },
            "perlcritic": {
                "enabled": false,
                "severity": 99,
                "profile": "  .perlcriticrc  ",
                "theme": "  core && !pbp  "
            }
        }));

        assert!(!config.inlay_hints_enabled);
        assert!(!config.inlay_hints_parameter_hints);
        assert!(!config.inlay_hints_type_hints);
        assert!(config.inlay_hints_chained_hints);
        assert_eq!(config.inlay_hints_max_length, 12);
        assert!(!config.test_runner_enabled);
        assert_eq!(config.test_runner_command, "prove");
        assert_eq!(config.test_runner_args, vec!["-lv".to_string(), "t/unit.t".to_string()]);
        assert_eq!(config.test_runner_timeout, 12_345);
        assert!(config.telemetry_enabled);
        assert!(!config.perlcritic_enabled);
        assert_eq!(config.perlcritic_severity, 5);
        assert_eq!(config.perlcritic_profile.as_deref(), Some(".perlcriticrc"));
        assert_eq!(config.perlcritic_theme.as_deref(), Some("core && !pbp"));

        config.update_from_value(&serde_json::json!({
            "perlcritic": {
                "severity": 0,
                "profile": "   ",
                "theme": ""
            }
        }));

        assert_eq!(config.perlcritic_severity, 1);
        assert!(config.perlcritic_profile.is_none());
        assert!(config.perlcritic_theme.is_none());
        Ok(())
    }

    #[test]
    fn server_config_update_from_value_applies_formatting_and_ai_settings() -> TestResult {
        let mut config = ServerConfig::default();

        config.update_from_value(&serde_json::json!({
            "formatting": {
                "enabled": false,
                "engine": "perltidy_compat",
                "profile": "  .perltidyrc  ",
                "maximumLineLength": 120,
                "indentColumns": 2,
                "tabs": true,
                "openingBraceOnNewLine": true,
                "cuddledElse": false,
                "spaceAfterKeyword": false,
                "addTrailingCommas": true,
                "verticalAlignment": false,
                "blockCommentIndentation": 1,
                "extraArgs": ["-noll", false, "-bar"],
                "timeoutSecs": 7
            },
            "aiCompletion": {
                "enabled": true,
                "provider": "local",
                "endpoint": "http://127.0.0.1:11434/v1",
                "model": "codellama",
                "apiKeyEnv": "LOCAL_AI_KEY",
                "timeoutMs": 2500,
                "maxOutputTokens": 128,
                "rateLimitRps": 2.5,
                "maxInflight": 3,
                "fallback": false,
                "streaming": {
                    "enabled": false,
                    "updateDebounceMs": 125
                }
            }
        }));

        assert!(!config.perltidy_enabled);
        assert_eq!(config.formatting_engine, FormatterMode::Compat);
        assert_eq!(config.perltidy_profile.as_deref(), Some(".perltidyrc"));
        assert_eq!(config.perltidy_maximum_line_length, Some(120));
        assert_eq!(config.perltidy_indent_columns, Some(2));
        assert_eq!(config.perltidy_tabs, Some(true));
        assert_eq!(config.perltidy_opening_brace_on_new_line, Some(true));
        assert_eq!(config.perltidy_cuddled_else, Some(false));
        assert_eq!(config.perltidy_space_after_keyword, Some(false));
        assert_eq!(config.perltidy_add_trailing_commas, Some(true));
        assert_eq!(config.perltidy_vertical_alignment, Some(false));
        assert_eq!(config.perltidy_block_comment_indentation, Some(1));
        assert_eq!(config.perltidy_extra_args, vec!["-noll".to_string(), "-bar".to_string()]);
        assert_eq!(config.perltidy_timeout_secs, 7);
        assert!(config.ai_completion.enabled);
        assert_eq!(config.ai_completion.provider, "local");
        assert_eq!(config.ai_completion.endpoint, "http://127.0.0.1:11434/v1");
        assert_eq!(config.ai_completion.model, "codellama");
        assert_eq!(config.ai_completion.api_key_env, "LOCAL_AI_KEY");
        assert_eq!(config.ai_completion.timeout_ms, 2500);
        assert_eq!(config.ai_completion.max_output_tokens, 128);
        assert_eq!(config.ai_completion.rate_limit_rps, 2.5);
        assert_eq!(config.ai_completion.max_inflight, 3);
        assert!(!config.ai_completion.fallback);
        assert!(!config.ai_completion.streaming.enabled);
        assert_eq!(config.ai_completion.streaming.update_debounce_ms, 125);

        config.update_from_value(&serde_json::json!({
            "formatting": {
                "profile": ""
            }
        }));
        assert!(config.perltidy_profile.is_none());
        Ok(())
    }

    #[test]
    fn server_config_accepts_formatter_engine_aliases() {
        let mut config = ServerConfig::default();

        config.update_from_value(&serde_json::json!({
            "formatting": {
                "engine": "external-perltidy"
            }
        }));
        assert_eq!(config.formatting_engine, FormatterMode::ExternalLegacy);

        config.update_from_value(&serde_json::json!({
            "formatting": {
                "engine": "native"
            }
        }));
        assert_eq!(config.formatting_engine, FormatterMode::Native);
    }

    #[test]
    fn server_config_accepts_native_critic_engine() {
        let mut config = ServerConfig::default();
        assert_eq!(config.critic_engine, CriticEngine::Native);
        assert_eq!(config.native_critic_profile, "recommended");

        config.update_from_value(&serde_json::json!({
            "critic": {
                "engine": "native",
                "profile": "recommended"
            }
        }));
        assert_eq!(config.critic_engine, CriticEngine::Native);
        assert_eq!(config.native_critic_profile, "recommended");

        config.update_from_value(&serde_json::json!({
            "critic": {
                "engine": "perlcritic",
                "profile": "strict"
            }
        }));
        assert_eq!(config.critic_engine, CriticEngine::Legacy);
        assert_eq!(config.native_critic_profile, "strict");

        config.update_from_value(&serde_json::json!({
            "critic": {
                "profile": "unknown"
            }
        }));
        assert_eq!(config.native_critic_profile, "strict");
    }

    #[test]
    fn server_config_accepts_native_critic_include_and_exclude_filters() {
        let mut config = ServerConfig::default();

        config.update_from_value(&serde_json::json!({
            "critic": {
                "include": [
                    "native.testing.require_use_strict",
                    "",
                    "native.testing.require_use_warnings"
                ],
                "exclude": [
                    "native.common.assignment_in_condition"
                ]
            }
        }));

        assert_eq!(
            config.native_critic_include,
            vec![
                "native.testing.require_use_strict".to_string(),
                "native.testing.require_use_warnings".to_string()
            ]
        );
        assert_eq!(
            config.native_critic_exclude,
            vec!["native.common.assignment_in_condition".to_string()]
        );

        config.update_from_value(&serde_json::json!({
            "critic": {
                "include": []
            }
        }));
        assert!(config.native_critic_include.is_empty());
        assert_eq!(
            config.native_critic_exclude,
            vec!["native.common.assignment_in_condition".to_string()]
        );
    }

    #[test]
    fn project_config_applies_formatter_engine() {
        let mut config = ServerConfig::default();
        let mut project = ProjectConfig::default();
        project.formatting.engine = Some("off".to_string());

        project.apply_to_server_config(&mut config);

        assert_eq!(config.formatting_engine, FormatterMode::Off);
    }

    #[test]
    fn project_config_applies_native_formatting_policy_fields() {
        let mut config = ServerConfig::default();
        let mut project = ProjectConfig::default();
        project.formatting.engine = Some("native".to_string());
        project.formatting.perltidy_opening_brace_on_new_line = Some(true);
        project.formatting.perltidy_cuddled_else = Some(false);
        project.formatting.perltidy_space_after_keyword = Some(false);
        project.formatting.perltidy_add_trailing_commas = Some(true);

        project.apply_to_server_config(&mut config);

        assert_eq!(config.formatting_engine, FormatterMode::Native);
        assert_eq!(config.perltidy_opening_brace_on_new_line, Some(true));
        assert_eq!(config.perltidy_cuddled_else, Some(false));
        assert_eq!(config.perltidy_space_after_keyword, Some(false));
        assert_eq!(config.perltidy_add_trailing_commas, Some(true));
    }

    #[test]
    fn project_config_applies_native_critic_engine() {
        let mut config = ServerConfig::default();
        let mut project = ProjectConfig::default();
        project.critic.engine = Some("native".to_string());
        project.critic.profile = Some("recommended".to_string());
        project.critic.include = Some(vec![
            "native.testing.require_use_strict".to_string(),
            " ".to_string(),
            "native.testing.require_use_strict".to_string(),
        ]);
        project.critic.exclude = Some(vec!["native.common.assignment_in_condition".to_string()]);

        project.apply_to_server_config(&mut config);

        assert_eq!(config.critic_engine, CriticEngine::Native);
        assert_eq!(config.native_critic_profile, "recommended");
        assert_eq!(
            config.native_critic_include,
            vec!["native.testing.require_use_strict".to_string()]
        );
        assert_eq!(
            config.native_critic_exclude,
            vec!["native.common.assignment_in_condition".to_string()]
        );
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
        // normalize_include_path round-trips through PathBuf, which emits the
        // platform-native separator.  Gate the expected value accordingly.
        #[cfg(windows)]
        assert_eq!(parsed, vec!["lib", "local\\lib"]);
        #[cfg(not(windows))]
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

        // normalize_include_path uses PathBuf internally, so separators are platform-native.
        #[cfg(windows)]
        assert_eq!(paths, vec!["local\\lib", "vendor\\lib", "lib"]);
        #[cfg(not(windows))]
        assert_eq!(paths, vec!["local/lib", "vendor/lib", "lib"]);
    }

    /// Precedence audit: when the SAME path appears in both `PERL5LIB` and
    /// `include_paths`, `Prepend` resolves it from PERL5LIB's slot (early in
    /// the search list) and `Append` resolves it from `include_paths`' slot.
    ///
    /// This pins the policy decision flagged in the 2026-05-11 @INC
    /// checkpoint: `perl5libPrecedence=prepend` is **Perl-faithful**
    /// (PERL5LIB shadows workspace `includePaths` for overlapping modules).
    /// `Append` makes workspace `includePaths` win over PERL5LIB. The
    /// resolver iterates in the returned order and picks the first hit, so
    /// the asserted index is the resolution outcome.
    #[test]
    fn perl5lib_precedence_pins_shadow_order_for_overlapping_paths() {
        let shared = "shared/lib".to_string();
        let perl5lib = vec![shared.clone()];
        let workspace = vec![shared.clone()];

        let prepend_config = WorkspaceConfig {
            include_paths: workspace.clone(),
            perl5lib_precedence: Perl5LibPrecedence::Prepend,
            ..WorkspaceConfig::default()
        };
        let prepend = prepend_config.effective_include_paths(&perl5lib);
        assert_eq!(prepend.len(), 1, "dedup must collapse to one entry");
        // With prepend, the overlapping path is contributed by PERL5LIB
        // first and the workspace copy is deduped. The kept entry is
        // conceptually the PERL5LIB one — which matches Perl's runtime
        // behaviour where `-I` / PERL5LIB precedes the default `@INC`.

        let append_config = WorkspaceConfig {
            include_paths: workspace,
            perl5lib_precedence: Perl5LibPrecedence::Append,
            ..WorkspaceConfig::default()
        };
        let append = append_config.effective_include_paths(&perl5lib);
        assert_eq!(append.len(), 1, "dedup must collapse to one entry");
        // Symmetric: with append, the workspace `includePaths` entry comes
        // first; the PERL5LIB duplicate is deduped after it.

        // Multi-path overlap: shared appears in both lists, plus distinct
        // entries on each side. Prepend interleaves PERL5LIB-first.
        let prepend_multi_config = WorkspaceConfig {
            include_paths: vec!["workspace_only".into(), shared.clone()],
            perl5lib_precedence: Perl5LibPrecedence::Prepend,
            ..WorkspaceConfig::default()
        };
        let prepend_multi =
            prepend_multi_config.effective_include_paths(&["env_only".into(), shared.clone()]);
        // PERL5LIB entries first → env_only then shared (from env). The
        // workspace's `shared` is deduped. workspace_only follows.
        #[cfg(windows)]
        assert_eq!(prepend_multi, vec!["env_only", "shared\\lib", "workspace_only"]);
        #[cfg(not(windows))]
        assert_eq!(prepend_multi, vec!["env_only", "shared/lib", "workspace_only"]);

        // And append flips it: workspace entries first, then env-only.
        let append_multi_config = WorkspaceConfig {
            include_paths: vec!["workspace_only".into(), shared.clone()],
            perl5lib_precedence: Perl5LibPrecedence::Append,
            ..WorkspaceConfig::default()
        };
        let append_multi =
            append_multi_config.effective_include_paths(&["env_only".into(), shared.clone()]);
        #[cfg(windows)]
        assert_eq!(append_multi, vec!["workspace_only", "shared\\lib", "env_only"]);
        #[cfg(not(windows))]
        assert_eq!(append_multi, vec!["workspace_only", "shared/lib", "env_only"]);
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

        // normalize_include_path uses PathBuf internally, so separators are platform-native.
        #[cfg(windows)]
        assert_eq!(paths, vec!["lib", "local\\lib", "vendor\\lib"]);
        #[cfg(not(windows))]
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
    fn parse_perl5lib_normalizes_dot_and_trailing_slash_entries() {
        #[cfg(windows)]
        let input = ".;./lib;lib\\;./lib\\";
        #[cfg(not(windows))]
        let input = ".:./lib:lib/:./lib/";

        let parsed = WorkspaceConfig::parse_perl5lib(input);
        assert_eq!(parsed, vec![".", "lib"]);
    }

    #[test]
    fn effective_include_paths_normalizes_equivalent_entries_before_dedupe() {
        let config = WorkspaceConfig {
            include_paths: vec!["./lib".to_string(), "lib/".to_string(), ".".to_string()],
            perl5lib_precedence: Perl5LibPrecedence::Prepend,
            ..WorkspaceConfig::default()
        };

        let paths = config.effective_include_paths(&["./lib/".to_string(), " ./ ".to_string()]);
        assert_eq!(paths, vec!["lib", "."]);
    }

    /// Security regression: workspace settings must not be able to redirect the
    /// Perl interpreter / argv used for the @INC probe (issue #3729). A hostile
    /// project could otherwise execute arbitrary code at config-load time.
    #[test]
    fn workspace_config_ignores_untrusted_perl_probe_settings() {
        let mut config = WorkspaceConfig::default();
        config.update_from_value(&serde_json::json!({
            "workspace": {
                "perlPath": "/opt/custom/perl",
                "perlArgs": ["-I", "/tmp/custom/lib"]
            }
        }));
        assert!(config.perl_path.is_none(), "perlPath from workspace must be ignored");
        assert!(config.perl_args.is_empty(), "perlArgs from workspace must be ignored");
    }
    #[test]
    fn parse_perl_inc_output_filters_dynamic_hook_entries() {
        let parsed = WorkspaceConfig::parse_perl_inc_output(
            "lib\nCODE(0x123)\nARRAY(0xabc)\nHASH(0xdef)\n/usr/lib/perl5\n",
        );
        assert_eq!(parsed, vec![PathBuf::from("lib"), PathBuf::from("/usr/lib/perl5")]);
    }

    #[test]
    fn parse_perl_inc_output_dedupes_and_drops_dot() {
        let parsed =
            WorkspaceConfig::parse_perl_inc_output("lib\n.\nlib\n/usr/lib/perl5\n/usr/lib/perl5\n");
        assert_eq!(parsed, vec![PathBuf::from("lib"), PathBuf::from("/usr/lib/perl5")]);
    }

    /// Regression guard for the unbounded `Command::output()` stall: a long-running
    /// probe must be killed within roughly `timeout + poll_interval`. We use perl
    /// itself (via the same toolchain resolver that `fetch_perl_inc` uses) to
    /// guarantee an interpreter is present; the test skips when no perl is
    /// available.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn output_with_timeout_kills_long_running_subprocess() -> TestResult {
        let perl_path = match resolve_perl_path_with_toolchain() {
            Ok(path) => path,
            Err(_) => return Ok(()),
        };

        let mut command = Command::new(perl_path);
        command.args(["-e", "sleep 10; print 'should not reach'"]);

        let start = Instant::now();
        let result = output_with_timeout(command, Duration::from_millis(250));
        let elapsed = start.elapsed();

        let err = match result {
            Ok(out) => return Err(format!("expected timeout, got success: {out:?}").into()),
            Err(e) => e,
        };
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::TimedOut,
            "expected ErrorKind::TimedOut, got {err:?}",
        );
        // Allow generous overhead (slow CI cold start, antivirus, etc.).
        assert!(
            elapsed < Duration::from_secs(3),
            "timeout should fire within reasonable overhead, took {elapsed:?}",
        );
        Ok(())
    }

    /// `get_system_inc` must respect `SYSTEM_INC_PROBE_TIMEOUT` so a hung
    /// interpreter cannot block the LSP request thread. Verifies the full
    /// path: `use_system_inc=true`, slow `perl_path`, lazy probe times out,
    /// returned slice is empty, cache holds the empty result.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn get_system_inc_does_not_stall_on_slow_interpreter() -> TestResult {
        let perl_path = match resolve_perl_path_with_toolchain() {
            Ok(path) => path,
            Err(_) => return Ok(()),
        };

        let mut config = WorkspaceConfig::default();
        config.use_system_inc = true;
        config.perl_path = Some(perl_path.to_string_lossy().into_owned());
        // perl_args runs BEFORE -e 'print @INC', so we make perl sleep up front.
        // The sleep is much longer than SYSTEM_INC_PROBE_TIMEOUT (1s).
        config.perl_args = vec!["-e".into(), "sleep 10".into()];

        let start = Instant::now();
        let paths = config.get_system_inc().to_vec();
        let elapsed = start.elapsed();

        assert!(paths.is_empty(), "expected empty @INC on timeout, got {paths:?}");
        // Generous bound: SYSTEM_INC_PROBE_TIMEOUT (1s) + spawn + poll overhead.
        assert!(
            elapsed < Duration::from_secs(4),
            "get_system_inc must return within timeout+overhead, took {elapsed:?}",
        );

        // Cached empty result — second call does not respawn perl.
        let start2 = Instant::now();
        let paths2 = config.get_system_inc().to_vec();
        let elapsed2 = start2.elapsed();
        assert!(paths2.is_empty());
        assert!(
            elapsed2 < Duration::from_millis(50),
            "cached lookup should be fast, took {elapsed2:?}",
        );
        Ok(())
    }

    /// `usePerl5lib` and `useSystemInc` must produce independent startup-`@INC`
    /// caches. When `usePerl5lib` toggles, the cache must be invalidated so the
    /// next `get_system_inc` call re-probes Perl with the correct PERL5LIB
    /// environment (stripped or inherited based on `use_perl5lib`).
    #[test]
    fn use_perl5lib_toggle_invalidates_system_inc_cache() {
        let mut config = WorkspaceConfig::default();
        config.use_system_inc = true;
        assert!(config.use_perl5lib, "default usePerl5lib should be true");

        // Pre-populate the cache; flipping usePerl5lib must clear it.
        config.system_inc_cache = Some(vec![PathBuf::from("/sentinel/cached")]);
        config.update_from_value(&serde_json::json!({
            "workspace": { "usePerl5lib": false }
        }));
        assert!(!config.use_perl5lib);
        assert!(
            config.system_inc_cache.is_none(),
            "system_inc_cache must invalidate when usePerl5lib changes (true -> false); \
             got {:?}",
            config.system_inc_cache
        );

        // Flip back the other direction.
        config.system_inc_cache = Some(vec![PathBuf::from("/sentinel/cached2")]);
        config.update_from_value(&serde_json::json!({
            "workspace": { "usePerl5lib": true }
        }));
        assert!(config.use_perl5lib);
        assert!(
            config.system_inc_cache.is_none(),
            "system_inc_cache must invalidate when usePerl5lib changes (false -> true); \
             got {:?}",
            config.system_inc_cache
        );

        // No-op (same value) must NOT clear the cache.
        let stable = vec![PathBuf::from("/sentinel/stable")];
        config.system_inc_cache = Some(stable.clone());
        config.update_from_value(&serde_json::json!({
            "workspace": { "usePerl5lib": true }
        }));
        assert_eq!(
            config.system_inc_cache.as_deref(),
            Some(stable.as_slice()),
            "cache must survive when usePerl5lib value does not change",
        );
    }
}
