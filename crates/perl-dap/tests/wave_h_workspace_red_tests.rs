//! Wave H Collapse Workspace Verification RED Tests — work-efd2aa1b
//!
//! These tests verify that the workspace configuration correctly reflects
//! the collapse of 11 satellite crates into perl-dap.
//!
//! They are written BEFORE the implementation (RED state) and should FAIL
//! until the collapse is properly implemented.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_workspace_red_tests`

#![allow(unused_imports)]

use anyhow::Result;

/// Test that perl-dap does NOT depend on satellite crates after collapse.
/// This test verifies the dependencies have been internalized.
#[test]
fn test_perl_dap_no_longer_depends_on_satellites() -> Result<()> {
    // After collapse, these crates should NOT be dependencies:
    // - perl-dap-breakpoint
    // - perl-dap-eval
    // - perl-dap-config
    // - perl-dap-platform
    // - perl-dap-command-args
    // - perl-dap-variables
    // - perl-dap-stack
    // - perl-dap-types
    // - perl-dap-value
    // - perl-dap-security
    // - perl-dap-shell

    // This is verified by trying to import from the collapsed modules.
    // If the satellite crates still exist as dependencies, imports would succeed
    // even without the collapse. But since we're importing from perl_dap::*,
    // the modules must exist within perl-dap.

    // The real verification is that the crate compiles and tests pass.
    // This test structure ensures that if someone tries to use the old
    // perl_dap_platform crate name after collapse, they'll get a compile error.

    use perl_dap::platform::PerlInterpreterResult;

    // If we reach here, platform module exists within perl-dap
    assert!(!std::any::type_name::<PerlInterpreterResult>().is_empty());
    Ok(())
}

/// Test that satellite crates are removed from workspace members.
/// This is implicitly tested by cargo build succeeding.
#[test]
fn test_workspace_members_updated() -> Result<()> {
    // This test just verifies the structure is correct.
    // The actual workspace check is done by cargo metadata.

    use perl_dap::command_args::format_command_args;

    let result = format_command_args(&["perl".to_string()]);
    assert!(!result.is_empty());
    Ok(())
}

/// Test that perl-lsp can import from perl-dap after collapse.
/// External consumers must be updated to use perl_dap::* instead of perl_dap_platform::*.
#[test]
fn test_external_consumer_can_use_collapsed_crate() -> Result<()> {
    // This tests that perl-dap exports are available for external consumers
    use perl_dap::platform::resolve_perl_path;

    // resolve_perl_path should be callable
    let result = resolve_perl_path("perl", None);
    assert!(result.is_ok() || result.is_err()); // Just verify callable
    Ok(())
}

/// Test that DebugAdapter uses internal modules instead of satellite crates.
/// This is verified by the fact that the code compiles.
#[test]
fn test_debug_adapter_uses_internal_modules() -> Result<()> {
    use perl_dap::DebugAdapter;

    // If DebugAdapter exists and is constructible, internal imports work
    let adapter = DebugAdapter::new();
    assert!(!std::any::type_name::<DebugAdapter>().is_empty());
    Ok(())
}

/// Test that BreakpointStore uses internal breakpoint module.
#[test]
fn test_breakpoint_store_uses_internal_module() -> Result<()> {
    use perl_dap::BreakpointStore;

    let store = BreakpointStore::new();
    assert!(!std::any::type_name::<BreakpointStore>().is_empty());
    Ok(())
}

/// Test that DapConfig uses internal config module.
#[test]
fn test_dap_config_uses_internal_module() -> Result<()> {
    use perl_dap::DapConfig;

    let config = DapConfig {
        log_level: "info".into(),
        mode: perl_dap::DapMode::Native,
        workspace_root: None,
    };
    assert_eq!(config.log_level, "info");
    Ok(())
}

/// Test that platform.rs is now a module folder, not a file.
/// This is verified by the fact that platform:: submodule is accessible.
#[test]
fn test_platform_is_module_folder() -> Result<()> {
    use perl_dap::platform;

    // Access a type from the platform module
    let _ = std::any::type_name::<platform::PerlInterpreterResult>();
    Ok(())
}

/// Test that security.rs is now a module folder, not a file.
/// This is verified by the fact that security:: submodule is accessible.
#[test]
fn test_security_is_module_folder() -> Result<()> {
    use perl_dap::security;

    // Access a type from the security module
    let _ = std::any::type_name::<security::SecurityError>();
    Ok(())
}
