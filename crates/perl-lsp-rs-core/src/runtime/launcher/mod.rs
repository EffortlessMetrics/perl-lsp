#![warn(missing_docs)]
//! CLI and startup configuration primitives for the Perl LSP binary.
//!
//! This crate extracts the runtime launch decision surface into a dedicated crate so
//! feature profiles, transport mode semantics, and BDD-grid interoperability stay in one
//! place and remain stable across binaries.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::io;
use std::io::IsTerminal;
use std::sync::{Once, OnceLock};

use clap::{Args, Parser};
pub mod timing;
pub use crate::features::contracts::trackable_feature_count_for_grid;
pub use crate::features::grid::{compliance_percent_for_profile, to_json_for_profile};
pub use crate::features::policy::{FeatureProfile, catalog_advertised_feature_ids};
use crate::features::profile_cli::{feature_profile_supported_tokens, parse_feature_profile_arg};
use crate::runtime::tuning::{DiagnosticMode, RuntimeMode, RuntimeTuning};
pub use timing::{StartupReport, StartupTimer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt as tracing_fmt};

static LOGGING_INIT: Once = Once::new();
/// Keeps the non-blocking file writer alive for the process lifetime.
static LOG_FILE_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Default port used by socket transport.
pub const DEFAULT_LSP_PORT: u16 = 9257;

/// Returns whether runtime logging should be enabled.
///
/// Logging activates when the CLI explicitly requests it or when
/// `PERL_LSP_LOG`/`RUST_LOG` is already set in the environment, which keeps
/// environment-driven tracing behavior consistent with the historical Perl LSP
/// binary contract.
pub fn should_enable_logging(explicit_flag: bool) -> bool {
    explicit_flag || logging_env_directive().is_some()
}

fn logging_env_directive() -> Option<String> {
    std::env::var("PERL_LSP_LOG").ok().or_else(|| std::env::var("RUST_LOG").ok())
}

/// Resolve the effective tracing filter for the current process.
///
/// Environment overrides take precedence; otherwise the returned filter uses
/// `explicit_default_filter` when logging was requested explicitly and
/// `implicit_default_filter` when logging is only enabled via default behavior.
pub fn logging_filter(
    explicit_flag: bool,
    explicit_default_filter: &str,
    implicit_default_filter: &str,
) -> String {
    logging_env_directive().unwrap_or_else(|| {
        if explicit_flag {
            explicit_default_filter.to_string()
        } else {
            implicit_default_filter.to_string()
        }
    })
}

/// Initialize tracing once for the current process.
///
/// When `PERL_LSP_LOG_FILE` is set, logs are written to a daily-rotated file
/// (max 5 files) **in addition to** stderr. Invalid `RUST_LOG` values fall
/// back to `default_filter`.
pub fn init_logging(default_filter: &str) {
    LOGGING_INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(default_filter))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        let use_ansi = should_use_ansi_stderr();

        // If PERL_LSP_LOG_FILE is set, add a rolling file appender alongside stderr.
        if let Ok(log_path) = std::env::var("PERL_LSP_LOG_FILE") {
            let path = std::path::Path::new(&log_path);
            let log_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let log_file_prefix = path.file_name().and_then(|f| f.to_str()).unwrap_or("perl-lsp");

            if let Ok(file_appender) = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix(log_file_prefix)
                .max_log_files(5)
                .build(log_dir)
            {
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                let _ = LOG_FILE_GUARD.set(guard);

                let stderr_layer = tracing_subscriber::fmt::layer()
                    .with_writer(io::stderr)
                    .with_ansi(use_ansi)
                    .with_target(true);

                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(true);

                let _ = tracing_subscriber::registry()
                    .with(filter)
                    .with(stderr_layer)
                    .with(file_layer)
                    .try_init();

                return;
            }
            // Fall through to stderr-only if file appender fails to build.
        }

        let _ = tracing_fmt()
            .with_env_filter(filter)
            .with_writer(io::stderr)
            .with_ansi(use_ansi)
            .with_target(true)
            .try_init();
    });
}

fn env_truthy(var_name: &str) -> Option<bool> {
    std::env::var(var_name).ok().map(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        !(normalized.is_empty() || normalized == "0" || normalized == "false" || normalized == "no")
    })
}

fn is_warp_terminal() -> bool {
    matches!(std::env::var("TERM_PROGRAM"), Ok(value) if value.eq_ignore_ascii_case("WarpTerminal"))
}

fn is_dumb_terminal() -> bool {
    matches!(std::env::var("TERM"), Ok(value) if value.eq_ignore_ascii_case("dumb"))
}

fn should_use_ansi(is_terminal: bool) -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    if matches!(env_truthy("FORCE_COLOR"), Some(true))
        || matches!(env_truthy("CLICOLOR_FORCE"), Some(true))
    {
        return true;
    }

    if matches!(env_truthy("CLICOLOR"), Some(false)) {
        return false;
    }

    if is_dumb_terminal() {
        return false;
    }

    is_terminal || is_warp_terminal()
}

/// Returns whether ANSI color should be used for stdout output.
pub fn should_use_ansi_stdout() -> bool {
    should_use_ansi(io::stdout().is_terminal())
}

/// Returns whether ANSI color should be used for stderr output.
pub fn should_use_ansi_stderr() -> bool {
    should_use_ansi(io::stderr().is_terminal())
}

/// Emit a consistent startup log line for server binaries.
///
/// When a `startup_report` is provided, phase-level timing is logged at `debug`
/// level and the total startup time at `info` level, enabling profiling without
/// adding noise to normal output.
pub fn log_server_startup(
    server_name: &str,
    version: &str,
    transport: TransportMode,
    feature_profile: Option<FeatureProfile>,
    startup_report: Option<&StartupReport>,
) {
    tracing::info!(server = server_name, version, transport = transport.label(), "server starting");

    if let Some(port) = transport.port() {
        tracing::info!(server = server_name, port, "listening port configured");
    }

    if let Some(profile) = feature_profile {
        let feature_count = catalog_advertised_feature_ids(profile).len();
        tracing::info!(
            server = server_name,
            feature_profile = profile.as_str(),
            features = feature_count,
            "feature profile active"
        );
    }

    if let Some(report) = startup_report {
        report.log();
    }
}

/// Transport options shared by server binaries.
#[derive(Args, Debug, Clone)]
pub struct TransportArgs {
    /// Use stdio for communication (default)
    #[arg(long, visible_alias = "mcp", default_value_t = false, conflicts_with = "socket")]
    pub stdio: bool,

    /// Use TCP socket for communication
    #[arg(long, conflicts_with = "stdio")]
    pub socket: bool,

    /// Port to listen on (for socket mode)
    #[arg(long)]
    pub port: Option<u16>,
}

impl TransportArgs {
    /// Returns the resolved transport mode.
    pub fn mode(&self) -> TransportMode {
        if self.socket || self.port.is_some() {
            TransportMode::Socket { port: self.port.unwrap_or(DEFAULT_LSP_PORT) }
        } else {
            TransportMode::Stdio
        }
    }
}

/// Command line arguments for the Perl LSP binary.
#[derive(Parser, Debug, Clone)]
#[command(name = "perl-lsp", version, about = "Perl Language Server", long_about = None)]
pub struct LspArgs {
    /// Transport configuration (stdio or socket).
    #[command(flatten)]
    pub transport: TransportArgs,

    /// Enable logging to stderr
    #[arg(long)]
    pub log: bool,

    /// Quick health check (prints 'ok `<version>`')
    #[arg(long)]
    pub health: bool,

    /// Show server info (version, features, coverage)
    #[arg(long)]
    pub info: bool,

    /// Validate Perl files and report parse errors (batch mode)
    #[arg(long)]
    pub check: bool,

    /// Scan a project directory and report parsability summary
    #[arg(long, conflicts_with = "check")]
    pub check_project: Option<Option<String>>,

    /// Generate shell completions (bash, zsh, fish, powershell, pwsh)
    #[arg(long)]
    pub completion: Option<String>,

    /// Output features catalog as JSON
    #[arg(long)]
    pub features_json: bool,

    /// Report how a .perltidyrc profile maps to native formatting
    #[arg(long, value_name = "PROFILE", conflicts_with = "perlcritic_compat_report")]
    pub perltidy_compat_report: Option<String>,

    /// Report how a .perlcriticrc profile maps to native critic rules
    #[arg(long, value_name = "PROFILE", conflicts_with = "perltidy_compat_report")]
    pub perlcritic_compat_report: Option<String>,

    /// Set feature profile
    #[arg(long)]
    pub feature_profile: Option<String>,

    /// Runtime workload mode: "normal" (default) or "e2e" (latency-focused harness).
    #[arg(long, value_name = "MODE")]
    pub runtime_mode: Option<String>,

    /// Diagnostic publication scope: "normal" or "syntax-only".
    #[arg(long, value_name = "MODE")]
    pub diagnostic_mode: Option<String>,

    /// Diagnostic publication debounce window in milliseconds (0 = immediate).
    #[arg(long, value_name = "MS")]
    pub diagnostic_debounce_ms: Option<u64>,

    /// Whether `initialized` triggers eager workspace indexing.
    #[arg(long, value_name = "BOOL")]
    pub eager_workspace_indexing: Option<bool>,

    /// Whether to register file watchers with the client.
    #[arg(long, value_name = "BOOL")]
    pub file_watchers: Option<bool>,

    /// Files to check (used with --check)
    #[arg(trailing_var_arg = true, requires = "check")]
    pub files: Vec<String>,
}

/// How the server should connect to the editor or test client.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TransportMode {
    /// Use stdio transport (JSON-RPC over stdin/stdout).
    Stdio,
    /// Use TCP socket transport.
    Socket {
        /// TCP port to bind.
        port: u16,
    },
}

impl TransportMode {
    /// Human-friendly label for logging.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Socket { .. } => "socket",
        }
    }

    /// TCP port used by the transport, if any.
    pub const fn port(self) -> Option<u16> {
        match self {
            Self::Stdio => None,
            Self::Socket { port } => Some(port),
        }
    }

    /// Returns true for TCP socket mode.
    pub const fn is_socket(self) -> bool {
        matches!(self, Self::Socket { .. })
    }
}

/// Runtime action selected by CLI parsing.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LaunchAction {
    /// Start a running server.
    Run,
    /// Print quick health status.
    Health,
    /// Show server info (version, features, coverage).
    Info,
    /// Validate Perl files in batch mode.
    Check,
    /// Scan a project directory and report parsability summary.
    CheckProject {
        /// Directory to scan (defaults to ".").
        dir: String,
    },
    /// Generate shell completions for a given shell.
    Completion {
        /// Target shell (bash, zsh, fish, powershell; pwsh aliases to powershell).
        shell: String,
    },
    /// Print version information.
    Version,
    /// Print profile-scoped feature catalog JSON.
    FeaturesJson,
    /// Classify a `.perltidyrc` profile against native formatter support.
    PerltidyCompatReport {
        /// Profile path to classify.
        profile: String,
    },
    /// Classify a `.perlcriticrc` profile against native critic support.
    PerlcriticCompatReport {
        /// Profile path to classify.
        profile: String,
    },
    /// Print CLI help output.
    Help,
}

/// Canonical launch configuration consumed by the server runtime.
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// Transport used by the server.
    pub transport: TransportMode,
    /// Whether to emit startup logs.
    pub enable_logging: bool,
    /// Effective feature profile selected by CLI/default policy.
    pub feature_profile: FeatureProfile,
    /// Runtime workload tuning (env + CLI layered over compiled defaults).
    pub runtime_tuning: RuntimeTuning,
}

impl LaunchConfig {
    /// Create a default launch configuration for a given feature profile.
    pub const fn new(feature_profile: FeatureProfile) -> Self {
        Self {
            transport: TransportMode::Stdio,
            enable_logging: false,
            feature_profile,
            runtime_tuning: RuntimeTuning::normal_defaults(),
        }
    }

    /// JSON payload describing profile-scoped advertised feature grid entries.
    pub fn features_json(&self) -> String {
        to_json_for_profile(self.feature_profile)
    }

    /// Feature IDs advertised for this profile under current catalog policy.
    pub fn advertised_feature_ids(&self) -> Vec<&'static str> {
        catalog_advertised_feature_ids(self.feature_profile)
    }
}

/// Fully resolved launch request.
#[derive(Debug, Clone)]
pub struct LaunchPlan {
    /// Requested runtime action.
    pub action: LaunchAction,
    /// Config to use when action is [`LaunchAction::Run`].
    pub config: LaunchConfig,
    /// Trailing file paths (used for `--check` mode).
    pub files: Vec<String>,
}

/// Parse-time errors emitted by the CLI parser.
#[derive(Debug, Clone)]
pub enum LaunchParseError {
    /// Unknown CLI token.
    UnknownOption {
        /// Unknown token passed on CLI.
        option: String,
    },
    /// A flag was missing its required value.
    MissingValue {
        /// Flag that needs a value.
        option: String,
    },
    /// Invalid profile token.
    InvalidFeatureProfile {
        /// Raw profile token from CLI.
        raw_profile: String,
    },
    /// Invalid TCP port value.
    InvalidPort {
        /// Raw port token from CLI.
        raw_port: String,
        /// Parse failure details.
        reason: String,
    },
    /// Invalid shell name for completions.
    InvalidShell {
        /// Raw shell token from CLI.
        raw_shell: String,
    },
    /// Invalid `--runtime-mode` token.
    InvalidRuntimeMode {
        /// Raw token from CLI.
        raw_mode: String,
    },
    /// Invalid `--diagnostic-mode` token.
    InvalidDiagnosticMode {
        /// Raw token from CLI.
        raw_mode: String,
    },
}

impl fmt::Display for LaunchParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption { option } => {
                write!(f, "Unknown option: {option}")
            }
            Self::MissingValue { option } => {
                write!(f, "Missing value for {option}")
            }
            Self::InvalidFeatureProfile { raw_profile } => {
                let supported = feature_profile_supported_tokens().join(", ");
                write!(f, "Invalid feature profile: {raw_profile}. Supported: {supported}")
            }
            Self::InvalidPort { raw_port, reason } => {
                write!(f, "Invalid port value: {raw_port}. {reason}")
            }
            Self::InvalidShell { raw_shell } => {
                write!(
                    f,
                    "Unknown shell: {raw_shell}. Supported: bash, zsh, fish, powershell, pwsh"
                )
            }
            Self::InvalidRuntimeMode { raw_mode } => {
                write!(f, "Invalid runtime mode: {raw_mode}. Supported: normal, e2e")
            }
            Self::InvalidDiagnosticMode { raw_mode } => {
                write!(f, "Invalid diagnostic mode: {raw_mode}. Supported: normal, syntax-only")
            }
        }
    }
}

impl Error for LaunchParseError {}

/// Parse command line arguments for the Perl LSP launcher.
pub fn parse_args<I>(args: I) -> Result<LaunchPlan, LaunchParseError>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString> + Clone,
{
    let collected_args: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    prevalidate_cli_values(&collected_args)?;

    match LspArgs::try_parse_from(collected_args) {
        Ok(parsed_args) => {
            let mut config = LaunchConfig::new(FeatureProfile::current());

            config.transport = parsed_args.transport.mode();
            config.enable_logging = parsed_args.log;

            if let Some(raw_profile) = parsed_args.feature_profile {
                config.feature_profile = parse_feature_profile(&raw_profile)?;
            }

            // Resolve runtime tuning: env layered over compiled defaults,
            // then CLI overrides on top.
            let cli_runtime_mode = match parsed_args.runtime_mode {
                Some(ref raw) => Some(RuntimeMode::parse(raw).ok_or_else(|| {
                    LaunchParseError::InvalidRuntimeMode { raw_mode: raw.clone() }
                })?),
                None => None,
            };
            let cli_diagnostic_mode = match parsed_args.diagnostic_mode {
                Some(ref raw) => Some(DiagnosticMode::parse(raw).ok_or_else(|| {
                    LaunchParseError::InvalidDiagnosticMode { raw_mode: raw.clone() }
                })?),
                None => None,
            };

            let mut runtime_tuning = RuntimeTuning::from_env();
            runtime_tuning.apply_cli_overrides(
                cli_runtime_mode,
                cli_diagnostic_mode,
                parsed_args.diagnostic_debounce_ms,
                parsed_args.eager_workspace_indexing,
                parsed_args.file_watchers,
            );
            config.runtime_tuning = runtime_tuning;

            let action = if parsed_args.health {
                LaunchAction::Health
            } else if parsed_args.info {
                LaunchAction::Info
            } else if parsed_args.check {
                LaunchAction::Check
            } else if let Some(maybe_dir) = parsed_args.check_project {
                let dir = maybe_dir.unwrap_or_else(|| ".".to_string());
                LaunchAction::CheckProject { dir }
            } else if let Some(raw_shell) = parsed_args.completion {
                let shell = normalize_completion_shell(&raw_shell).ok_or_else(|| {
                    LaunchParseError::InvalidShell { raw_shell: raw_shell.clone() }
                })?;
                LaunchAction::Completion { shell: shell.to_string() }
            } else if parsed_args.features_json {
                LaunchAction::FeaturesJson
            } else if let Some(profile) = parsed_args.perltidy_compat_report {
                LaunchAction::PerltidyCompatReport { profile }
            } else if let Some(profile) = parsed_args.perlcritic_compat_report {
                LaunchAction::PerlcriticCompatReport { profile }
            } else {
                LaunchAction::Run
            };

            Ok(LaunchPlan { action, config, files: parsed_args.files })
        }
        Err(err) => {
            let is_help = err.kind() == clap::error::ErrorKind::DisplayHelp
                || err.kind() == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand;
            let is_version = err.kind() == clap::error::ErrorKind::DisplayVersion;

            if is_help {
                return Ok(LaunchPlan {
                    action: LaunchAction::Help,
                    config: LaunchConfig::new(FeatureProfile::current()),
                    files: Vec::new(),
                });
            } else if is_version {
                return Ok(LaunchPlan {
                    action: LaunchAction::Version,
                    config: LaunchConfig::new(FeatureProfile::current()),
                    files: Vec::new(),
                });
            }

            Err(LaunchParseError::UnknownOption { option: err.to_string() })
        }
    }
}

fn prevalidate_cli_values(args: &[std::ffi::OsString]) -> Result<(), LaunchParseError> {
    let mut index = 1usize;

    while index < args.len() {
        let token = args[index].to_string_lossy();

        if token == "--port" {
            let next = args.get(index + 1).map(|value| value.to_string_lossy().to_string());
            let Some(raw_port) = next else {
                return Err(LaunchParseError::MissingValue { option: "--port".to_string() });
            };

            if raw_port.starts_with("--") {
                return Err(LaunchParseError::MissingValue { option: "--port".to_string() });
            }

            raw_port.parse::<u16>().map_err(|reason| LaunchParseError::InvalidPort {
                raw_port: raw_port.clone(),
                reason: reason.to_string(),
            })?;

            index += 2;
            continue;
        }

        if let Some(raw_port) = token.strip_prefix("--port=") {
            if raw_port.is_empty() {
                return Err(LaunchParseError::MissingValue { option: "--port".to_string() });
            }

            raw_port.parse::<u16>().map_err(|reason| LaunchParseError::InvalidPort {
                raw_port: raw_port.to_string(),
                reason: reason.to_string(),
            })?;
        }

        if token == "--completion" {
            let next = args.get(index + 1).map(|value| value.to_string_lossy().to_string());
            let Some(raw_shell) = next else {
                return Err(LaunchParseError::MissingValue { option: "--completion".to_string() });
            };

            if raw_shell.starts_with("--") {
                return Err(LaunchParseError::MissingValue { option: "--completion".to_string() });
            }

            if normalize_completion_shell(&raw_shell).is_none() {
                return Err(LaunchParseError::InvalidShell { raw_shell });
            }

            index += 2;
            continue;
        }

        if token == "--feature-profile" {
            let next = args.get(index + 1).map(|value| value.to_string_lossy().to_string());
            let Some(raw_profile) = next else {
                return Err(LaunchParseError::MissingValue {
                    option: "--feature-profile".to_string(),
                });
            };

            if raw_profile.starts_with("--") {
                return Err(LaunchParseError::MissingValue {
                    option: "--feature-profile".to_string(),
                });
            }

            index += 2;
            continue;
        }

        if token == "--feature-profile=" {
            return Err(LaunchParseError::MissingValue { option: "--feature-profile".to_string() });
        }

        index += 1;
    }

    Ok(())
}

/// Human-readable CLI help text shared by CLI consumers.
pub fn help_text() -> String {
    let supported_profiles = feature_profile_supported_tokens().join(", ");

    let mut out = String::with_capacity(1024);
    out.push_str("Perl Language Server\n");
    out.push('\n');
    out.push_str("Usage: perllsp [options]\n");
    out.push_str("       perllsp --check <file.pl> [file2.pm ...]\n");
    out.push_str("       perllsp --check-project [dir]\n");
    out.push('\n');
    out.push_str("Server options:\n");
    out.push_str("  --stdio, --mcp       Use stdio for communication (default)\n");
    out.push_str("  --socket             Use TCP socket for communication\n");
    out.push_str(&format!(
        "  --port <port>        Port to listen on (default: {DEFAULT_LSP_PORT})\n"
    ));
    out.push_str("  --log                Enable logging to stderr\n");
    out.push_str("  --feature-profile <name>\n");
    out.push_str(&format!("                       Set feature profile ({supported_profiles})\n"));
    out.push_str("  --runtime-mode <mode>\n");
    out.push_str("                       Runtime workload mode (normal, e2e)\n");
    out.push_str("  --diagnostic-mode <mode>\n");
    out.push_str("                       Diagnostic scope (normal, syntax-only)\n");
    out.push_str("  --diagnostic-debounce-ms <ms>\n");
    out.push_str("                       Diagnostic publish debounce window (0 = immediate)\n");
    out.push_str("  --eager-workspace-indexing <bool>\n");
    out.push_str("                       Index workspace on `initialized` (default: true)\n");
    out.push_str("  --file-watchers <bool>\n");
    out.push_str("                       Register file watchers with the client (default: true)\n");
    out.push('\n');
    out.push_str("Diagnostic options:\n");
    out.push_str("  --health             Quick health check (prints 'ok <version>')\n");
    out.push_str("  --info               Show version, features, and coverage info\n");
    out.push_str("  --version            Show version information\n");
    out.push_str("  --features-json      Output features catalog as JSON\n");
    out.push('\n');
    out.push_str("Tool options:\n");
    out.push_str("  --check <files...>   Validate Perl files and report parse errors\n");
    out.push_str("  --check-project [dir] Scan project directory for parsability report\n");
    out.push_str("  --perltidy-compat-report <profile>\n");
    out.push_str("                       Report native formatter compatibility for .perltidyrc\n");
    out.push_str("  --perlcritic-compat-report <profile>\n");
    out.push_str("                       Report native critic compatibility for .perlcriticrc\n");
    out.push_str(
        "  --completion <shell> Generate shell completions (bash, zsh, fish, powershell, pwsh)\n",
    );
    out.push_str("  --help               Show this help message\n");
    out.push('\n');
    out.push_str("Examples:\n");
    out.push_str("  perllsp --stdio                        # stdio mode (default)\n");
    out.push_str("  perllsp --mcp                          # stdio mode alias for MCP clients\n");
    out.push_str("  perllsp --stdio --log                   # with logging\n");
    out.push_str("  perllsp --socket --port 9257            # TCP socket mode\n");
    out.push_str("  perllsp --stdio --feature-profile=prod  # production profile\n");
    out.push_str("  perllsp --check lib/MyModule.pm         # syntax check\n");
    out.push_str("  perllsp --check-project lib/             # project scan\n");
    out.push_str("  perllsp --perltidy-compat-report .perltidyrc\n");
    out.push_str("  perllsp --perlcritic-compat-report .perlcriticrc\n");
    out.push_str("  perllsp --info                          # server information\n");
    out.push_str("  perllsp --completion bash >> ~/.bashrc   # install completions\n");
    out.push('\n');
    out.push_str("Environment:\n");
    out.push_str("  PERL_LSP_LOG=1       Enable logging (alternative to --log)\n");
    out.push_str("  PERL_LSP_LOG_FILE=<path>\n");
    out.push_str("                       Also log to a daily-rotated file (max 5 files)\n");
    out.push_str("  PERL_LSP_QUIET=1     Suppress the startup banner on stderr\n");
    out.push_str("  PERL_LSP_E2E=1       Enable e2e runtime mode (latency-focused harnesses)\n");
    out.push_str("  PERL_LSP_DIAGNOSTIC_MODE=<mode>\n");
    out.push_str("                       Override diagnostic scope (normal, syntax-only)\n");
    out.push_str("  PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS=<ms>\n");
    out.push_str("                       Override diagnostic debounce window\n");
    out.push_str("  RUST_LOG=<filter>    Set tracing filter (e.g. perl_lsp=debug)\n");
    out.push_str("  NO_COLOR=1           Disable colored output\n");
    out
}

/// Generate shell completion script for the given shell name.
///
/// Returns `None` for unknown shell names.
pub fn shell_completion(shell: &str) -> Option<&'static str> {
    match normalize_completion_shell(shell)? {
        "bash" => Some(BASH_COMPLETION),
        "zsh" => Some(ZSH_COMPLETION),
        "fish" => Some(FISH_COMPLETION),
        "powershell" => Some(POWERSHELL_COMPLETION),
        _ => None,
    }
}

fn normalize_completion_shell(shell: &str) -> Option<&'static str> {
    match shell.to_ascii_lowercase().as_str() {
        "bash" => Some("bash"),
        "zsh" => Some("zsh"),
        "fish" => Some("fish"),
        "powershell" | "pwsh" => Some("powershell"),
        _ => None,
    }
}

const BASH_COMPLETION: &str = r#"_perl_lsp() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="--stdio --mcp --socket --port --log --health --info --check --check-project --version --features-json --perltidy-compat-report --perlcritic-compat-report --feature-profile --completion --help"

    case "${prev}" in
        --port)
            return 0
            ;;
        --feature-profile)
            COMPREPLY=( $(compgen -W "ga-lock ga prod production all auto" -- "${cur}") )
            return 0
            ;;
        --completion)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell pwsh" -- "${cur}") )
            return 0
            ;;
        --perltidy-compat-report|--perlcritic-compat-report)
            COMPREPLY=( $(compgen -f -- "${cur}") )
            return 0
            ;;
        --check)
            COMPREPLY=( $(compgen -f -X '!*.pl' -- "${cur}") $(compgen -f -X '!*.pm' -- "${cur}") $(compgen -f -X '!*.t' -- "${cur}") $(compgen -d -- "${cur}") )
            return 0
            ;;
    esac

    if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi
}
complete -F _perl_lsp perl-lsp
"#;

const ZSH_COMPLETION: &str = r#"#compdef perl-lsp

_perl-lsp() {
    _arguments \
        '--stdio[Use stdio for communication (default)]' \
        '--mcp[Alias for stdio mode (MCP clients)]' \
        '--socket[Use TCP socket for communication]' \
        '--port[Port to listen on]:port:' \
        '--log[Enable logging to stderr]' \
        '--health[Quick health check]' \
        '--info[Show server info]' \
        '--check[Validate Perl files]:file:_files -g "*.{pl,pm,t}"' \
        '--check-project[Scan project directory for parsability report]:dir:_directories' \
        '--version[Show version information]' \
        '--features-json[Output features catalog as JSON]' \
        '--perltidy-compat-report[Report native formatter compatibility for .perltidyrc]:profile:_files' \
        '--perlcritic-compat-report[Report native critic compatibility for .perlcriticrc]:profile:_files' \
        '--feature-profile[Set feature profile]:profile:(ga-lock ga prod production all auto)' \
        '--completion[Generate shell completions]:shell:(bash zsh fish powershell pwsh)' \
        '--help[Show help message]' \
        '*:file:_files -g "*.{pl,pm,t}"'
}

_perl-lsp "$@"
"#;

const FISH_COMPLETION: &str = r#"complete -c perl-lsp -l stdio -d 'Use stdio for communication (default)'
complete -c perl-lsp -l mcp -d 'Alias for stdio mode (MCP clients)'
complete -c perl-lsp -l socket -d 'Use TCP socket for communication'
complete -c perl-lsp -l port -x -d 'Port to listen on'
complete -c perl-lsp -l log -d 'Enable logging to stderr'
complete -c perl-lsp -l health -d 'Quick health check'
complete -c perl-lsp -l info -d 'Show server info'
complete -c perl-lsp -l check -F -d 'Validate Perl files'
complete -c perl-lsp -l check-project -d 'Scan project directory for parsability report'
complete -c perl-lsp -l version -d 'Show version information'
complete -c perl-lsp -l features-json -d 'Output features catalog as JSON'
complete -c perl-lsp -l perltidy-compat-report -F -d 'Report native formatter compatibility for .perltidyrc'
complete -c perl-lsp -l perlcritic-compat-report -F -d 'Report native critic compatibility for .perlcriticrc'
complete -c perl-lsp -l feature-profile -x -a 'ga-lock ga prod production all auto' -d 'Set feature profile'
complete -c perl-lsp -l completion -x -a 'bash zsh fish powershell pwsh' -d 'Generate shell completions'
complete -c perl-lsp -l help -d 'Show help message'
"#;

const POWERSHELL_COMPLETION: &str = r#"Register-ArgumentCompleter -Native -CommandName perl-lsp -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $options = @(
        [CompletionResult]::new('--stdio', '--stdio', 'ParameterName', 'Use stdio for communication (default)')
        [CompletionResult]::new('--mcp', '--mcp', 'ParameterName', 'Alias for stdio mode (MCP clients)')
        [CompletionResult]::new('--socket', '--socket', 'ParameterName', 'Use TCP socket for communication')
        [CompletionResult]::new('--port', '--port', 'ParameterName', 'Port to listen on')
        [CompletionResult]::new('--log', '--log', 'ParameterName', 'Enable logging to stderr')
        [CompletionResult]::new('--health', '--health', 'ParameterName', 'Quick health check')
        [CompletionResult]::new('--info', '--info', 'ParameterName', 'Show server info')
        [CompletionResult]::new('--check', '--check', 'ParameterName', 'Validate Perl files')
        [CompletionResult]::new('--check-project', '--check-project', 'ParameterName', 'Scan project directory for parsability report')
        [CompletionResult]::new('--version', '--version', 'ParameterName', 'Show version information')
        [CompletionResult]::new('--features-json', '--features-json', 'ParameterName', 'Output features catalog as JSON')
        [CompletionResult]::new('--perltidy-compat-report', '--perltidy-compat-report', 'ParameterName', 'Report native formatter compatibility for .perltidyrc')
        [CompletionResult]::new('--perlcritic-compat-report', '--perlcritic-compat-report', 'ParameterName', 'Report native critic compatibility for .perlcriticrc')
        [CompletionResult]::new('--feature-profile', '--feature-profile', 'ParameterName', 'Set feature profile')
        [CompletionResult]::new('--completion', '--completion', 'ParameterName', 'Generate shell completions')
        [CompletionResult]::new('--help', '--help', 'ParameterName', 'Show help message')
    )

    $elements = $commandAst.CommandElements
    $prevWord = if ($elements.Count -ge 2) { $elements[$elements.Count - 2].Extent.Text } else { '' }

    switch ($prevWord) {
        '--completion' {
            @('bash', 'zsh', 'fish', 'powershell', 'pwsh') | Where-Object { $_ -like "$wordToComplete*" } |
                ForEach-Object { [CompletionResult]::new($_, $_, 'ParameterValue', $_) }
            return
        }
        '--feature-profile' {
            @('ga-lock', 'ga', 'prod', 'production', 'all', 'auto') | Where-Object { $_ -like "$wordToComplete*" } |
                ForEach-Object { [CompletionResult]::new($_, $_, 'ParameterValue', $_) }
            return
        }
    }

    $options | Where-Object { $_.CompletionText -like "$wordToComplete*" }
}
"#;

/// Format a colored health status line.
///
/// When `use_color` is true, "ok" is wrapped in ANSI green and the version
/// is shown in bold. Callers should pass `use_color = true` only when stdout
/// is a terminal (output goes to stdout, not stderr).
pub fn format_health_output(version: &str, use_color: bool) -> String {
    if use_color {
        format!("\x1b[32;1mok\x1b[0m \x1b[1m{version}\x1b[0m")
    } else {
        format!("ok {version}")
    }
}

/// Format the `--info` output block.
///
/// `version`, `git_tag`, `exe_path` are supplied by the binary crate.
pub fn format_info_output(
    version: &str,
    git_tag: &str,
    exe_path: &str,
    profile: FeatureProfile,
    use_color: bool,
) -> String {
    let feature_count = catalog_advertised_feature_ids(profile).len();
    let spec_total = trackable_feature_count_for_grid();
    let coverage = compliance_percent_for_profile(profile);

    let mut out = String::with_capacity(256);

    if use_color {
        out.push_str(&format!("\x1b[1mperl-lsp\x1b[0m {version}\n"));
    } else {
        out.push_str(&format!("perl-lsp {version}\n"));
    }
    out.push_str(&format!("Git tag:          {git_tag}\n"));
    out.push_str("Parser:           perl-parser v3 (recursive descent)\n");
    out.push_str(&format!("Profile:          {}\n", profile.as_str()));
    out.push_str(&format!("Features:         {feature_count}/{feature_count} active (100%)\n"));
    out.push_str(&format!("LSP spec coverage: {feature_count}/{spec_total} ({coverage:.0}%)\n"));
    out.push_str(&format!("Executable:       {exe_path}\n"));
    out.push_str("\nTip: run with --log or set PERL_LSP_LOG=1 for diagnostics\n");

    out
}

/// Format the one-line process-start banner written to stderr before the LSP handshake.
///
/// The `is_socket` parameter controls whether the transport hint reads "socket" or "stdio".
/// Callers should pass `is_socket = true` when the server is started in TCP socket mode.
///
/// Suppressible at the call site via `startup_banner()` which checks `PERL_LSP_QUIET`.
pub fn format_startup_banner(version: &str, profile: FeatureProfile, is_socket: bool) -> String {
    let feature_count = catalog_advertised_feature_ids(profile).len();
    let transport_hint = if is_socket { "socket" } else { "stdio" };
    format!("perl-lsp v{version} starting ({transport_hint}, {feature_count} features)")
}

/// Emit the process-start banner to stderr.
///
/// Fires before the LSP handshake begins. Writes directly to stderr, not through
/// tracing, so it is visible regardless of whether `--log` is active.
/// Suppressed when `PERL_LSP_QUIET` is set in the environment.
// The startup banner is intentionally written to stderr before the tracing subscriber
// is configured. This is the one permitted `eprintln!` in this crate.
#[allow(clippy::print_stderr)]
pub fn startup_banner(version: &str, profile: FeatureProfile, transport: TransportMode) {
    if std::env::var("PERL_LSP_QUIET").is_ok() {
        return;
    }
    eprintln!("{}", format_startup_banner(version, profile, transport.is_socket()));
}

/// Produce a user-friendly message when the TCP port is already in use.
pub fn port_in_use_message(port: u16) -> String {
    let alt1 = port.wrapping_add(1);
    let alt2 = port.wrapping_add(10);
    format!(
        "Port {port} is already in use. Another instance of perl-lsp may be running.\n\
         Try a different port:\n\
         \n\
         \x20 perllsp --socket --port {alt1}\n\
         \x20 perllsp --socket --port {alt2}\n\
         \n\
         Or stop the existing process using port {port}."
    )
}

fn parse_feature_profile(raw_profile: &str) -> Result<FeatureProfile, LaunchParseError> {
    parse_feature_profile_arg(raw_profile).map_err(|_| LaunchParseError::InvalidFeatureProfile {
        raw_profile: raw_profile.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_LSP_PORT, DiagnosticMode, LaunchAction, RuntimeMode, RuntimeTuning, TransportMode,
        parse_args,
    };
    use perl_tdd_support::must;

    #[test]
    fn init_logging_does_not_panic_without_log_file() {
        // init_logging is guarded by Once, so calling it multiple times is safe.
        // This test verifies the stderr-only path does not panic.
        super::init_logging("warn");
    }

    #[test]
    #[allow(unsafe_code)]
    fn init_logging_does_not_panic_with_log_file() {
        let dir = std::env::temp_dir().join("perl-lsp-test-log-rotation");
        let _ = std::fs::create_dir_all(&dir);
        let log_path = dir.join("test.log");

        // Set the env var for this test — init_logging is Once-guarded so the
        // file path may not actually be used if another test already initialized,
        // but this must not panic regardless.
        // SAFETY: test-only, single-threaded access to this env var.
        unsafe {
            std::env::set_var("PERL_LSP_LOG_FILE", log_path.to_str().unwrap_or_default());
        }
        super::init_logging("debug");
        // SAFETY: test-only cleanup.
        unsafe {
            std::env::remove_var("PERL_LSP_LOG_FILE");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn help_mentions_log_file_env_var() {
        let text = super::help_text();
        assert!(text.contains("PERL_LSP_LOG_FILE"));
    }

    #[test]
    fn parse_defaults_to_stdio_with_current_profile() {
        let plan = must(parse_args(["perl-lsp"]));

        assert_eq!(plan.action, LaunchAction::Run);
        assert_eq!(plan.config.transport, TransportMode::Stdio);
        assert!(!plan.config.enable_logging);
        assert_eq!(plan.config.feature_profile, super::FeatureProfile::current());
    }

    #[test]
    fn parse_mcp_alias_uses_stdio_transport() {
        let plan = must(parse_args(["perl-lsp", "--mcp"]));
        assert_eq!(plan.config.transport, TransportMode::Stdio);
    }

    #[test]
    fn mcp_alias_documented_consistently_across_surfaces() {
        // Help text, every shell completion, and the CLI parser must all
        // advertise --mcp. If any surface is forgotten when a future rename
        // lands, this test catches it before users hit broken tab-completion
        // or stale docs.
        let help = super::help_text();
        assert!(help.contains("--mcp"), "help_text is missing --mcp: {help}");
        assert!(
            help.contains("perllsp --mcp"),
            "help_text examples are missing a --mcp invocation: {help}"
        );

        // Fish uses `-l mcp` (long-option form without double-dashes);
        // other shells embed `--mcp` literally. Pick the right token per shell.
        for (shell, needle) in
            [("bash", "--mcp"), ("zsh", "--mcp"), ("fish", "-l mcp"), ("powershell", "--mcp")]
        {
            let script = super::shell_completion(shell)
                .unwrap_or_else(|| panic!("missing completion for {shell}"));
            assert!(script.contains(needle), "{shell} completion is missing {needle}: {script}");
        }

        // Parser side: --mcp must still resolve to stdio (alias semantics).
        let plan = must(parse_args(["perl-lsp", "--mcp"]));
        assert_eq!(plan.config.transport, TransportMode::Stdio);
    }

    #[test]
    fn parse_socket_and_port_options() {
        let plan = must(parse_args(["perl-lsp", "--socket", "--port", "8123"]));
        assert_eq!(plan.config.transport, TransportMode::Socket { port: 8123 });

        let plan = must(parse_args(["perl-lsp", "--port", "8123", "--socket"]));
        assert_eq!(plan.config.transport, TransportMode::Socket { port: 8123 });
    }

    #[test]
    fn parse_port_implies_socket() {
        let plan = must(parse_args(["perl-lsp", "--port", "8080"]));
        assert_eq!(plan.config.transport, TransportMode::Socket { port: 8080 });
    }

    #[test]
    fn parse_feature_profile_aliases() {
        let plan = must(parse_args(["perl-lsp", "--feature-profile", "ga_lock"]));
        assert_eq!(plan.config.feature_profile.as_str(), "ga-lock");

        let plan = must(parse_args(["perl-lsp", "--feature-profile=all"]));
        assert_eq!(plan.config.feature_profile.as_str(), "all");
    }

    #[test]
    fn parse_help_is_terminal_action() {
        let plan = must(parse_args(["perl-lsp", "--help"]));
        assert_eq!(plan.action, LaunchAction::Help);
        assert_eq!(plan.config.transport, TransportMode::Stdio);
    }

    #[test]
    fn parse_features_json_has_transport_defaults() {
        let plan = must(parse_args(["perl-lsp", "--features-json"]));
        assert_eq!(plan.action, LaunchAction::FeaturesJson);
        assert_eq!(plan.config.transport, TransportMode::Stdio);
    }

    #[test]
    fn help_mentions_default_port() {
        let text = super::help_text();
        assert!(text.contains(&DEFAULT_LSP_PORT.to_string()));
    }

    // ── --info flag ───────────────────────────────────────────────

    #[test]
    fn parse_info_flag_sets_info_action() {
        let plan = must(parse_args(["perl-lsp", "--info"]));
        assert_eq!(plan.action, LaunchAction::Info);
    }

    // ── --check flag ──────────────────────────────────────────────

    #[test]
    fn parse_check_flag_sets_check_action() {
        let plan = must(parse_args(["perl-lsp", "--check"]));
        assert_eq!(plan.action, LaunchAction::Check);
    }

    // ── --completion flag ─────────────────────────────────────────

    #[test]
    fn parse_completion_bash() {
        let plan = must(parse_args(["perl-lsp", "--completion", "bash"]));
        assert_eq!(plan.action, LaunchAction::Completion { shell: "bash".to_string() });
    }

    #[test]
    fn parse_completion_zsh() {
        let plan = must(parse_args(["perl-lsp", "--completion", "zsh"]));
        assert_eq!(plan.action, LaunchAction::Completion { shell: "zsh".to_string() });
    }

    #[test]
    fn parse_completion_fish() {
        let plan = must(parse_args(["perl-lsp", "--completion", "fish"]));
        assert_eq!(plan.action, LaunchAction::Completion { shell: "fish".to_string() });
    }

    #[test]
    fn parse_completion_powershell() {
        let plan = must(parse_args(["perl-lsp", "--completion", "powershell"]));
        assert_eq!(plan.action, LaunchAction::Completion { shell: "powershell".to_string() });
    }

    #[test]
    fn parse_completion_pwsh_alias_canonicalizes_to_powershell() {
        let plan = must(parse_args(["perl-lsp", "--completion", "pwsh"]));
        assert_eq!(plan.action, LaunchAction::Completion { shell: "powershell".to_string() });
    }

    #[test]
    fn parse_completion_is_case_insensitive() {
        let plan = must(parse_args(["perl-lsp", "--completion=PowerShell"]));
        assert_eq!(plan.action, LaunchAction::Completion { shell: "powershell".to_string() });
    }

    #[test]
    fn parse_completion_unknown_shell_errors() {
        let result = parse_args(["perl-lsp", "--completion", "nushell"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_completion_missing_value_errors() {
        let result = parse_args(["perl-lsp", "--completion"]);
        assert!(result.is_err());
    }

    // ── shell_completion function ─────────────────────────────────

    #[test]
    fn shell_completion_bash_is_nonempty() {
        assert!(super::shell_completion("bash").is_some());
    }

    #[test]
    fn shell_completion_zsh_is_nonempty() {
        assert!(super::shell_completion("zsh").is_some());
    }

    #[test]
    fn shell_completion_fish_is_nonempty() {
        assert!(super::shell_completion("fish").is_some());
    }

    #[test]
    fn shell_completion_powershell_is_nonempty() {
        assert!(super::shell_completion("powershell").is_some());
    }

    #[test]
    fn shell_completion_pwsh_alias_is_nonempty() {
        assert!(super::shell_completion("pwsh").is_some());
    }

    #[test]
    fn shell_completion_unknown_is_none() {
        assert!(super::shell_completion("nushell").is_none());
    }

    // ── format_health_output ──────────────────────────────────────

    #[test]
    fn health_output_plain_contains_ok_and_version() {
        let out = super::format_health_output("0.10.0", false);
        assert!(out.contains("ok"));
        assert!(out.contains("0.10.0"));
        assert!(!out.contains("\x1b["));
    }

    #[test]
    fn health_output_colored_contains_ansi() {
        let out = super::format_health_output("0.10.0", true);
        assert!(out.contains("\x1b[32;1m"));
        assert!(out.contains("ok"));
        assert!(out.contains("0.10.0"));
    }

    // ── format_info_output ────────────────────────────────────────

    #[test]
    fn info_output_contains_essential_fields() {
        let out = super::format_info_output(
            "0.10.0",
            "v0.10.0",
            "/usr/bin/perl-lsp",
            super::FeatureProfile::current(),
            false,
        );
        assert!(out.contains("0.10.0"));
        assert!(out.contains("perl-parser v3"));
        assert!(out.contains("active (100%)"));
        assert!(out.contains("LSP spec coverage:"));
        assert!(out.contains("/usr/bin/perl-lsp"));
    }

    // ── port_in_use_message ───────────────────────────────────────

    #[test]
    fn port_in_use_message_suggests_alternatives() {
        let msg = super::port_in_use_message(9257);
        assert!(msg.contains("9257"));
        assert!(msg.contains("9258"));
        assert!(msg.contains("9267"));
        assert!(msg.contains("already in use"));
    }

    // ── help text new entries ─────────────────────────────────────

    #[test]
    fn help_mentions_info_flag() {
        let text = super::help_text();
        assert!(text.contains("--info"));
    }

    #[test]
    fn help_mentions_check_flag() {
        let text = super::help_text();
        assert!(text.contains("--check"));
    }

    #[test]
    fn help_mentions_completion_flag() {
        let text = super::help_text();
        assert!(text.contains("--completion"));
        assert!(text.contains("powershell"));
        assert!(text.contains("pwsh"));
    }

    #[test]
    fn help_mentions_quiet_environment_flag() {
        let text = super::help_text();
        assert!(text.contains("PERL_LSP_QUIET=1"));
        assert!(text.contains("startup banner"));
    }

    // -- --check-project flag -----------------------------------------

    #[test]
    fn parse_check_project_no_dir_defaults_to_dot() {
        let plan = must(parse_args(["perl-lsp", "--check-project"]));
        assert_eq!(plan.action, LaunchAction::CheckProject { dir: ".".to_string() });
    }

    #[test]
    fn parse_check_project_with_dir() {
        let plan = must(parse_args(["perl-lsp", "--check-project", "lib/"]));
        assert_eq!(plan.action, LaunchAction::CheckProject { dir: "lib/".to_string() });
    }

    #[test]
    fn help_mentions_check_project_flag() {
        let text = super::help_text();
        assert!(text.contains("--check-project"));
    }

    // ── InvalidShell error ────────────────────────────────────────

    #[test]
    fn error_display_invalid_shell() {
        let err = super::LaunchParseError::InvalidShell { raw_shell: "tcsh".to_string() };
        let msg = format!("{err}");
        assert!(msg.contains("tcsh"));
        assert!(msg.contains("bash"));
    }

    // ── format_startup_banner ─────────────────────────────────────

    #[test]
    fn startup_banner_contains_version() {
        let out = super::format_startup_banner("0.12.0", super::FeatureProfile::current(), false);
        assert!(out.contains("perl-lsp"), "banner must contain 'perl-lsp'");
        assert!(out.contains("0.12.0"), "banner must contain version");
        assert!(out.contains("starting"), "banner must contain 'starting'");
    }

    #[test]
    fn startup_banner_contains_feature_count() {
        let profile = super::FeatureProfile::current();
        let feature_count = super::catalog_advertised_feature_ids(profile).len();
        let out = super::format_startup_banner("0.12.0", profile, false);
        assert!(feature_count > 0, "feature count must be positive");
        assert!(
            out.contains(&feature_count.to_string()),
            "banner must contain feature count ({feature_count})"
        );
    }

    #[test]
    fn startup_banner_stdio_transport_hint() {
        let out = super::format_startup_banner("0.12.0", super::FeatureProfile::current(), false);
        assert!(out.contains("stdio"), "banner must show transport hint 'stdio'");
    }

    #[test]
    fn startup_banner_socket_transport_hint() {
        let out = super::format_startup_banner("0.12.0", super::FeatureProfile::current(), true);
        assert!(out.contains("socket"), "banner must show transport hint 'socket'");
    }

    #[test]
    #[allow(unsafe_code)]
    fn startup_banner_suppressed_by_quiet_env() {
        // Save previous value to avoid test pollution even if test panics.
        let previous = std::env::var_os("PERL_LSP_QUIET");

        // SAFETY: test-only env var manipulation; previous value is restored after test.
        unsafe {
            std::env::set_var("PERL_LSP_QUIET", "1");
        }

        // startup_banner must not panic when PERL_LSP_QUIET is set.
        // The transport argument must propagate through without crashing.
        super::startup_banner(
            "0.12.0",
            super::FeatureProfile::current(),
            super::TransportMode::Stdio,
        );

        // SAFETY: restore previous value.
        match previous {
            Some(value) => unsafe { std::env::set_var("PERL_LSP_QUIET", value) },
            None => unsafe { std::env::remove_var("PERL_LSP_QUIET") },
        }
    }

    // ANSI detection helpers

    /// Guard: NO_COLOR=1 must disable ANSI regardless of terminal state.
    #[test]
    fn ansi_no_color_env_disables_ansi() {
        let _guard = EnvGuard::set("NO_COLOR", "1");
        // Even pretending we have a terminal, NO_COLOR wins.
        assert!(!super::should_use_ansi(true));
    }

    /// Guard: CLICOLOR=0 must disable ANSI.
    #[test]
    fn ansi_clicolor_zero_disables_ansi() {
        let _guard_nc = EnvGuard::remove("NO_COLOR");
        let _guard_fc = EnvGuard::remove("FORCE_COLOR");
        let _guard_cfc = EnvGuard::remove("CLICOLOR_FORCE");
        let _guard = EnvGuard::set("CLICOLOR", "0");
        assert!(!super::should_use_ansi(true), "CLICOLOR=0 must disable ANSI");
    }

    /// Guard: FORCE_COLOR=1 must enable ANSI even without a terminal.
    #[test]
    fn ansi_force_color_enables_ansi_without_terminal() {
        let _guard_nc = EnvGuard::remove("NO_COLOR");
        let _guard = EnvGuard::set("FORCE_COLOR", "1");
        assert!(
            super::should_use_ansi(false),
            "FORCE_COLOR=1 must enable ANSI even without a terminal"
        );
    }

    /// Guard: TERM=dumb disables ANSI even when stdout/stderr is a terminal.
    #[test]
    fn ansi_term_dumb_disables_ansi() {
        let _guard_nc = EnvGuard::remove("NO_COLOR");
        let _guard_fc = EnvGuard::remove("FORCE_COLOR");
        let _guard_cfc = EnvGuard::remove("CLICOLOR_FORCE");
        let _guard_cc = EnvGuard::remove("CLICOLOR");
        let _guard = EnvGuard::set("TERM", "dumb");
        assert!(!super::should_use_ansi(true), "TERM=dumb must disable ANSI");
    }

    /// Guard: explicit force color still wins when TERM=dumb is present.
    #[test]
    fn ansi_force_color_overrides_term_dumb() {
        let _guard_nc = EnvGuard::remove("NO_COLOR");
        let _guard_cfc = EnvGuard::remove("CLICOLOR_FORCE");
        let _guard_cc = EnvGuard::remove("CLICOLOR");
        let _guard_term = EnvGuard::set("TERM", "dumb");
        let _guard_force = EnvGuard::set("FORCE_COLOR", "1");
        assert!(super::should_use_ansi(false), "FORCE_COLOR=1 must override TERM=dumb");
    }

    /// Guard: is_warp_terminal() must be true when TERM_PROGRAM=WarpTerminal.
    #[test]
    fn ansi_warp_terminal_detection() {
        let _guard = EnvGuard::set("TERM_PROGRAM", "WarpTerminal");
        assert!(super::is_warp_terminal(), "WarpTerminal must be detected");
    }

    /// Guard: TERM_PROGRAM=vscode (a non-Warp terminal) must NOT trigger the
    /// Warp fallback path. VSCode's integrated terminal already reports
    /// is_terminal() correctly, so ANSI support must flow through the
    /// terminal-detection branch rather than the Warp override.
    ///
    /// Regression guard: if `is_warp_terminal()` ever loosened its match
    /// (e.g. any non-empty TERM_PROGRAM → Warp), this test fails — which
    /// would otherwise silently start emitting ANSI in pipelines where
    /// VSCode ran perl-lsp with stderr redirected.
    #[test]
    fn ansi_vscode_does_not_trigger_warp_path() {
        let _guard_nc = EnvGuard::remove("NO_COLOR");
        let _guard_fc = EnvGuard::remove("FORCE_COLOR");
        let _guard_cfc = EnvGuard::remove("CLICOLOR_FORCE");
        let _guard_cc = EnvGuard::remove("CLICOLOR");
        let _guard_term = EnvGuard::remove("TERM");
        let _guard = EnvGuard::set("TERM_PROGRAM", "vscode");

        assert!(
            !super::is_warp_terminal(),
            "TERM_PROGRAM=vscode must not be classified as WarpTerminal"
        );

        // With a real terminal: ANSI enabled via the terminal-detection path.
        assert!(
            super::should_use_ansi(true),
            "TERM_PROGRAM=vscode with is_terminal=true must still enable ANSI"
        );

        // Without a terminal (e.g. vscode's task runner capturing stderr):
        // the Warp fallback must not rescue this case.
        assert!(
            !super::should_use_ansi(false),
            "TERM_PROGRAM=vscode with is_terminal=false must not enable ANSI via the Warp path"
        );
    }

    /// Global lock serializing env-var mutation across parallel tests.
    ///
    /// Env vars are process-global on Unix and Windows, and libtest runs
    /// unit tests on a threadpool by default. Without this lock, a test
    /// that sets NO_COLOR can briefly be observed by a parallel test that
    /// expects NO_COLOR unset, producing flaky failures. The thread-local
    /// depth counter makes the guard reentrant: a single test may create
    /// several `EnvGuard`s (e.g. to scrub multiple env vars) without
    /// deadlocking on itself.
    static ANSI_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    std::thread_local! {
        static ANSI_ENV_LOCK_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    fn acquire_ansi_env_lock() -> Option<std::sync::MutexGuard<'static, ()>> {
        ANSI_ENV_LOCK_DEPTH.with(|depth| {
            let current = depth.get();
            depth.set(current + 1);
            if current == 0 {
                Some(
                    ANSI_ENV_LOCK
                        .get_or_init(|| std::sync::Mutex::new(()))
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()),
                )
            } else {
                None
            }
        })
    }

    /// Helper to temporarily set/restore an env var for test isolation.
    ///
    /// Holds the shared `ANSI_ENV_LOCK` for the lifetime of the outermost
    /// guard on this thread; inner guards bump a depth counter and release
    /// the mutex when the outermost drops. This lets a single test freely
    /// create multiple guards without deadlocking, while still serializing
    /// across parallel tests.
    struct EnvGuard {
        key: String,
        previous: Option<String>,
        _lock: Option<std::sync::MutexGuard<'static, ()>>,
    }

    impl EnvGuard {
        #[allow(unsafe_code)]
        fn set(key: &str, value: &str) -> Self {
            let lock = acquire_ansi_env_lock();
            let previous = std::env::var(key).ok();
            // SAFETY: test-only env var manipulation, serialized by ANSI_ENV_LOCK;
            // restored in Drop.
            unsafe { std::env::set_var(key, value) };
            EnvGuard { key: key.to_string(), previous, _lock: lock }
        }

        #[allow(unsafe_code)]
        fn remove(key: &str) -> Self {
            let lock = acquire_ansi_env_lock();
            let previous = std::env::var(key).ok();
            // SAFETY: test-only env var manipulation, serialized by ANSI_ENV_LOCK;
            // restored in Drop.
            unsafe { std::env::remove_var(key) };
            EnvGuard { key: key.to_string(), previous, _lock: lock }
        }
    }

    impl Drop for EnvGuard {
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            match &self.previous {
                // SAFETY: restoring the previous value while still holding
                // the ANSI_ENV_LOCK mutex (via the outermost guard on this thread).
                Some(v) => unsafe { std::env::set_var(&self.key, v) },
                None => unsafe { std::env::remove_var(&self.key) },
            }
            ANSI_ENV_LOCK_DEPTH.with(|depth| {
                let current = depth.get();
                depth.set(current.saturating_sub(1));
            });
        }
    }

    #[test]
    fn startup_banner_socket_transport_derived_from_transport_mode() {
        // Verify that format_startup_banner reads the is_socket flag, not an env var.
        // Socket transport must show "socket"; stdio must show "stdio".
        let socket_banner = super::format_startup_banner(
            "0.12.0",
            super::FeatureProfile::current(),
            super::TransportMode::Socket { port: 9257 }.is_socket(),
        );
        assert!(
            socket_banner.contains("socket"),
            "socket transport must appear in banner: {socket_banner}"
        );
        assert!(
            !socket_banner.contains("stdio"),
            "socket banner must not show stdio: {socket_banner}"
        );

        let stdio_banner = super::format_startup_banner(
            "0.12.0",
            super::FeatureProfile::current(),
            super::TransportMode::Stdio.is_socket(),
        );
        assert!(
            stdio_banner.contains("stdio"),
            "stdio transport must appear in banner: {stdio_banner}"
        );
        assert!(
            !stdio_banner.contains("socket"),
            "stdio banner must not show socket: {stdio_banner}"
        );
    }

    // ── runtime tuning CLI surface (PR 1) ────────────────────────────

    /// Scrub every env var consulted by `RuntimeTuning::from_env` so the
    /// parse_args tests below see compiled defaults, not whatever the
    /// runner happened to export.
    fn scrub_runtime_tuning_env() -> Vec<EnvGuard> {
        ["PERL_LSP_E2E", "PERL_LSP_DIAGNOSTIC_MODE", "PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS",
         "PERL_LSP_EAGER_WORKSPACE_INDEXING", "PERL_LSP_FILE_WATCHERS"]
            .iter()
            .map(|key| EnvGuard::remove(key))
            .collect()
    }

    #[test]
    fn parse_defaults_apply_normal_runtime_tuning() {
        let _guards = scrub_runtime_tuning_env();
        let plan = must(parse_args(["perl-lsp"]));
        assert_eq!(plan.config.runtime_tuning, RuntimeTuning::normal_defaults());
    }

    #[test]
    fn parse_runtime_mode_e2e_sets_e2e_defaults() {
        let _guards = scrub_runtime_tuning_env();
        let plan = must(parse_args(["perl-lsp", "--runtime-mode", "e2e"]));
        assert_eq!(plan.config.runtime_tuning, RuntimeTuning::e2e_defaults());
    }

    #[test]
    fn parse_diagnostic_debounce_zero_flag() {
        let _guards = scrub_runtime_tuning_env();
        let plan = must(parse_args(["perl-lsp", "--diagnostic-debounce-ms", "0"]));
        assert_eq!(plan.config.runtime_tuning.diagnostic_debounce_ms, 0);
        assert!(plan.config.runtime_tuning.diagnostic_debounce_is_immediate());
    }

    #[test]
    fn parse_diagnostic_mode_syntax_only_flag() {
        let _guards = scrub_runtime_tuning_env();
        let plan = must(parse_args(["perl-lsp", "--diagnostic-mode", "syntax-only"]));
        assert_eq!(plan.config.runtime_tuning.diagnostic_mode, DiagnosticMode::SyntaxOnly);
        // Outside of --runtime-mode e2e, the other dials stay at normal defaults.
        assert_eq!(plan.config.runtime_tuning.runtime_mode, RuntimeMode::Normal);
        assert!(plan.config.runtime_tuning.eager_workspace_indexing);
    }

    #[test]
    fn parse_combined_runtime_e2e_with_overrides() {
        let _guards = scrub_runtime_tuning_env();
        let plan = must(parse_args([
            "perl-lsp",
            "--runtime-mode",
            "e2e",
            "--diagnostic-debounce-ms",
            "5",
            "--diagnostic-mode",
            "normal",
        ]));
        assert_eq!(plan.config.runtime_tuning.runtime_mode, RuntimeMode::E2e);
        // CLI wins over e2e default.
        assert_eq!(plan.config.runtime_tuning.diagnostic_mode, DiagnosticMode::Normal);
        assert_eq!(plan.config.runtime_tuning.diagnostic_debounce_ms, 5);
        // Other e2e defaults survive.
        assert!(!plan.config.runtime_tuning.eager_workspace_indexing);
        assert!(!plan.config.runtime_tuning.file_watchers);
    }

    #[test]
    fn parse_invalid_runtime_mode_errors() {
        let _guards = scrub_runtime_tuning_env();
        let err = parse_args(["perl-lsp", "--runtime-mode", "warp"])
            .expect_err("invalid runtime mode must be rejected");
        let msg = format!("{err}");
        assert!(msg.contains("Invalid runtime mode"), "saw: {msg}");
    }

    #[test]
    fn parse_invalid_diagnostic_mode_errors() {
        let _guards = scrub_runtime_tuning_env();
        let err = parse_args(["perl-lsp", "--diagnostic-mode", "loud"])
            .expect_err("invalid diagnostic mode must be rejected");
        let msg = format!("{err}");
        assert!(msg.contains("Invalid diagnostic mode"), "saw: {msg}");
    }

    #[test]
    fn env_e2e_propagates_through_parse_args() {
        let _scrub = scrub_runtime_tuning_env();
        let _enable = EnvGuard::set("PERL_LSP_E2E", "1");
        let plan = must(parse_args(["perl-lsp"]));
        assert_eq!(plan.config.runtime_tuning, RuntimeTuning::e2e_defaults());
    }

    #[test]
    fn cli_overrides_env_when_both_present() {
        let _scrub = scrub_runtime_tuning_env();
        let _enable = EnvGuard::set("PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS", "1000");
        let plan = must(parse_args(["perl-lsp", "--diagnostic-debounce-ms", "0"]));
        assert_eq!(plan.config.runtime_tuning.diagnostic_debounce_ms, 0);
    }

    #[test]
    fn help_mentions_runtime_tuning_flags() {
        let text = super::help_text();
        assert!(text.contains("--runtime-mode"));
        assert!(text.contains("--diagnostic-mode"));
        assert!(text.contains("--diagnostic-debounce-ms"));
        assert!(text.contains("PERL_LSP_E2E"));
    }
}
