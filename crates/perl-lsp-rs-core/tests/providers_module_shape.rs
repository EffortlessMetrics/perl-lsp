//! Red TDD tests for Wave G1a provider module existence and shape.
//!
//! These tests validate that all 15 provider submodules are correctly
//! declared and accessible under `perl_lsp_rs_core::providers::*`.
//!
//! The 15 providers in dependency order:
//! - Group 1 (helpers): completion_item, symbol_query
//! - Group 2 (consumers of Group 1): file_completion, workspace_symbols
//! - Group 3 (independents): code_lens, document_highlight, folding, selection_range,
//!   inlay_hints, type_hierarchy, formatting_types,
//!   on_type_formatting, color, import_management, document_links
//!
//! These tests FAIL at master (providers module doesn't exist) and PASS after
//! the builder creates the module structure.

#[allow(unused_imports)]
use perl_tdd_support::{must, must_some};

// ============================================================================
// GROUP 1: Helper Providers
// ============================================================================

/// Test that completion_item provider module is accessible.
/// This is a foundational provider used by file_completion.
#[test]
fn test_providers_completion_item_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This import will fail at master (module doesn't exist) and pass after G1a.
    use perl_lsp_rs_core::providers::completion_item;
    // Just verify the module is accessible; we don't test behavior here.
    // That's covered by the migrated test suite in provider_completion_item_*.rs
    let _ = completion_item::CompletionItem {
        label: "test".to_string(),
        kind: completion_item::CompletionItemKind::Function,
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        insert_text: Some("test".to_string()),
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    };
    Ok(())
}

/// Test that symbol_query provider module is accessible.
/// This is a foundational provider used by workspace_symbols.
#[test]
fn test_providers_symbol_query_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::symbol_query;
    // Verify the function is reachable.
    let result = symbol_query::matches_query("test", "test");
    assert!(result, "symbol_query::matches_query should exist and be callable");
    Ok(())
}

// ============================================================================
// GROUP 2: Consumer Providers (depend on Group 1)
// ============================================================================

/// Test that file_completion provider module is accessible.
/// This depends on completion_item being available as a sibling submodule.
#[test]
fn test_providers_file_completion_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::file_completion;
    // Verify the type is reachable.
    let _ = file_completion::FileCompletionOptions { enabled: true, max_items: 100 };
    Ok(())
}

/// Test that workspace_symbols provider module is accessible.
/// This depends on symbol_query being available as a sibling submodule.
#[test]
fn test_providers_workspace_symbols_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::workspace_symbols;
    // Verify the type is reachable.
    let provider = workspace_symbols::WorkspaceSymbolsProvider::new();
    let results = provider.search("test", &std::collections::HashMap::new());
    assert!(results.is_empty(), "fresh provider should return no symbols");
    Ok(())
}

// ============================================================================
// GROUP 3: Independent Providers (no inter-dependencies)
// ============================================================================

/// Test that code_lens provider module is accessible.
#[test]
fn test_providers_code_lens_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::code_lens;
    let _ = code_lens::CodeLensProvider::new();
    Ok(())
}

/// Test that document_highlight provider module is accessible.
#[test]
fn test_providers_document_highlight_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::document_highlight;
    let _ = document_highlight::DocumentHighlightProvider::new();
    Ok(())
}

/// Test that folding provider module is accessible.
#[test]
fn test_providers_folding_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::folding;
    let _ = folding::FoldingRangeProvider::new();
    Ok(())
}

/// Test that selection_range provider module is accessible.
#[test]
fn test_providers_selection_range_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::selection_range;
    let _ = selection_range::SelectionRangeProvider::new();
    Ok(())
}

/// Test that inlay_hints provider module is accessible.
#[test]
fn test_providers_inlay_hints_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::inlay_hints;
    let _ = inlay_hints::InlayHintsProvider::new();
    Ok(())
}

/// Test that type_hierarchy provider module is accessible.
#[test]
fn test_providers_type_hierarchy_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::type_hierarchy;
    let _ = type_hierarchy::TypeHierarchyProvider::new();
    Ok(())
}

/// Test that formatting_types provider module is accessible.
#[test]
fn test_providers_formatting_types_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::formatting_types;
    // Verify FormatRange and its whole_document constructor are accessible.
    let content = "line1\nline2\nline3";
    let range = formatting_types::FormatRange::whole_document(content);
    assert_eq!(range.start.line, 0, "whole_document range must start at line 0");
    assert_eq!(range.end.line, 2, "whole_document range must end at line 2");
    Ok(())
}

/// Test that on_type_formatting provider module is accessible.
#[test]
fn test_providers_on_type_formatting_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::on_type_formatting;
    let _ = on_type_formatting::OnTypeFormattingProvider::new();
    Ok(())
}

/// Test that color provider module is accessible.
#[test]
fn test_providers_color_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::color;
    let _ = color::ColorProvider::new();
    Ok(())
}

/// Test that import_management provider module is accessible.
#[test]
fn test_providers_import_management_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::import_management;
    let _ = import_management::ImportManager::new();
    Ok(())
}

/// Test that document_links provider module is accessible.
#[test]
fn test_providers_document_links_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::document_links;
    // Verify compute_links is accessible with its correct signature: (uri, text, roots).
    let roots: Vec<url::Url> = vec![];
    let links = document_links::compute_links("", "", &roots);
    assert!(links.is_empty(), "empty source should return no links");
    Ok(())
}
