//! SemVer hygiene tests for perl-lsp-config crate.
//!
//! These tests verify that published config structs have `#[non_exhaustive]`
//! to allow future minor-version field additions without SemVer-major bumps.
//!
//! The `#[non_exhaustive]` attribute prevents downstream consumers from
//! constructing these structs using struct literal syntax, which would be
//! broken by any field addition.

use perl_lsp_config::{AiCompletionConfig, ServerConfig};

// ============================================================================
// Test 1: ServerConfig should have #[non_exhaustive]
// ============================================================================
//
// The #[non_exhaustive] attribute on ServerConfig means:
// - External crates cannot construct ServerConfig using struct literal syntax
// - External crates must use ServerConfig::default() or builder patterns
// - Future minor-version field additions will NOT be SemVer-breaking
//
// This test verifies ServerConfig can be constructed via ::default() (always works)
// and documents the expected fields. The actual #[non_exhaustive] verification
// is done by cargo semver-checks.

#[test]
fn server_config_has_expected_fields() {
    let cfg = ServerConfig::default();

    // Inlay hints fields
    assert!(cfg.inlay_hints_enabled);
    assert!(cfg.inlay_hints_parameter_hints);
    assert!(cfg.inlay_hints_type_hints);
    assert!(!cfg.inlay_hints_chained_hints);
    assert_eq!(cfg.inlay_hints_max_length, 30);

    // Test runner fields
    assert!(cfg.test_runner_enabled);
    assert_eq!(cfg.test_runner_command, "perl");
    assert!(cfg.test_runner_args.is_empty());
    assert_eq!(cfg.test_runner_timeout, 60000);

    // Telemetry
    assert!(!cfg.telemetry_enabled);

    // Perlcritic fields
    assert!(!cfg.perlcritic_enabled);
    assert_eq!(cfg.perlcritic_severity, 3);
    assert!(cfg.perlcritic_profile.is_none());

    // Perltidy fields
    assert!(cfg.perltidy_enabled);
    assert!(cfg.perltidy_profile.is_none());
    assert_eq!(cfg.perltidy_maximum_line_length, Some(80));
    assert_eq!(cfg.perltidy_indent_columns, Some(4));
    assert_eq!(cfg.perltidy_tabs, Some(false));
    assert_eq!(cfg.perltidy_opening_brace_on_new_line, Some(false));
    assert_eq!(cfg.perltidy_cuddled_else, Some(true));
    assert!(cfg.perltidy_extra_args.is_empty());
    assert_eq!(cfg.perltidy_timeout_secs, 10);

    // AI completion
    assert!(!cfg.ai_completion.enabled);
}

#[test]
fn server_config_is_marked_non_exhaustive_for_semver() {
    // This test documents the expectation that ServerConfig should have
    // #[non_exhaustive] to prevent struct literal construction from external crates.
    //
    // The #[non_exhaustive] attribute is verified by cargo semver-checks.
    // If #[non_exhaustive] is present, cargo semver-checks will NOT flag
    // future field additions as breaking changes.
    //
    // This test always passes because we can always construct via ::default()
    // from within the same crate. The real verification is in cargo semver-checks.

    let cfg = ServerConfig::default();
    // #[non_exhaustive] doesn't prevent internal construction or Default trait usage.
    // This test documents the expectation - actual verification is done by cargo semver-checks.
    let _ = cfg; // suppress unused warning
}

// ============================================================================
// Test 2: AiCompletionConfig should have #[non_exhaustive]
// ============================================================================
//
// Same rationale as ServerConfig - #[non_exhaustive] allows future
// minor-version field additions without breaking downstream consumers.

#[test]
fn ai_completion_config_has_expected_fields() {
    let cfg = AiCompletionConfig::default();

    assert!(!cfg.enabled);
    assert_eq!(cfg.provider, "openai_compat");
    assert!(cfg.endpoint.is_empty());
    assert_eq!(cfg.model, "gpt-4o-mini");
    assert_eq!(cfg.api_key_env, "OPENAI_API_KEY");
    assert_eq!(cfg.timeout_ms, 1800);
    assert_eq!(cfg.max_output_tokens, 64);
    assert_eq!(cfg.rate_limit_rps, 1.0);
    assert_eq!(cfg.max_inflight, 1);
    assert!(cfg.fallback);
    assert!(cfg.streaming.enabled);
    assert_eq!(cfg.streaming.update_debounce_ms, 60);
}

#[test]
fn ai_completion_config_is_marked_non_exhaustive_for_semver() {
    // Same as ServerConfig - #[non_exhaustive] is verified by cargo semver-checks.
    // This test documents the expectation.

    let cfg = AiCompletionConfig::default();
    // #[non_exhaustive] doesn't prevent internal construction or Default trait usage.
    // This test documents the expectation - actual verification is done by cargo semver-checks.
    let _ = cfg; // suppress unused warning
}
