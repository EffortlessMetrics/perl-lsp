//! CLI and startup configuration primitives for the Perl LSP binary.
//!
//! This crate extracts the runtime launch decision surface into a dedicated crate so
//! feature profiles, transport mode semantics, and BDD-grid interoperability stay in one
//! place and remain stable across binaries.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;

use clap::{Args, Parser};
pub use perl_lsp_feature_governance::{
    FeatureProfile, catalog_advertised_feature_ids, to_json_for_profile,
};
use perl_lsp_feature_governance::{feature_profile_supported_tokens, parse_feature_profile_arg};

/// Default port used by socket transport.
pub const DEFAULT_LSP_PORT: u16 = 9257;

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
    #[command(flatten)]
    pub transport: TransportArgs,

    /// Enable logging to stderr
    #[arg(long)]
    pub log: bool,

    /// Quick health check (prints 'ok `<version>`')
    #[arg(long)]
    pub health: bool,

    /// Output features catalog as JSON
    #[arg(long)]
    pub features_json: bool,

    /// Set feature profile
    #[arg(long)]
    pub feature_profile: Option<String>,
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
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LaunchAction {
    /// Start a running server.
    Run,
    /// Print quick health status.
    Health,
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
    match LspArgs::try_parse_from(args) {
        Ok(parsed_args) => {
            let mut config = LaunchConfig::new(FeatureProfile::current());

            config.transport = parsed_args.transport.mode();
            config.enable_logging = parsed_args.log;

            if let Some(raw_profile) = parsed_args.feature_profile {
                config.feature_profile = parse_feature_profile(&raw_profile)?;
            }

            let action = if parsed_args.health {
                LaunchAction::Health
            } else if parsed_args.features_json {
                LaunchAction::FeaturesJson
            } else {
                LaunchAction::Run
            };

            Ok(LaunchPlan { action, config })
        }
        Err(err) => {
            let is_help = err.kind() == clap::error::ErrorKind::DisplayHelp
                || err.kind() == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand;
            let is_version = err.kind() == clap::error::ErrorKind::DisplayVersion;

            if is_help {
                return Ok(LaunchPlan {
                    action: LaunchAction::Help,
                    config: LaunchConfig::new(FeatureProfile::current()),
                });
            } else if is_version {
                return Ok(LaunchPlan {
                    action: LaunchAction::Version,
                    config: LaunchConfig::new(FeatureProfile::current()),
                });
            }

            Err(LaunchParseError::UnknownOption { option: err.to_string() })
        }
    }
}

/// Human-readable CLI help text shared by CLI consumers.
pub fn help_text() -> String {
    let supported_profiles = feature_profile_supported_tokens().join(", ");

    format!(
        "Perl Language Server\n\
\
Usage: perl-lsp [options]\n\
\
Options:\n\
  --stdio          Use stdio for communication (default)\n\
  --socket         Use TCP socket for communication\n\
  --port           Port to listen on (default: {DEFAULT_LSP_PORT})\n\
  --log            Enable logging to stderr\n\
  --health         Quick health check (prints \'ok <version>\')\n\
  --version        Show version information\n\
  --features-json  Output features catalog as JSON\n\
  --feature-profile <name> Set feature profile\n\
                   Values: {supported_profiles}\n\
  --help           Show this help message\n\
\
Examples:\n\
  # Run in stdio mode (default)\n\
  perl-lsp --stdio\n\
\
  # Run with production profile\n\
  perl-lsp --stdio --feature-profile=prod\n\
\
  # Run with logging enabled\n\
  perl-lsp --stdio --log\n\
\
  # Run in socket mode\n\
  perl-lsp --socket --port 9257\n"
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
}
