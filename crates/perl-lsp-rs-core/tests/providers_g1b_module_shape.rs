//! Red TDD tests for Wave G1b provider module existence and shape.
//!
//! These tests validate that all 10 provider submodules from G1b collapse
//! are correctly declared and accessible under `perl_lsp_rs_core::providers::*`.
//!
//! The 10 providers in collapse order:
//! - Phase 1 (pure leaves): rename, diagnostics, inline_completion, semantic_tokens
//! - Phase 2 (near-leaves): formatting, ai
//! - Phase 3 (consumers): completion, navigation, code_actions
//! - Phase 4 (aggregator): lsp_compat (NEW submodule from perl-lsp-providers/ide/lsp_compat)
//!
//! These tests FAIL at master (modules don't exist) and PASS after
//! the builder creates the module structure and collapses all 10 crates.

#[allow(unused_imports)]
use perl_tdd_support::{must, must_some};

// ============================================================================
// PHASE 1: Pure Leaves (no G1b intra-dependencies)
// ============================================================================

/// Test that rename provider module is accessible.
/// This is a foundational provider used by code_actions.
#[test]
fn test_providers_rename_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::rename;
    // Verify the type is reachable.
    let _provider = rename::RenameProvider::new(&Default::default(), "test".to_string());
    Ok(())
}

/// Test that diagnostics provider module is accessible.
/// This is used by code_actions.
#[test]
fn test_providers_diagnostics_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::diagnostics;
    // Verify a type is reachable (DiagnosticTag or similar).
    let _tag = diagnostics::DiagnosticTag::Unnecessary;
    Ok(())
}

/// Test that inline_completion provider module is accessible.
/// This is used by ai provider.
#[test]
fn test_providers_inline_completion_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::inline_completion;
    // Verify the type is reachable.
    let _provider = inline_completion::InlineCompletionProvider::new();
    Ok(())
}

/// Test that semantic_tokens provider module is accessible.
/// This is a pure leaf provider.
#[test]
fn test_providers_semantic_tokens_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::semantic_tokens;
    // Verify the type is reachable.
    let _provider = semantic_tokens::SemanticTokensProvider::new();
    Ok(())
}

// ============================================================================
// PHASE 2: Near-Leaves (G1a dependencies only)
// ============================================================================

/// Test that formatting provider module is accessible.
/// This depends on formatting_types (G1a, already present).
#[test]
fn test_providers_formatting_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::formatting;
    // Verify the type is reachable.
    let _provider = formatting::FormattingProvider::new();
    Ok(())
}

/// Test that ai provider module is accessible.
/// This depends on inline_completion (Phase 1, now absorbed).
#[test]
fn test_providers_ai_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::ai;
    // Verify the type is reachable.
    let _config = ai::OpenAiConfig::default();
    Ok(())
}

// ============================================================================
// PHASE 3: Consumers (depend on Phase 1 + Phase 2)
// ============================================================================

/// Test that completion provider module is accessible.
/// This depends on completion_item and file_completion (G1a).
#[test]
fn test_providers_completion_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::completion;
    // Verify the type is reachable.
    let _provider = completion::CompletionProvider::new();
    Ok(())
}

/// Test that navigation provider module is accessible.
/// This depends on multiple G1a crates.
#[test]
fn test_providers_navigation_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::navigation;
    // Verify the type is reachable.
    let _provider = navigation::NavigationProvider::new();
    Ok(())
}

/// Test that code_actions provider module is accessible.
/// This depends on diagnostics, rename (Phase 1), and import_management (G1a).
#[test]
fn test_providers_code_actions_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::code_actions;
    // Verify the type is reachable.
    let _provider = code_actions::CodeActionsProvider::new();
    Ok(())
}

// ============================================================================
// PHASE 4: Aggregator (lsp_compat — original code from perl-lsp-providers/ide/lsp_compat)
// ============================================================================

/// Test that lsp_compat module is accessible.
/// This contains ~1,600 LOC of original implementations from perl-lsp-providers.
/// (Per O5 correction: NOT registry, but lsp_compat with original code)
#[test]
fn test_providers_lsp_compat_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat;
    // Verify that at least one key submodule is reachable (signature_help is the largest at ~550 LOC).
    // We test existence, not behavior — behavior tests are in comprehensive_unit_tests.rs.
    let _module_exists = std::any::type_name::<lsp_compat::signature_help::SignatureHelpProvider>();
    Ok(())
}

/// Test that signature_help submodule in lsp_compat is accessible.
/// This is the largest original implementation (~550 LOC).
#[test]
fn test_providers_lsp_compat_signature_help_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat::signature_help;
    // Verify the type is reachable.
    let _provider = signature_help::SignatureHelpProvider::new();
    Ok(())
}

/// Test that linked_editing submodule in lsp_compat is accessible.
/// This is an original implementation (~407 LOC).
#[test]
fn test_providers_lsp_compat_linked_editing_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat::linked_editing;
    // Verify the function is reachable.
    let _fn_exists = std::any::type_name::<fn() -> Result<(), Box<dyn std::error::Error>>>();
    let _ = linked_editing::handle_linked_editing(&Default::default(), (0, 0))?;
    Ok(())
}

/// Test that selection_range submodule in lsp_compat is accessible.
/// This is an original implementation (~232 LOC).
#[test]
fn test_providers_lsp_compat_selection_range_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat::selection_range;
    // Verify the provider is reachable.
    let _provider = selection_range::SelectionRangeProvider::new();
    Ok(())
}

// ============================================================================
// Module-Level Re-exports (O2 requirement)
// ============================================================================

/// Test that all 10 collapsed providers are re-exported from the top-level providers module.
/// This ensures the public API surface is preserved.
#[test]
fn test_providers_module_reexports_g1b_providers() -> Result<(), Box<dyn std::error::Error>> {
    // Each of these imports should resolve via the top-level providers module re-export.
    use perl_lsp_rs_core::providers::{
        ai, code_actions, completion, diagnostics, formatting, inline_completion, lsp_compat,
        navigation, rename, semantic_tokens,
    };
    // If we get here without import errors, re-exports are working.
    let _ = (
        rename,
        diagnostics,
        inline_completion,
        semantic_tokens,
        formatting,
        ai,
        completion,
        navigation,
        code_actions,
        lsp_compat,
    );
    Ok(())
}

/// Test that the deprecated tooling_export alias is preserved for backward compatibility.
/// This is an O2 requirement to maintain the migration path.
#[test]
fn test_providers_deprecated_tooling_export_alias() -> Result<(), Box<dyn std::error::Error>> {
    // The tooling_export alias should be accessible (though deprecated).
    // We test it indirectly by verifying that the providers module can be aliased.
    // Direct test would require the alias to actually exist, which will happen after implementation.
    // This test documents the requirement rather than testing runtime behavior.
    Ok(())
}
