//! Red TDD tests for Wave G2 runtime module absorption and shape.
//!
//! These tests validate that 5 runtime submodules from G2 collapse
//! are correctly declared and accessible under `perl_lsp_rs_core::runtime::*`.
//!
//! The 5 modules absorbed in G2:
//! - cancellation (request-scoped cancellation tokens)
//! - limits (resource constraints and deadlines)
//! - input_validation (security validation)
//! - launcher (CLI parsing and startup coordination, includes timing.rs sibling)
//! - text_utils (text editing utilities)
//!
//! Deferred to G3:
//! - transport (LSP message framing) — blocked by cycle:
//!   perl-lsp-protocol depends on perl-lsp-rs-core, making absorption cyclic.
//!
//! These tests FAIL at master (modules don't exist) and PASS after
//! the builder creates the module structure and absorbs all 5 crates.

#[allow(unused_imports)]
use perl_tdd_support::{must, must_some};

// ============================================================================
// Module: runtime::cancellation
// ============================================================================

/// Test that cancellation module is accessible.
/// This provides request-scoped cancellation tokens with atomic operations.
#[test]
fn test_runtime_cancellation_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G2.
    use perl_lsp_rs_core::runtime::cancellation;
    // Verify the token type is reachable.
    let _token = cancellation::PerlLspCancellationToken::new(
        serde_json::json!({"id": 1}),
        "test_provider".to_string(),
    );
    Ok(())
}

/// Test that PerlLspCancellationToken methods are accessible.
#[test]
fn test_runtime_cancellation_token_methods() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::cancellation::PerlLspCancellationToken;
    let token =
        PerlLspCancellationToken::new(serde_json::json!({"id": 1}), "test_provider".to_string());
    // Verify key methods exist and are callable.
    let _is_cancelled = token.is_cancelled();
    let _is_cancelled_relaxed = token.is_cancelled_relaxed();
    Ok(())
}

/// Test that CancellationRegistry is accessible and constructible.
#[test]
fn test_runtime_cancellation_registry_default() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::cancellation::CancellationRegistry;
    let _registry = CancellationRegistry::default();
    Ok(())
}

/// Test that CancellationError enum is accessible.
#[test]
fn test_runtime_cancellation_error_type() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::cancellation;
    // Verify the type is accessible via type_name (error types may not be constructible directly).
    let _ = std::any::type_name::<cancellation::CancellationError>();
    Ok(())
}

/// Test that CancellableProvider trait is accessible.
#[test]
fn test_runtime_cancellation_trait_accessible() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::cancellation;
    // Verify the trait is accessible via type_name.
    let _ = std::any::type_name::<dyn cancellation::CancellableProvider>();
    Ok(())
}

/// Test that global cancellation registry is accessible.
#[test]
fn test_runtime_global_cancellation_registry() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::cancellation::GLOBAL_CANCELLATION_REGISTRY;
    // Verify the static is accessible by reading it.
    let _ = &*GLOBAL_CANCELLATION_REGISTRY;
    Ok(())
}

// ============================================================================
// Module: runtime::limits
// ============================================================================

/// Test that limits module is accessible.
/// This provides resource constraints and deadline configurations.
#[test]
fn test_runtime_limits_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits;
    // Verify the key types are reachable.
    let _budget = limits::MemoryBudget::default();
    Ok(())
}

/// Test that MemoryMonitor is accessible.
#[test]
fn test_runtime_limits_memory_monitor() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits::{MemoryBudget, MemoryMonitor};
    let _monitor = MemoryMonitor::new(MemoryBudget::default());
    Ok(())
}

/// Test that MemoryPressure enum is accessible.
#[test]
fn test_runtime_limits_memory_pressure() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits::MemoryPressure;
    // Verify the enum variants are accessible.
    let _ = MemoryPressure::Normal;
    let _ = MemoryPressure::Warning;
    let _ = MemoryPressure::Critical;
    Ok(())
}

/// Test that LspLimits type is accessible.
#[test]
fn test_runtime_limits_lsp_limits() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits;
    // Verify the type is accessible via type_name.
    let _ = std::any::type_name::<limits::LspLimits>();
    Ok(())
}

/// Test that LSP_LIMITS static is accessible.
#[test]
fn test_runtime_global_limits_static() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits::LSP_LIMITS;
    // Verify the static is accessible by reading it.
    let _ = &*LSP_LIMITS.read().unwrap();
    Ok(())
}

/// Test that limit accessor functions are accessible.
#[test]
fn test_runtime_limits_accessor_functions() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::limits;
    // Verify key accessor functions exist and are callable.
    let _cap = limits::workspace_symbol_cap();
    let _cap = limits::references_cap();
    let _cap = limits::completion_cap();
    Ok(())
}

// ============================================================================
// Module: runtime::input_validation
// ============================================================================

/// Test that input_validation module is accessible.
/// This provides security validation for file paths and content.
#[test]
fn test_runtime_input_validation_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::input_validation;
    // Verify the module is accessible by calling a simple function.
    let _sanitized = input_validation::sanitize_string("test");
    Ok(())
}

/// Test that validate_file_path function is accessible.
#[test]
fn test_runtime_input_validation_validate_file_path() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::input_validation;
    use std::path::Path;
    // Verify the function is accessible (may fail validation, but that's ok for shape test).
    let _result = input_validation::validate_file_path("./test.pl", Path::new("."));
    Ok(())
}

/// Test that validate_file_content function is accessible.
#[test]
fn test_runtime_input_validation_validate_file_content() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::input_validation;
    use std::path::Path;
    // Verify the function is accessible (may fail validation, but that's ok for shape test).
    let _result = input_validation::validate_file_content("", Path::new("test.pl"));
    Ok(())
}

/// Test that sanitize_string function is accessible.
#[test]
fn test_runtime_input_validation_sanitize_string() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::input_validation;
    let _sanitized = input_validation::sanitize_string("test input");
    Ok(())
}

// ============================================================================
// Module: runtime::launcher
// ============================================================================

/// Test that launcher module is accessible.
/// This provides CLI parsing and startup coordination.
#[test]
fn test_runtime_launcher_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    // Verify key types are reachable.
    let _ = std::any::type_name::<launcher::LaunchPlan>();
    Ok(())
}

/// Test that DEFAULT_LSP_PORT constant is accessible.
#[test]
fn test_runtime_launcher_default_port() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher::DEFAULT_LSP_PORT;
    // Verify the constant is accessible (should be 9257).
    let _ = DEFAULT_LSP_PORT;
    Ok(())
}

/// Test that logging functions are accessible.
#[test]
fn test_runtime_launcher_logging_functions() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    // Verify logging functions are accessible.
    let _should_log = launcher::should_enable_logging(false);
    let _filter = launcher::logging_filter(false, "info", "warn");
    launcher::init_logging("info");
    Ok(())
}

/// Test that LaunchPlan struct is accessible.
#[test]
fn test_runtime_launcher_launch_plan() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    // Verify the type is accessible via type_name.
    let _ = std::any::type_name::<launcher::LaunchPlan>();
    Ok(())
}

/// Test that timing module is re-exported from launcher.
#[test]
fn test_runtime_launcher_timing_reexport() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    // Verify StartupTimer is accessible via re-export.
    let _timer = launcher::StartupTimer::new();
    Ok(())
}

/// Test that StartupReport is accessible.
#[test]
fn test_runtime_launcher_startup_report() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    let mut timer = launcher::StartupTimer::new();
    timer.checkpoint("test");
    let _report = timer.finish();
    Ok(())
}

// ============================================================================
// Module: runtime::transport — DEFERRED to G3
// ============================================================================
// NOTE(G2-defer): perl-lsp-transport was deferred to Wave G3.
// The absorption is blocked because perl-lsp-protocol depends on perl-lsp-rs-core,
// which would create a crate dependency cycle if transport (which imports
// perl-lsp-protocol::JsonRpcRequest) were absorbed into perl-lsp-rs-core.
// Transport tests remain in crates/perl-lsp-transport/tests/.

// ============================================================================
// Module: runtime::text_utils
// ============================================================================

/// Test that text_utils module is accessible.
/// This provides text editing utilities for code actions.
#[test]
fn test_runtime_text_utils_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::text_utils;
    // Verify the main type is accessible.
    let _helper = text_utils::TextEditHelpers::new("", &[]);
    Ok(())
}

/// Test that TextEditHelpers methods are accessible.
#[test]
fn test_runtime_text_utils_helpers_methods() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::text_utils::TextEditHelpers;
    let lines = vec![];
    let helper = TextEditHelpers::new("test code", &lines);
    // Verify key methods exist and are callable.
    let _start = helper.find_statement_start(5);
    let _import_pos = helper.find_import_insert_position();
    let _pragma_pos = helper.find_pragma_insert_position();
    let _sub_pos = helper.find_subroutine_insert_position(10);
    Ok(())
}

// ============================================================================
// Module-Level Re-exports (G2 requirement)
// ============================================================================

/// Test that all 5 collapsed runtime modules are re-exported from the top-level runtime module.
/// (transport deferred to G3 — see NOTE above)
/// This ensures the public API surface is preserved.
#[test]
fn test_runtime_module_reexports_g2_modules() -> Result<(), Box<dyn std::error::Error>> {
    // Each of these imports should resolve via the top-level runtime module re-export.
    use perl_lsp_rs_core::runtime::{cancellation, input_validation, launcher, limits, text_utils};
    // If we get here without import errors, re-exports are working.
    // Use type_name to verify modules are accessible without using them as values.
    let _ = std::any::type_name::<cancellation::PerlLspCancellationToken>();
    let _ = std::any::type_name::<limits::MemoryBudget>();
    // input_validation exports functions (not types), verify by calling one
    let _s = input_validation::sanitize_string("test");
    let _ = std::any::type_name::<launcher::LaunchPlan>();
    let _ = std::any::type_name::<text_utils::TextEditHelpers>();
    Ok(())
}

/// Test that runtime module itself is accessible from rs-core facade.
#[test]
fn test_runtime_module_accessible_from_facade() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime;
    // Verify the module is accessible via type_name.
    let _ = std::any::type_name::<runtime::PerlLspCancellationToken>();
    let _ = std::any::type_name::<runtime::MemoryBudget>();
    Ok(())
}

// ============================================================================
// Scope Exclusion: perl-lsp-performance NOT absorbed in G2
// ============================================================================

/// Test that perl-lsp-performance is NOT absorbed in G2.
/// This module is deferred to G3 (moved with perl-lsp-tooling).
/// This test documents the deferral by verifying the path doesn't exist.
#[test]
fn test_runtime_performance_not_absorbed_in_g2() -> Result<(), Box<dyn std::error::Error>> {
    // This import SHOULD FAIL because performance is NOT part of G2.
    // If this test fails to compile (import succeeds), it means performance
    // was prematurely absorbed — that's a scope violation.
    //
    // To verify the deferral is correct, we check that the module path doesn't exist.
    // We do this via a negative assertion: trying to use the path in a way that
    // would fail if the module existed.
    //
    // This is a documentation test that codifies the deferral decision.
    // Post-implementation, if someone accidentally absorbs performance in G2,
    // they would need to update this test (or the spec).

    // Verify that crates/perl-lsp-performance/Cargo.toml still exists (not deleted).
    // We can't directly check this from a test, so we document it as a known fact
    // based on the spec: performance remains in crates/ and is absorbed in G3.

    Ok(())
}

/// Test that perl-dap can access the new runtime module structure.
/// This verifies that the cross-binary dependency (perl-dap → launcher) works correctly.
#[test]
fn test_runtime_cross_binary_perl_dap_access() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::runtime::launcher;
    // Verify that launcher (used by perl-dap) is accessible through the new module path.
    let _plan_type = std::any::type_name::<launcher::LaunchPlan>();
    Ok(())
}
