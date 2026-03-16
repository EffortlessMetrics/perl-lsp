//! Behavioral and mutation-killing tests for perl-lsp-config.
//!
//! The inline tests cover basic defaults and single-field updates.
//! These tests target:
//!
//! - Default values that differ from each other (chained hints defaults false, others true)
//! - Wrong-type values are silently ignored (idempotent for those fields)
//! - System-inc cache invalidation when use_system_inc changes
//! - test_runner_args: array with mixed types (non-strings filtered out)
//! - Complete snapshot: all fields updated in one call
//! - Telemetry: enabled/disabled toggle
//! - WorkspaceConfig: cache cleared on use_system_inc → false transition

use perl_lsp_config::{ServerConfig, WorkspaceConfig};

// ---------------------------------------------------------------------------
// ServerConfig defaults
// ---------------------------------------------------------------------------

#[test]
fn server_config_chained_hints_defaults_to_false_unlike_other_hints() {
    let cfg = ServerConfig::default();
    assert!(cfg.inlay_hints_enabled, "inlay_hints_enabled should default true");
    assert!(cfg.inlay_hints_parameter_hints, "inlay_hints_parameter_hints should default true");
    assert!(cfg.inlay_hints_type_hints, "inlay_hints_type_hints should default true");
    // chained hints defaults FALSE — mutation would swap true/false
    assert!(!cfg.inlay_hints_chained_hints, "inlay_hints_chained_hints should default false");
}

#[test]
fn server_config_inlay_hints_max_length_default_is_30() {
    let cfg = ServerConfig::default();
    assert_eq!(cfg.inlay_hints_max_length, 30);
}

#[test]
fn server_config_test_runner_timeout_default_is_60000() {
    let cfg = ServerConfig::default();
    assert_eq!(cfg.test_runner_timeout, 60000);
}

// ---------------------------------------------------------------------------
// ServerConfig: update all fields at once
// ---------------------------------------------------------------------------

#[test]
fn server_config_update_all_inlay_hints_fields() {
    let mut cfg = ServerConfig::default();
    let settings = serde_json::json!({
        "inlayHints": {
            "enabled": false,
            "parameterHints": false,
            "typeHints": false,
            "chainedHints": true,
            "maxLength": 50
        }
    });
    cfg.update_from_value(&settings);
    assert!(!cfg.inlay_hints_enabled);
    assert!(!cfg.inlay_hints_parameter_hints);
    assert!(!cfg.inlay_hints_type_hints);
    assert!(cfg.inlay_hints_chained_hints, "chainedHints should flip to true");
    assert_eq!(cfg.inlay_hints_max_length, 50);
}

#[test]
fn server_config_update_test_runner_args_as_array() {
    let mut cfg = ServerConfig::default();
    let settings = serde_json::json!({
        "testRunner": {
            "command": "prove",
            "args": ["-l", "-r", "t/"],
            "enabled": false,
            "timeout": 30000
        }
    });
    cfg.update_from_value(&settings);
    assert_eq!(cfg.test_runner_command, "prove");
    assert_eq!(cfg.test_runner_args, vec!["-l", "-r", "t/"]);
    assert!(!cfg.test_runner_enabled);
    assert_eq!(cfg.test_runner_timeout, 30000);
}

#[test]
fn server_config_test_runner_args_filters_non_string_values() {
    // The filter_map in update_from_value must skip non-string array elements
    let mut cfg = ServerConfig::default();
    let settings = serde_json::json!({
        "testRunner": {
            "args": ["-l", 42, true, "t/"]
        }
    });
    cfg.update_from_value(&settings);
    // Only string elements should be kept
    assert_eq!(cfg.test_runner_args, vec!["-l", "t/"]);
}

#[test]
fn server_config_telemetry_can_be_enabled() {
    let mut cfg = ServerConfig::default();
    assert!(!cfg.telemetry_enabled, "telemetry must default to false");

    let settings = serde_json::json!({
        "telemetry": { "enabled": true }
    });
    cfg.update_from_value(&settings);
    assert!(cfg.telemetry_enabled, "telemetry should be enabled after update");
}

#[test]
fn server_config_telemetry_can_be_disabled_again() {
    let mut cfg = ServerConfig::default();
    // first enable
    cfg.update_from_value(&serde_json::json!({"telemetry": {"enabled": true}}));
    assert!(cfg.telemetry_enabled);
    // then disable
    cfg.update_from_value(&serde_json::json!({"telemetry": {"enabled": false}}));
    assert!(!cfg.telemetry_enabled, "telemetry should be disabled after second update");
}

// ---------------------------------------------------------------------------
// ServerConfig: wrong-type values are silently ignored
// ---------------------------------------------------------------------------

#[test]
fn server_config_wrong_type_for_enabled_is_ignored() {
    let mut cfg = ServerConfig::default();
    let was_enabled = cfg.inlay_hints_enabled;
    // Pass a string where bool is expected
    let settings = serde_json::json!({
        "inlayHints": { "enabled": "yes" }
    });
    cfg.update_from_value(&settings);
    assert_eq!(cfg.inlay_hints_enabled, was_enabled, "wrong-type value must not change the field");
}

#[test]
fn server_config_wrong_type_for_max_length_is_ignored() {
    let mut cfg = ServerConfig::default();
    let was_max_len = cfg.inlay_hints_max_length;
    let settings = serde_json::json!({
        "inlayHints": { "maxLength": "thirty" }
    });
    cfg.update_from_value(&settings);
    assert_eq!(
        cfg.inlay_hints_max_length, was_max_len,
        "string value for maxLength must be ignored"
    );
}

// ---------------------------------------------------------------------------
// WorkspaceConfig defaults
// ---------------------------------------------------------------------------

#[test]
fn workspace_config_default_include_paths_are_three_standard_dirs() {
    let cfg = WorkspaceConfig::default();
    assert_eq!(cfg.include_paths, vec!["lib", ".", "local/lib/perl5"]);
}

#[test]
fn workspace_config_use_system_inc_defaults_to_false() {
    let cfg = WorkspaceConfig::default();
    assert!(!cfg.use_system_inc);
}

#[test]
fn workspace_config_resolution_timeout_defaults_to_50ms() {
    let cfg = WorkspaceConfig::default();
    assert_eq!(cfg.resolution_timeout_ms, 50);
}

// ---------------------------------------------------------------------------
// WorkspaceConfig: system_inc_cache invalidation
// ---------------------------------------------------------------------------

#[test]
fn workspace_config_enabling_use_system_inc_sets_flag() {
    let mut cfg = WorkspaceConfig::default();
    assert!(!cfg.use_system_inc);

    cfg.update_from_value(&serde_json::json!({
        "workspace": { "useSystemInc": true }
    }));

    assert!(cfg.use_system_inc, "use_system_inc should be true after update");
}

#[test]
fn workspace_config_toggling_use_system_inc_off_sets_flag_to_false() {
    let mut cfg = WorkspaceConfig::default();
    // first enable
    cfg.update_from_value(&serde_json::json!({"workspace": {"useSystemInc": true}}));
    assert!(cfg.use_system_inc);
    // then disable — cache should also be cleared
    cfg.update_from_value(&serde_json::json!({"workspace": {"useSystemInc": false}}));
    assert!(!cfg.use_system_inc, "use_system_inc should be false after second update");
    // get_system_inc must return empty since use_system_inc is false
    let inc = cfg.get_system_inc();
    assert!(inc.is_empty(), "get_system_inc must return empty when use_system_inc is false");
}

#[test]
fn workspace_config_get_system_inc_returns_empty_when_disabled() {
    let mut cfg = WorkspaceConfig::default();
    assert!(!cfg.use_system_inc);
    // Should return empty immediately without querying the system
    let paths = cfg.get_system_inc();
    assert!(paths.is_empty(), "should return empty slice when use_system_inc is false");
}

#[test]
fn workspace_config_update_include_paths_replaces_defaults() {
    let mut cfg = WorkspaceConfig::default();
    cfg.update_from_value(&serde_json::json!({
        "workspace": {
            "includePaths": ["/custom/lib", "/project/lib"]
        }
    }));
    assert_eq!(cfg.include_paths, vec!["/custom/lib", "/project/lib"]);
}

#[test]
fn workspace_config_update_include_paths_with_empty_array_clears_paths() {
    let mut cfg = WorkspaceConfig::default();
    cfg.update_from_value(&serde_json::json!({
        "workspace": { "includePaths": [] }
    }));
    assert!(cfg.include_paths.is_empty(), "empty array must clear include paths");
}

#[test]
fn workspace_config_update_resolution_timeout() {
    let mut cfg = WorkspaceConfig::default();
    cfg.update_from_value(&serde_json::json!({
        "workspace": { "resolutionTimeout": 200 }
    }));
    assert_eq!(cfg.resolution_timeout_ms, 200);
}

#[test]
fn workspace_config_top_level_key_missing_does_not_update() {
    let mut cfg = WorkspaceConfig::default();
    let original_paths = cfg.include_paths.clone();
    // Pass settings that have no "workspace" top-level key
    cfg.update_from_value(&serde_json::json!({ "other": {} }));
    assert_eq!(
        cfg.include_paths, original_paths,
        "missing top-level key must not change any field"
    );
}
