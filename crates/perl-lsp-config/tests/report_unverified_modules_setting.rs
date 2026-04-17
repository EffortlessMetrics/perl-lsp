//! Tests for the reportUnverifiedModules workspace setting.
//!
//! These tests verify that the WorkspaceConfig correctly parses and stores
//! the reportUnverifiedModules setting (default: false).
//!
//! IMPORTANT: These tests verify EXPECTED behavior that does not yet exist.
//! They will FAIL until the implementation is complete.

use perl_lsp_config::WorkspaceConfig;

// ============================================================================
// reportUnverifiedModules default value tests
// ============================================================================

/// report_unverified_modules should default to false
#[test]
fn workspace_config_report_unverified_modules_defaults_to_false() {
    let config = WorkspaceConfig::default();
    assert!(!config.report_unverified_modules, "report_unverified_modules should default to false");
}

// ============================================================================
// reportUnverifiedModules parsing tests
// ============================================================================

/// reportUnverifiedModules: true should be parsed correctly
#[test]
fn workspace_config_parses_report_unverified_modules_true() {
    let mut config = WorkspaceConfig::default();
    let settings = serde_json::json!({
        "workspace": {
            "reportUnverifiedModules": true
        }
    });
    config.update_from_value(&settings);
    assert!(
        config.report_unverified_modules,
        "report_unverified_modules should be true when set to true in settings"
    );
}

/// reportUnverifiedModules: false should be parsed correctly
#[test]
fn workspace_config_parses_report_unverified_modules_false() {
    let mut config = WorkspaceConfig::default();
    // First set it to true
    let settings_true = serde_json::json!({
        "workspace": {
            "reportUnverifiedModules": true
        }
    });
    config.update_from_value(&settings_true);
    assert!(config.report_unverified_modules, "should be true after setting to true");

    // Then set it to false
    let settings_false = serde_json::json!({
        "workspace": {
            "reportUnverifiedModules": false
        }
    });
    config.update_from_value(&settings_false);
    assert!(
        !config.report_unverified_modules,
        "report_unverified_modules should be false when set to false in settings"
    );
}

/// Missing reportUnverifiedModules should leave the field unchanged
#[test]
fn workspace_config_missing_report_unverified_modules_unchanged() {
    let mut config = WorkspaceConfig::default();
    // Set to true first
    let settings_true = serde_json::json!({
        "workspace": {
            "reportUnverifiedModules": true
        }
    });
    config.update_from_value(&settings_true);
    assert!(config.report_unverified_modules, "should be true after setting to true");

    // Update with a different field only
    let settings_partial = serde_json::json!({
        "workspace": {
            "includePaths": ["/custom/lib"]
        }
    });
    config.update_from_value(&settings_partial);
    // report_unverified_modules should remain true
    assert!(
        config.report_unverified_modules,
        "report_unverified_modules should remain unchanged when not in settings"
    );
}

/// Empty workspace section should leave report_unverified_modules unchanged
#[test]
fn workspace_config_empty_workspace_leaves_report_unverified_modules_unchanged() {
    let mut config = WorkspaceConfig::default();
    assert!(!config.report_unverified_modules, "should default to false initially");

    let settings = serde_json::json!({
        "workspace": {}
    });
    config.update_from_value(&settings);
    assert!(
        !config.report_unverified_modules,
        "report_unverified_modules should remain false after empty workspace update"
    );
}

/// Non-boolean value for reportUnverifiedModules should be silently ignored
#[test]
fn workspace_config_ignores_non_bool_report_unverified_modules() {
    let mut config = WorkspaceConfig::default();

    // Set to true first
    let settings_true = serde_json::json!({
        "workspace": {
            "reportUnverifiedModules": true
        }
    });
    config.update_from_value(&settings_true);
    assert!(config.report_unverified_modules, "should be true");

    // Try to set to a string (should be ignored)
    let settings_string = serde_json::json!({
        "workspace": {
            "reportUnverifiedModules": "true"
        }
    });
    config.update_from_value(&settings_string);
    // Should remain true (string was ignored)
    assert!(
        config.report_unverified_modules,
        "report_unverified_modules should remain true when string value is ignored"
    );
}

// ============================================================================
// Integration: reportUnverifiedModules with other settings
// ============================================================================

/// reportUnverifiedModules can be set alongside other workspace settings
#[test]
fn workspace_config_report_unverified_modules_with_other_settings() {
    let mut config = WorkspaceConfig::default();
    let settings = serde_json::json!({
        "workspace": {
            "includePaths": ["/lib", "/local/lib"],
            "useSystemInc": true,
            "reportUnverifiedModules": true,
            "resolutionTimeout": 100
        }
    });
    config.update_from_value(&settings);

    assert!(config.report_unverified_modules, "report_unverified_modules should be true");
    assert_eq!(config.include_paths, vec!["/lib", "/local/lib"]);
    assert!(config.use_system_inc, "use_system_inc should be true");
    assert_eq!(config.resolution_timeout_ms, 100);
}
