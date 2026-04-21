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

        let use_ansi = std::env::var("NO_COLOR").is_err() && io::stderr().is_terminal();

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
    #[arg(long, default_value_t = false, conflicts_with = "socket")]
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

    /// Generate shell completions (bash, zsh, fish, powershell)
    #[arg(long)]
    pub completion: Option<String>,

    /// Output features catalog as JSON
    #[arg(long)]
    pub features_json: bool,

    /// Set feature profile
    #[arg(long)]
    pub feature_profile: Option<String>,

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
        /// Target shell (bash, zsh, fish, powershell).
        shell: String,
    },
    /// Print version information.
    Version,
    /// Print profile-scoped feature catalog JSON.
    FeaturesJson,
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
}

impl LaunchConfig {
    /// Create a default launch configuration for a given feature profile.
    pub const fn new(feature_profile: FeatureProfile) -> Self {
        Self { transport: TransportMode::Stdio, enable_logging: false, feature_profile }
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
                write!(f, "Invalid feature profile: {raw_profile}. Supported: {supported}",)
            }
            Self::InvalidPort { raw_port, reason } => {
                write!(f, "Invalid port value: {raw_port}. {reason}")
            }
            Self::InvalidShell { raw_shell } => {
                write!(f, "Unknown shell: {raw_shell}. Supported: bash, zsh, fish, powershell")
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

            let action = if parsed_args.health {
                LaunchAction::Health
            } else if parsed_args.info {
                LaunchAction::Info
            } else if parsed_args.check {
                LaunchAction::Check
            } else if let Some(maybe_dir) = parsed_args.check_project {
                let dir = maybe_dir.unwrap_or_else(|| ".".to_string());
                LaunchAction::CheckProject { dir }
            } else if let Some(shell) = parsed_args.completion {
                LaunchAction::Completion { shell }
            } else if parsed_args.features_json {
                LaunchAction::FeaturesJson
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

            match raw_shell.as_str() {
                "bash" | "zsh" | "fish" | "powershell" => {}
                _ => {
                    return Err(LaunchParseError::InvalidShell { raw_shell });
                }
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
    out.push_str("Usage: perl-lsp [options]\n");
    out.push_str("       perl-lsp --check <file.pl> [file2.pm ...]\n");
    out.push_str("       perl-lsp --check-project [dir]\n");
    out.push('\n');
    out.push_str("Server options:\n");
    out.push_str("  --stdio              Use stdio for communication (default)\n");
    out.push_str("  --socket             Use TCP socket for communication\n");
    out.push_str(&format!(
        "  --port <port>        Port to listen on (default: {DEFAULT_LSP_PORT})\n"
    ));
    out.push_str("  --log                Enable logging to stderr\n");
    out.push_str("  --feature-profile <name>\n");
    out.push_str(&format!("                       Set feature profile ({supported_profiles})\n"));
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
    out.push_str("  --completion <shell> Generate shell completions (bash, zsh, fish)\n");
    out.push_str("  --help               Show this help message\n");
    out.push('\n');
    out.push_str("Examples:\n");
    out.push_str("  perl-lsp --stdio                        # stdio mode (default)\n");
    out.push_str("  perl-lsp --stdio --log                   # with logging\n");
    out.push_str("  perl-lsp --socket --port 9257            # TCP socket mode\n");
    out.push_str("  perl-lsp --stdio --feature-profile=prod  # production profile\n");
    out.push_str("  perl-lsp --check lib/MyModule.pm         # syntax check\n");
    out.push_str("  perl-lsp --check-project lib/             # project scan\n");
    out.push_str("  perl-lsp --info                          # server information\n");
    out.push_str("  perl-lsp --completion bash >> ~/.bashrc   # install completions\n");
    out.push('\n');
    out.push_str("Environment:\n");
    out.push_str("  PERL_LSP_LOG=1       Enable logging (alternative to --log)\n");
    out.push_str("  PERL_LSP_LOG_FILE=<path>\n");
    out.push_str("                       Also log to a daily-rotated file (max 5 files)\n");
    out.push_str("  RUST_LOG=<filter>    Set tracing filter (e.g. perl_lsp=debug)\n");
    out.push_str("  NO_COLOR=1           Disable colored output\n");
    out
}

/// Generate shell completion script for the given shell name.
///
/// Returns `None` for unknown shell names.
pub fn shell_completion(shell: &str) -> Option<&'static str> {
    match shell {
        "bash" => Some(BASH_COMPLETION),
        "zsh" => Some(ZSH_COMPLETION),
        "fish" => Some(FISH_COMPLETION),
        "powershell" => Some(POWERSHELL_COMPLETION),
        _ => None,
    }
}

const BASH_COMPLETION: &str = r#"_perl_lsp() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="--stdio --socket --port --log --health --info --check --check-project --version --features-json --feature-profile --completion --help"

    case "${prev}" in
        --port)
            return 0
            ;;
        --feature-profile)
            COMPREPLY=( $(compgen -W "ga-lock ga prod production all auto" -- "${cur}") )
            return 0
            ;;
        --completion)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell" -- "${cur}") )
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
        '--socket[Use TCP socket for communication]' \
        '--port[Port to listen on]:port:' \
        '--log[Enable logging to stderr]' \
        '--health[Quick health check]' \
        '--info[Show server info]' \
        '--check[Validate Perl files]:file:_files -g "*.{pl,pm,t}"' \
        '--check-project[Scan project directory for parsability report]:dir:_directories' \
        '--version[Show version information]' \
        '--features-json[Output features catalog as JSON]' \
        '--feature-profile[Set feature profile]:profile:(ga-lock ga prod production all auto)' \
        '--completion[Generate shell completions]:shell:(bash zsh fish powershell)' \
        '--help[Show help message]' \
        '*:file:_files -g "*.{pl,pm,t}"'
}

_perl-lsp "$@"
"#;

const FISH_COMPLETION: &str = r#"complete -c perl-lsp -l stdio -d 'Use stdio for communication (default)'
complete -c perl-lsp -l socket -d 'Use TCP socket for communication'
complete -c perl-lsp -l port -x -d 'Port to listen on'
complete -c perl-lsp -l log -d 'Enable logging to stderr'
complete -c perl-lsp -l health -d 'Quick health check'
complete -c perl-lsp -l info -d 'Show server info'
complete -c perl-lsp -l check -F -d 'Validate Perl files'
complete -c perl-lsp -l check-project -d 'Scan project directory for parsability report'
complete -c perl-lsp -l version -d 'Show version information'
complete -c perl-lsp -l features-json -d 'Output features catalog as JSON'
complete -c perl-lsp -l feature-profile -x -a 'ga-lock ga prod production all auto' -d 'Set feature profile'
complete -c perl-lsp -l completion -x -a 'bash zsh fish powershell' -d 'Generate shell completions'
complete -c perl-lsp -l help -d 'Show help message'
"#;

const POWERSHELL_COMPLETION: &str = r#"Register-ArgumentCompleter -Native -CommandName perl-lsp -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $options = @(
        [CompletionResult]::new('--stdio', '--stdio', 'ParameterName', 'Use stdio for communication (default)')
        [CompletionResult]::new('--socket', '--socket', 'ParameterName', 'Use TCP socket for communication')
        [CompletionResult]::new('--port', '--port', 'ParameterName', 'Port to listen on')
        [CompletionResult]::new('--log', '--log', 'ParameterName', 'Enable logging to stderr')
        [CompletionResult]::new('--health', '--health', 'ParameterName', 'Quick health check')
        [CompletionResult]::new('--info', '--info', 'ParameterName', 'Show server info')
        [CompletionResult]::new('--check', '--check', 'ParameterName', 'Validate Perl files')
        [CompletionResult]::new('--check-project', '--check-project', 'ParameterName', 'Scan project directory for parsability report')
        [CompletionResult]::new('--version', '--version', 'ParameterName', 'Show version information')
        [CompletionResult]::new('--features-json', '--features-json', 'ParameterName', 'Output features catalog as JSON')
        [CompletionResult]::new('--feature-profile', '--feature-profile', 'ParameterName', 'Set feature profile')
        [CompletionResult]::new('--completion', '--completion', 'ParameterName', 'Generate shell completions')
        [CompletionResult]::new('--help', '--help', 'ParameterName', 'Show help message')
    )

    $elements = $commandAst.CommandElements
    $prevWord = if ($elements.Count -ge 2) { $elements[$elements.Count - 2].Extent.Text } else { '' }

    switch ($prevWord) {
        '--completion' {
            @('bash', 'zsh', 'fish', 'powershell') | Where-Object { $_ -like "$wordToComplete*" } |
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
         \x20 perl-lsp --socket --port {alt1}\n\
         \x20 perl-lsp --socket --port {alt2}\n\
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
    use super::{DEFAULT_LSP_PORT, LaunchAction, TransportMode, parse_args};
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
}
