//! Green TDD: API stability regression tests for Wave G2 runtime modules.
//!
//! These tests ensure that the public API surfaces of the 5 absorbed runtime
//! modules remain stable and accessible at their original visibility levels.
//!
//! Risk context: When moving code from separate crates into submodules,
//! visibility semantics can change unexpectedly:
//! - Items that were `pub` in a crate remain `pub` in a module
//! - But test code that imported private items might break
//! - The module re-exports (pub use) must preserve the API
//!
//! These tests guard against accidental visibility regressions by testing
//! that key types and functions remain publicly accessible.
//!
//! All tests are green at HEAD (post-G2).

use perl_lsp_rs_core::runtime::cancellation::{
    CancellationError, CancellationRegistry, CancellableProvider, PerlLspCancellationToken,
};
use perl_lsp_rs_core::runtime::input_validation::{
    sanitize_string, validate_file_content, validate_file_path, validate_lsp_request,
};
use perl_lsp_rs_core::runtime::launcher::{
    DEFAULT_LSP_PORT, LaunchAction, LaunchConfig, LaunchParseError, LaunchPlan, TransportArgs,
    TransportMode,
};
use perl_lsp_rs_core::runtime::limits::{
    LspLimits, MemoryBudget, MemoryMonitor, MemoryPressure, LSP_LIMITS,
};
use perl_lsp_rs_core::runtime::text_utils::TextEditHelpers;
use std::path::Path;

/// Test that PerlLspCancellationToken is publicly accessible.
#[test]
fn test_api_cancellation_token_public() -> Result<(), Box<dyn std::error::Error>> {
    // If this compiles, the type is public
    let _token = PerlLspCancellationToken::new(
        serde_json::json!({"id": 1}),
        "test_provider".to_string(),
    );
    Ok(())
}

/// Test that CancellationRegistry is publicly accessible.
#[test]
fn test_api_cancellation_registry_public() -> Result<(), Box<dyn std::error::Error>> {
    let _registry = CancellationRegistry::default();
    Ok(())
}

/// Test that CancellableProvider trait is publicly accessible.
#[test]
fn test_api_cancellable_provider_trait_public() -> Result<(), Box<dyn std::error::Error>> {
    // Verify the trait is accessible via type_name
    let _ = std::any::type_name::<dyn CancellableProvider>();
    Ok(())
}

/// Test that CancellationError enum is publicly accessible.
#[test]
fn test_api_cancellation_error_enum_public() -> Result<(), Box<dyn std::error::Error>> {
    // Verify enum is accessible via type_name
    let _ = std::any::type_name::<CancellationError>();
    Ok(())
}

/// Test that MemoryBudget struct is publicly accessible.
#[test]
fn test_api_memory_budget_public() -> Result<(), Box<dyn std::error::Error>> {
    let _budget = MemoryBudget::default();
    Ok(())
}

/// Test that MemoryMonitor struct is publicly accessible.
#[test]
fn test_api_memory_monitor_public() -> Result<(), Box<dyn std::error::Error>> {
    let _monitor = MemoryMonitor::new(MemoryBudget::default());
    Ok(())
}

/// Test that MemoryPressure enum variants are publicly accessible.
#[test]
fn test_api_memory_pressure_enum_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = MemoryPressure::Normal;
    let _ = MemoryPressure::Warning;
    let _ = MemoryPressure::Critical;
    Ok(())
}

/// Test that LspLimits type is publicly accessible.
#[test]
fn test_api_lsp_limits_type_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::any::type_name::<LspLimits>();
    Ok(())
}

/// Test that LSP_LIMITS static is publicly accessible.
#[test]
fn test_api_lsp_limits_static_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = &*LSP_LIMITS.read().unwrap();
    Ok(())
}

/// Test that input validation functions are publicly accessible.
#[test]
fn test_api_input_validation_functions_public() -> Result<(), Box<dyn std::error::Error>> {
    let _sanitized = sanitize_string("test");
    let _result = validate_file_path("./test.pl", Path::new("."));
    let _result = validate_file_content("", Path::new("test.pl"));
    let _result = validate_lsp_request("initialize", &serde_json::json!({}));
    Ok(())
}

/// Test that LaunchPlan struct is publicly accessible.
#[test]
fn test_api_launch_plan_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::any::type_name::<LaunchPlan>();
    Ok(())
}

/// Test that LaunchAction enum is publicly accessible.
#[test]
fn test_api_launch_action_enum_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = LaunchAction::Run;
    Ok(())
}

/// Test that LaunchConfig struct is publicly accessible.
#[test]
fn test_api_launch_config_public() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher::FeatureProfile;
    let config = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::Production,
    };
    let _ = config.transport;
    Ok(())
}

/// Test that TransportMode enum is publicly accessible.
#[test]
fn test_api_transport_mode_enum_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = TransportMode::Stdio;
    let _ = TransportMode::Socket { port: 9257 };
    Ok(())
}

/// Test that TransportArgs struct is publicly accessible.
#[test]
fn test_api_transport_args_public() -> Result<(), Box<dyn std::error::Error>> {
    // TransportArgs is a CLI parsing struct; verify type is accessible
    let _ = std::any::type_name::<TransportArgs>();
    Ok(())
}

/// Test that LaunchParseError enum is publicly accessible.
#[test]
fn test_api_launch_parse_error_enum_public() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::any::type_name::<LaunchParseError>();
    Ok(())
}

/// Test that DEFAULT_LSP_PORT constant is publicly accessible.
#[test]
fn test_api_default_lsp_port_constant_public() -> Result<(), Box<dyn std::error::Error>> {
    let _port = DEFAULT_LSP_PORT;
    assert_eq!(_port, 9257);
    Ok(())
}

/// Test that TextEditHelpers struct is publicly accessible.
#[test]
fn test_api_text_edit_helpers_public() -> Result<(), Box<dyn std::error::Error>> {
    let _helper = TextEditHelpers::new("test", &[]);
    Ok(())
}

/// Test that CancellationToken methods are publicly accessible and callable.
#[test]
fn test_api_cancellation_token_methods_public() -> Result<(), Box<dyn std::error::Error>> {
    let token = PerlLspCancellationToken::new(
        serde_json::json!({"id": 1}),
        "test_provider".to_string(),
    );
    let _is_cancelled = token.is_cancelled();
    let _is_cancelled_relaxed = token.is_cancelled_relaxed();
    Ok(())
}

/// Test that limit accessor functions are publicly accessible.
#[test]
fn test_api_limit_accessor_functions_public() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits;
    let _workspace_cap = limits::workspace_symbol_cap();
    let _refs_cap = limits::references_cap();
    let _completion_cap = limits::completion_cap();
    Ok(())
}

/// Test that TextEditHelpers methods are publicly accessible.
#[test]
fn test_api_text_edit_helpers_methods_public() -> Result<(), Box<dyn std::error::Error>> {
    let helper = TextEditHelpers::new("test code", &[]);
    let _start = helper.find_statement_start(5);
    let _import_pos = helper.find_import_insert_position();
    let _pragma_pos = helper.find_pragma_insert_position();
    let _sub_pos = helper.find_subroutine_insert_position(10);
    Ok(())
}

/// Test that re-exports from runtime module work (pub use).
/// Ensures items can be accessed both directly and via re-export.
#[test]
fn test_api_runtime_reexports_work() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime;
    let _token = runtime::PerlLspCancellationToken::new(
        serde_json::json!({"id": 1}),
        "test_provider".to_string(),
    );
    let _budget = runtime::MemoryBudget::default();
    Ok(())
}

/// Test that sibling module content is accessible (launcher/timing.rs re-export).
#[test]
fn test_api_launcher_timing_reexport() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    let _timer = launcher::StartupTimer::new();
    Ok(())
}
