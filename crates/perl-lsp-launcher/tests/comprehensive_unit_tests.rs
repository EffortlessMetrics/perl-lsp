//! Comprehensive unit tests for the `perl-lsp-launcher` crate.
//!
//! Covers: CLI arg parsing, transport modes, launch actions, feature profiles,
//! error handling, edge cases, help text, and LaunchConfig API.
#![allow(clippy::assertions_on_constants, clippy::absurd_extreme_comparisons, unused_comparisons)]

use perl_lsp_launcher::{
    DEFAULT_LSP_PORT, FeatureProfile, LaunchConfig, LaunchParseError, TransportMode,
    catalog_advertised_feature_ids, help_text, parse_args, to_json_for_profile,
};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Module: TransportMode unit behavior
// ---------------------------------------------------------------------------

#[test]
fn transport_mode_stdio_label() {
    assert_eq!(TransportMode::Stdio.label(), "stdio");
}

#[test]
fn transport_mode_socket_label() {
    let mode = TransportMode::Socket { port: 9999 };
    assert_eq!(mode.label(), "socket");
}

#[test]
fn transport_mode_stdio_has_no_port() {
    assert_eq!(TransportMode::Stdio.port(), None);
}

#[test]
fn transport_mode_socket_returns_port() {
    let mode = TransportMode::Socket { port: 4040 };
    assert_eq!(mode.port(), Some(4040));
}

#[test]
fn transport_mode_stdio_is_not_socket() {
    assert!(!TransportMode::Stdio.is_socket());
}

#[test]
fn transport_mode_socket_is_socket() {
    assert!(TransportMode::Socket { port: 1234 }.is_socket());
}

#[test]
fn transport_mode_socket_preserves_exact_port() {
    let mode = TransportMode::Socket { port: DEFAULT_LSP_PORT };
    assert_eq!(mode.port(), Some(DEFAULT_LSP_PORT));
}

#[test]
fn transport_mode_equality() {
    assert_eq!(TransportMode::Stdio, TransportMode::Stdio);
    assert_eq!(TransportMode::Socket { port: 100 }, TransportMode::Socket { port: 100 });
    assert_ne!(TransportMode::Stdio, TransportMode::Socket { port: 100 });
    assert_ne!(TransportMode::Socket { port: 100 }, TransportMode::Socket { port: 200 });
}

// ---------------------------------------------------------------------------
// Module: LaunchConfig construction and accessors
// ---------------------------------------------------------------------------

#[test]
fn launch_config_new_defaults_to_stdio_no_logging() {
    let config = LaunchConfig::new(FeatureProfile::current());
    assert_eq!(config.transport, TransportMode::Stdio);
    assert!(!config.enable_logging);
}

#[test]
fn launch_config_features_json_is_nonempty() {
    let config = LaunchConfig::new(FeatureProfile::current());
    let json = config.features_json();
    assert!(!json.is_empty());
}

#[test]
fn launch_config_features_json_is_valid_json() {
    let config = LaunchConfig::new(FeatureProfile::current());
    let json = config.features_json();
    // A valid JSON object starts with '{' or '['
    let first = json.trim().chars().next().unwrap_or(' ');
    assert!(
        first == '{' || first == '[',
        "features_json should start with JSON delimiter, got: {first}"
    );
}

#[test]
fn launch_config_advertised_feature_ids_nonempty() {
    let config = LaunchConfig::new(FeatureProfile::current());
    let ids = config.advertised_feature_ids();
    assert!(!ids.is_empty(), "expected at least one advertised feature ID");
}

#[test]
fn launch_config_with_all_profile_has_most_features() {
    let all = LaunchConfig::new(FeatureProfile::All);
    let ga = LaunchConfig::new(FeatureProfile::GaLock);
    assert!(
        all.advertised_feature_ids().len() >= ga.advertised_feature_ids().len(),
        "All profile should have at least as many features as GaLock"
    );
}

// ---------------------------------------------------------------------------
// Module: parse_args — default / basic invocations
// ---------------------------------------------------------------------------

#[test]
fn parse_bare_invocation_is_run_stdio() {
    let plan = must(parse_args(["perl-lsp"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::Run);
    assert_eq!(plan.config.transport, TransportMode::Stdio);
    assert!(!plan.config.enable_logging);
}

#[test]
fn parse_explicit_stdio_flag() {
    let plan = must(parse_args(["perl-lsp", "--stdio"]));
    assert_eq!(plan.config.transport, TransportMode::Stdio);
}

#[test]
fn parse_log_flag_enables_logging() {
    let plan = must(parse_args(["perl-lsp", "--log"]));
    assert!(plan.config.enable_logging);
}

#[test]
fn parse_log_flag_off_by_default() {
    let plan = must(parse_args(["perl-lsp"]));
    assert!(!plan.config.enable_logging);
}

// ---------------------------------------------------------------------------
// Module: parse_args — socket transport
// ---------------------------------------------------------------------------

#[test]
fn parse_socket_flag_uses_default_port() {
    let plan = must(parse_args(["perl-lsp", "--socket"]));
    assert_eq!(plan.config.transport, TransportMode::Socket { port: DEFAULT_LSP_PORT });
}

#[test]
fn parse_socket_with_custom_port() {
    let plan = must(parse_args(["perl-lsp", "--socket", "--port", "7777"]));
    assert_eq!(plan.config.transport, TransportMode::Socket { port: 7777 });
}

#[test]
fn parse_port_alone_implies_socket_mode() {
    let plan = must(parse_args(["perl-lsp", "--port", "5555"]));
    assert!(plan.config.transport.is_socket());
    assert_eq!(plan.config.transport.port(), Some(5555));
}

#[test]
fn parse_socket_port_min_boundary() {
    let plan = must(parse_args(["perl-lsp", "--port", "1"]));
    assert_eq!(plan.config.transport, TransportMode::Socket { port: 1 });
}

#[test]
fn parse_socket_port_max_boundary() {
    let plan = must(parse_args(["perl-lsp", "--port", "65535"]));
    assert_eq!(plan.config.transport, TransportMode::Socket { port: 65535 });
}

// ---------------------------------------------------------------------------
// Module: parse_args — launch actions
// ---------------------------------------------------------------------------

#[test]
fn parse_health_flag_sets_health_action() {
    let plan = must(parse_args(["perl-lsp", "--health"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::Health);
}

#[test]
fn parse_features_json_flag() {
    let plan = must(parse_args(["perl-lsp", "--features-json"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::FeaturesJson);
}

#[test]
fn parse_help_flag_produces_help_action() {
    let plan = must(parse_args(["perl-lsp", "--help"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::Help);
}

#[test]
fn parse_version_flag_produces_version_action() {
    let plan = must(parse_args(["perl-lsp", "--version"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::Version);
}

// ---------------------------------------------------------------------------
// Module: parse_args — feature profile variants
// ---------------------------------------------------------------------------

#[test]
fn parse_feature_profile_ga_lock_hyphen() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "ga-lock"]));
    assert_eq!(plan.config.feature_profile.as_str(), "ga-lock");
}

#[test]
fn parse_feature_profile_ga_lock_underscore() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "ga_lock"]));
    assert_eq!(plan.config.feature_profile.as_str(), "ga-lock");
}

#[test]
fn parse_feature_profile_ga_shorthand() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "ga"]));
    assert_eq!(plan.config.feature_profile.as_str(), "ga-lock");
}

#[test]
fn parse_feature_profile_production() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "production"]));
    assert_eq!(plan.config.feature_profile.as_str(), "production");
}

#[test]
fn parse_feature_profile_prod_shorthand() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "prod"]));
    assert_eq!(plan.config.feature_profile.as_str(), "production");
}

#[test]
fn parse_feature_profile_all() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "all"]));
    assert_eq!(plan.config.feature_profile.as_str(), "all");
}

#[test]
fn parse_feature_profile_auto_resolves_to_current() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile", "auto"]));
    assert_eq!(plan.config.feature_profile, FeatureProfile::current());
}

#[test]
fn parse_feature_profile_equals_syntax() {
    let plan = must(parse_args(["perl-lsp", "--feature-profile=prod"]));
    assert_eq!(plan.config.feature_profile.as_str(), "production");
}

// ---------------------------------------------------------------------------
// Module: parse_args — combined flags
// ---------------------------------------------------------------------------

#[test]
fn parse_log_with_socket_transport() {
    let plan = must(parse_args(["perl-lsp", "--socket", "--log"]));
    assert!(plan.config.enable_logging);
    assert!(plan.config.transport.is_socket());
}

#[test]
fn parse_health_with_log_flag() {
    let plan = must(parse_args(["perl-lsp", "--health", "--log"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::Health);
    assert!(plan.config.enable_logging);
}

#[test]
fn parse_features_json_with_profile() {
    let plan = must(parse_args(["perl-lsp", "--features-json", "--feature-profile", "all"]));
    assert_eq!(plan.action, perl_lsp_launcher::LaunchAction::FeaturesJson);
    assert_eq!(plan.config.feature_profile.as_str(), "all");
}

#[test]
fn parse_socket_with_profile_and_log() {
    let plan = must(parse_args([
        "perl-lsp",
        "--socket",
        "--port",
        "3000",
        "--log",
        "--feature-profile",
        "production",
    ]));
    assert_eq!(plan.config.transport, TransportMode::Socket { port: 3000 });
    assert!(plan.config.enable_logging);
    assert_eq!(plan.config.feature_profile.as_str(), "production");
}

// ---------------------------------------------------------------------------
// Module: parse_args — error cases
// ---------------------------------------------------------------------------

#[test]
fn parse_unknown_option_returns_error() {
    let result = parse_args(["perl-lsp", "--nonexistent-flag"]);
    assert!(result.is_err());
}

#[test]
fn parse_invalid_feature_profile_returns_error() {
    let result = parse_args(["perl-lsp", "--feature-profile", "bogus_profile"]);
    assert!(result.is_err());
}

#[test]
fn parse_empty_feature_profile_returns_error() {
    let result = parse_args(["perl-lsp", "--feature-profile", ""]);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Module: LaunchParseError display formatting
// ---------------------------------------------------------------------------

#[test]
fn error_display_unknown_option() {
    let err = LaunchParseError::UnknownOption { option: "--bad".to_string() };
    let msg = format!("{err}");
    assert!(msg.contains("--bad"), "display should contain the option");
}

#[test]
fn error_display_missing_value() {
    let err = LaunchParseError::MissingValue { option: "--port".to_string() };
    let msg = format!("{err}");
    assert!(msg.contains("--port"));
    assert!(msg.contains("Missing value"));
}

#[test]
fn error_display_invalid_feature_profile() {
    let err = LaunchParseError::InvalidFeatureProfile { raw_profile: "nope".to_string() };
    let msg = format!("{err}");
    assert!(msg.contains("nope"));
    assert!(msg.contains("Supported"));
}

#[test]
fn error_display_invalid_port() {
    let err = LaunchParseError::InvalidPort {
        raw_port: "abc".to_string(),
        reason: "not a number".to_string(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("abc"));
    assert!(msg.contains("not a number"));
}

#[test]
fn error_implements_std_error() {
    let err = LaunchParseError::UnknownOption { option: "x".to_string() };
    // Verify Error trait is implemented by calling source()
    let _source: Option<&dyn std::error::Error> = std::error::Error::source(&err);
}

#[test]
fn error_debug_formatting() {
    let err = LaunchParseError::InvalidPort {
        raw_port: "99999".to_string(),
        reason: "out of range".to_string(),
    };
    let debug = format!("{err:?}");
    assert!(debug.contains("InvalidPort"));
}

// ---------------------------------------------------------------------------
// Module: help_text content validation
// ---------------------------------------------------------------------------

#[test]
fn help_text_contains_default_port() {
    let text = help_text();
    assert!(text.contains(&DEFAULT_LSP_PORT.to_string()));
}

#[test]
fn help_text_mentions_stdio_option() {
    let text = help_text();
    assert!(text.contains("--stdio"));
}

#[test]
fn help_text_mentions_socket_option() {
    let text = help_text();
    assert!(text.contains("--socket"));
}

#[test]
fn help_text_mentions_feature_profile() {
    let text = help_text();
    assert!(text.contains("--feature-profile"));
}

#[test]
fn help_text_mentions_health() {
    let text = help_text();
    assert!(text.contains("--health"));
}

#[test]
fn help_text_includes_examples_section() {
    let text = help_text();
    assert!(text.contains("Examples:"));
}

// ---------------------------------------------------------------------------
// Module: FeatureProfile / catalog integration
// ---------------------------------------------------------------------------

#[test]
fn catalog_advertised_ids_for_all_profile_nonempty() {
    let ids = catalog_advertised_feature_ids(FeatureProfile::All);
    assert!(!ids.is_empty());
}

#[test]
fn catalog_advertised_ids_for_ga_lock_nonempty() {
    let ids = catalog_advertised_feature_ids(FeatureProfile::GaLock);
    assert!(!ids.is_empty());
}

#[test]
fn to_json_for_profile_returns_valid_json() {
    let json = to_json_for_profile(FeatureProfile::current());
    let first = json.trim().chars().next().unwrap_or(' ');
    assert!(first == '{' || first == '[', "expected JSON object or array, got: {first}");
}

#[test]
fn to_json_for_each_profile_succeeds() {
    for &profile in FeatureProfile::all() {
        let json = to_json_for_profile(profile);
        assert!(
            !json.is_empty(),
            "to_json_for_profile({}) should return non-empty string",
            profile.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// Module: DEFAULT_LSP_PORT constant sanity
// ---------------------------------------------------------------------------

#[test]
fn default_port_is_in_valid_range() {
    assert!(DEFAULT_LSP_PORT > 0);
    assert!(DEFAULT_LSP_PORT <= 65535);
}

#[test]
fn default_port_is_expected_value() {
    assert_eq!(DEFAULT_LSP_PORT, 9257);
}

// ---------------------------------------------------------------------------
// Module: LaunchAction equality and Debug
// ---------------------------------------------------------------------------

#[test]
fn launch_action_variants_are_distinct() {
    use perl_lsp_launcher::LaunchAction;
    let actions = [
        LaunchAction::Run,
        LaunchAction::Health,
        LaunchAction::Version,
        LaunchAction::FeaturesJson,
        LaunchAction::Help,
    ];
    for (i, a) in actions.iter().enumerate() {
        for (j, b) in actions.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn launch_action_debug_output() {
    let debug = format!("{:?}", perl_lsp_launcher::LaunchAction::Run);
    assert!(debug.contains("Run"));
}

// ---------------------------------------------------------------------------
// Module: LaunchPlan struct accessibility
// ---------------------------------------------------------------------------

#[test]
fn launch_plan_fields_accessible() {
    let plan = must(parse_args(["perl-lsp", "--health"]));
    // Verify both fields are public and usable
    let _action = plan.action;
    let _transport = plan.config.transport;
    let _logging = plan.config.enable_logging;
    let _profile = plan.config.feature_profile;
}
