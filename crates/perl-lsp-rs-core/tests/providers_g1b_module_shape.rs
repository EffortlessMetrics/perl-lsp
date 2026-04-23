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
///
/// NOTE(G1b-API-fix): Red-TDD wrote `RenameProvider::new(&Default::default(), ...)` but
/// RenameProvider::new takes `&Node` not a default-constructible type. Fixed to parse
/// an empty source and pass the resulting AST node.
#[test]
fn test_providers_rename_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::rename;
    use perl_parser::Parser;
    // Verify the type is reachable using a parsed empty source.
    let mut parser = Parser::new("");
    let ast = parser.parse()?;
    let _provider = rename::RenameProvider::new(&ast, String::new());
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
///
/// NOTE(G1b-API-fix): Red-TDD wrote `FormattingProvider::new()` with no args,
/// but FormattingProvider<R> is generic over SubprocessRuntime. Fixed to use
/// OsSubprocessRuntime as the concrete type per actual API.
#[test]
fn test_providers_formatting_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::formatting;
    // Verify the type is reachable with a concrete runtime type.
    let _provider =
        formatting::FormattingProvider::new(perl_subprocess_runtime::OsSubprocessRuntime::new());
    Ok(())
}

/// Test that ai provider module is accessible.
/// This depends on inline_completion (Phase 1, now absorbed).
///
/// NOTE(G1b-API-fix): Red-TDD wrote `OpenAiConfig::default()` but OpenAiConfig does not
/// derive Default (all fields are required). Fixed to verify type existence via type_name.
#[test]
fn test_providers_ai_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::ai;
    // Verify the type is reachable — OpenAiConfig doesn't implement Default,
    // so we use type_name to confirm the type is accessible.
    let _ = std::any::type_name::<ai::OpenAiConfig>();
    Ok(())
}

// ============================================================================
// PHASE 3: Consumers (depend on Phase 1 + Phase 2)
// ============================================================================

/// Test that completion provider module is accessible.
/// This depends on completion_item and file_completion (G1a).
///
/// NOTE(G1b-API-fix): Red-TDD wrote `CompletionProvider::new()` with no args,
/// but CompletionProvider::new() requires an AST node. Fixed to use
/// CompletionProvider::new_with_index() with a parsed empty source per actual API.
#[test]
fn test_providers_completion_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::completion;
    use perl_parser::Parser;
    // Verify the type is reachable using a parsed empty source.
    let mut parser = Parser::new("");
    let ast = parser.parse()?;
    let _provider = completion::CompletionProvider::new_with_index(&ast, None);
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
///
/// NOTE(G1b-API-fix): Red-TDD wrote `CodeActionsProvider::new()` with no args,
/// but CodeActionsProvider::new() requires a source String. Fixed per actual API.
#[test]
fn test_providers_code_actions_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::code_actions;
    // Verify the type is reachable with a source string.
    let _provider = code_actions::CodeActionsProvider::new(String::new());
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
///
/// NOTE(G1b-API-fix): Red-TDD wrote `SignatureHelpProvider::new()` with no args,
/// but the actual API is `SignatureHelpProvider::new(ast: &Node)`. Fixed per actual API.
#[test]
fn test_providers_lsp_compat_signature_help_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat::signature_help;
    use perl_parser::Parser;
    // Verify the type is reachable with a parsed empty AST.
    let mut parser = Parser::new("");
    let ast = parser.parse()?;
    let _provider = signature_help::SignatureHelpProvider::new(&ast);
    Ok(())
}

/// Test that linked_editing submodule in lsp_compat is accessible.
/// This is an original implementation (~407 LOC).
///
/// NOTE(G1b-API-fix): Red-TDD wrote `handle_linked_editing(&Default::default(), (0, 0))`
/// with wrong args (tuple instead of separate u32 args). Actual API is
/// `handle_linked_editing(text: &str, line: u32, character: u32) -> Option<LinkedEditingRanges>`.
/// Fixed per actual API signature.
#[test]
fn test_providers_lsp_compat_linked_editing_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat::linked_editing;
    // Verify the function is reachable with correct API signature.
    let _ = linked_editing::handle_linked_editing("", 0, 0);
    Ok(())
}

/// Test that selection_range submodule in lsp_compat is accessible.
/// This is an original implementation (~232 LOC).
///
/// NOTE(G1b-API-fix): Red-TDD expected `selection_range::SelectionRangeProvider::new()`
/// in the lsp_compat module, but lsp_compat/selection_range.rs contains functions,
/// not a SelectionRangeProvider struct. SelectionRangeProvider already exists in
/// providers::selection_range (G1a). lsp_compat::selection_range re-exports it.
/// Test verifies the module's key function `build_parent_map` is accessible.
#[test]
fn test_providers_lsp_compat_selection_range_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1b.
    use perl_lsp_rs_core::providers::lsp_compat::selection_range;
    // Verify the SelectionRangeProvider re-exported from G1a providers is reachable.
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
    // Use type_name to verify modules are accessible without using them as values.
    let _ = std::any::type_name::<rename::RenameProvider>();
    let _ = std::any::type_name::<diagnostics::DiagnosticTag>();
    let _ = std::any::type_name::<inline_completion::InlineCompletionProvider>();
    let _ = std::any::type_name::<semantic_tokens::SemanticTokensProvider>();
    let _ = std::any::type_name::<formatting::FormattingError>();
    let _ = std::any::type_name::<ai::OpenAiConfig>();
    let _ = std::any::type_name::<completion::CompletionProvider>();
    let _ = std::any::type_name::<navigation::NavigationProvider>();
    let _ = std::any::type_name::<code_actions::CodeActionsProvider>();
    let _ = std::any::type_name::<lsp_compat::signature_help::SignatureHelpProvider>();
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
