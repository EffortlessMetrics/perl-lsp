//! CLI and startup configuration primitives for the Perl LSP binary.
//!
//! This crate extracts the runtime launch decision surface into a dedicated crate so
//! feature profiles, transport mode semantics, and BDD-grid interoperability stay in one
//! place and remain stable across binaries.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;

pub use perl_lsp_feature_governance::{
    FeatureProfile, catalog_advertised_feature_ids, to_json_for_profile,
};
use perl_lsp_feature_governance::{feature_profile_supported_tokens, parse_feature_profile_arg};

/// Default port used by socket transport.
pub const DEFAULT_LSP_PORT: u16 = 9257;

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

    /// TCP port used by the transport.
    pub const fn port(self) -> u16 {
        match self {
            Self::Stdio => DEFAULT_LSP_PORT,
            Self::Socket { port } => port,
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
    I::Item: AsRef<str>,
{
    let args: Vec<String> = args.into_iter().map(|arg| arg.as_ref().to_string()).collect();

    let mut config = LaunchConfig::new(FeatureProfile::current());
    let mut socket_port = DEFAULT_LSP_PORT;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--stdio" => {
                config.transport = TransportMode::Stdio;
                index += 1;
            }
            "--socket" => {
                config.transport = TransportMode::Socket { port: socket_port };
                index += 1;
            }
            "--port" => {
                let raw_port = args.get(index + 1).ok_or_else(|| {
                    LaunchParseError::MissingValue { option: "--port".to_string() }
                })?;
                socket_port = parse_socket_port(raw_port)?;
                if matches!(config.transport, TransportMode::Socket { .. }) {
                    config.transport = TransportMode::Socket { port: socket_port };
                }
                index += 2;
            }
            "--log" => {
                config.enable_logging = true;
                index += 1;
            }
            "--health" => {
                return Ok(LaunchPlan { action: LaunchAction::Health, config });
            }
            "--feature-profile" => {
                let raw_profile = args.get(index + 1).ok_or_else(|| {
                    LaunchParseError::MissingValue { option: "--feature-profile".to_string() }
                })?;
                config.feature_profile = parse_feature_profile(raw_profile)?;
                index += 2;
            }
            arg if arg.starts_with("--feature-profile=") => {
                let raw_profile = arg.trim_start_matches("--feature-profile=");
                config.feature_profile = parse_feature_profile(raw_profile)?;
                index += 1;
            }
            "--features-json" => {
                return Ok(LaunchPlan { action: LaunchAction::FeaturesJson, config });
            }
            "--version" => {
                return Ok(LaunchPlan { action: LaunchAction::Version, config });
            }
            "--help" | "-h" => {
                return Ok(LaunchPlan { action: LaunchAction::Help, config });
            }
            arg => {
                return Err(LaunchParseError::UnknownOption { option: arg.to_string() });
            }
        }
    }

    Ok(LaunchPlan { action: LaunchAction::Run, config })
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

fn parse_socket_port(raw_port: &str) -> Result<u16, LaunchParseError> {
    raw_port.parse::<u16>().map_err(|error| LaunchParseError::InvalidPort {
        raw_port: raw_port.to_string(),
        reason: error.to_string(),
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
