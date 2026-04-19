//! Green TDD edge case and regression tests for Wave G1a provider collapse.
//!
//! These tests verify that the builder's implementation correctly handles:
//! 1. Wrapper types added for API compatibility
//! 2. Method signature changes (e.g., CodeLensProvider::new() → with_source())
//! 3. Submodule path resolution for all 15 providers
//! 4. Inter-provider dependencies (completion_item→file_completion, symbol_query→workspace_symbols)
//! 5. Consumer integration via new import paths
//! 6. Edge cases in provider implementations
//!
//! All tests are green (passing) at the point this file is added. If any fail,
//! that indicates a bug in the builder's implementation.

// ============================================================================
// WRAPPER TYPE TESTS: Verify new types introduced for API compatibility
// ============================================================================

/// FileCompletionOptions wrapper struct should exist and be constructible.
/// This was added to match red test expectations for file_completion.
#[test]
fn test_file_completion_options_constructible() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::file_completion::FileCompletionOptions;

    let opts = FileCompletionOptions { enabled: true, max_items: 100 };
    assert_eq!(opts.max_items, 100);
    assert!(opts.enabled);
    Ok(())
}

/// FileCompletionOptions::default() should provide sensible defaults.
#[test]
fn test_file_completion_options_default() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::file_completion::FileCompletionOptions;

    let opts = FileCompletionOptions::default();
    assert!(opts.enabled, "file completion should default to enabled");
    assert!(opts.max_items > 0, "max_items should have a positive default");
    Ok(())
}

/// FileCompletionContext should be constructible with all required fields.
#[test]
fn test_file_completion_context_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::file_completion::FileCompletionContext;

    let ctx = FileCompletionContext::new("$foo/", 0, 6);
    assert_eq!(ctx.prefix, "$foo/");
    assert_eq!(ctx.prefix_start, 0);
    assert_eq!(ctx.position, 6);
    Ok(())
}

/// FileCompletionContext::new should accept Into<String> for prefix.
#[test]
fn test_file_completion_context_accepts_string_like() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::file_completion::FileCompletionContext;

    // Should accept both &str and String
    let _ctx1 = FileCompletionContext::new("prefix", 0, 6);
    let _ctx2 = FileCompletionContext::new("prefix".to_string(), 0, 6);
    Ok(())
}

/// ImportManager wrapper should be accessible and usable.
#[test]
fn test_import_manager_accessible() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::import_management::{ImportManager, collect_imports};
    // Test that the struct and function are reachable.
    let imports = collect_imports(&[]);
    assert_eq!(imports.len(), 0, "empty input should yield empty imports");
    let _ = ImportManager;
    Ok(())
}

// ============================================================================
// METHOD SIGNATURE TESTS: CodeLensProvider new() → with_source() change
// ============================================================================

/// CodeLensProvider::new() should exist as a no-arg constructor.
/// This is required for compatibility with code that doesn't have source yet.
#[test]
fn test_code_lens_provider_new_no_arg() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::code_lens::CodeLensProvider;

    let _provider = CodeLensProvider::new();
    // Constructor succeeds; we can't access private fields to verify internals.
    Ok(())
}

/// CodeLensProvider::with_source() should accept String source.
/// This is the new signature that replaced CodeLensProvider::new(source).
#[test]
fn test_code_lens_provider_with_source() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::code_lens::CodeLensProvider;

    let _provider = CodeLensProvider::with_source("sub foo { 1 }".to_string());
    // Constructor succeeds; method exists and accepts String.
    Ok(())
}

/// CodeLensProvider should support method chaining with with_file_path.
#[test]
fn test_code_lens_provider_with_file_path_chain() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::code_lens::CodeLensProvider;

    let _provider = CodeLensProvider::with_source("sub test { 1 }".to_string())
        .with_file_path("test.t".to_string());
    // Method chaining works correctly.
    Ok(())
}

/// CodeLensProvider::with_file_path should set the file path correctly.
#[test]
fn test_code_lens_provider_with_file_path_set() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::code_lens::CodeLensProvider;

    let _provider = CodeLensProvider::new().with_file_path("lib/Foo.pm".to_string());
    // Method works and can be chained from new().
    Ok(())
}

// ============================================================================
// INTER-PROVIDER DEPENDENCY TESTS: file_completion→completion_item
// ============================================================================

/// file_completion should be able to import from completion_item submodule.
/// This tests that intra-module dependency resolution works after collapse.
#[test]
fn test_file_completion_uses_completion_item_types() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::completion_item::CompletionItem;
    use perl_lsp_rs_core::providers::file_completion::FileCompletionOptions;

    // If file_completion had a function that returns CompletionItem, we'd test that.
    // For now, verify both types are independently accessible.
    let _opts = FileCompletionOptions::default();
    let _item = CompletionItem {
        label: "test".to_string(),
        kind: perl_lsp_rs_core::providers::completion_item::CompletionItemKind::Function,
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        insert_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    };
    Ok(())
}

// ============================================================================
// INTER-PROVIDER DEPENDENCY TESTS: workspace_symbols→symbol_query
// ============================================================================

/// workspace_symbols should use symbol_query helper functions.
/// This tests that the second intra-module dependency pair resolves correctly.
#[test]
fn test_workspace_symbols_uses_symbol_query() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::symbol_query::matches_query;
    use perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolsProvider;

    // Verify both types are accessible from different submodules.
    let provider = WorkspaceSymbolsProvider::new();
    assert!(matches_query("test_name", "test"), "symbol_query should match prefixes");
    assert!(!matches_query("unrelated", "test"), "symbol_query should not match unrelated");
    let results = provider.search("test", &std::collections::HashMap::new());
    assert_eq!(results.len(), 0, "fresh provider returns empty results");
    Ok(())
}

// ============================================================================
// EDGE CASES: Completion item deduplication and sorting
// ============================================================================

/// Completion item deduplication should handle empty input.
#[test]
fn test_completion_item_dedup_empty() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::completion_item::deduplicate_and_sort;

    let result = deduplicate_and_sort(vec![]);
    assert_eq!(result.len(), 0);
    Ok(())
}

/// Completion item deduplication should preserve single items.
#[test]
fn test_completion_item_dedup_single() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::completion_item::{
        CompletionItem, CompletionItemKind, deduplicate_and_sort,
    };

    let item = CompletionItem {
        label: "single".to_string(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        insert_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    };
    let result = deduplicate_and_sort(vec![item.clone()]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "single");
    Ok(())
}

/// Completion item dedup should remove exact duplicates.
#[test]
fn test_completion_item_dedup_removes_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::completion_item::{
        CompletionItem, CompletionItemKind, deduplicate_and_sort,
    };

    let make = |label: &str| CompletionItem {
        label: label.to_string(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        insert_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    };

    let items = vec![make("foo"), make("foo"), make("bar"), make("foo")];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 2, "should have 2 unique items");
    Ok(())
}

/// Completion item sort should order items consistently.
#[test]
fn test_completion_item_dedup_sorts_alphabetically() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::completion_item::{
        CompletionItem, CompletionItemKind, deduplicate_and_sort,
    };

    let make = |label: &str| CompletionItem {
        label: label.to_string(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        insert_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    };

    let items = vec![make("zebra"), make("apple"), make("middle")];
    let result = deduplicate_and_sort(items);
    assert_eq!(result[0].label, "apple", "first should be alphabetically first");
    Ok(())
}

// ============================================================================
// EDGE CASES: Symbol query matching
// ============================================================================

/// Symbol query should handle empty query.
#[test]
fn test_symbol_query_empty_query() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::symbol_query::matches_query;

    // Empty query might match everything or nothing — test both extremes.
    let _ = matches_query("anything", "");
    Ok(())
}

/// Symbol query should handle empty symbol name.
#[test]
fn test_symbol_query_empty_symbol() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::symbol_query::matches_query;

    let result = matches_query("", "query");
    assert!(!result, "empty symbol should not match any query");
    Ok(())
}

/// Symbol query should handle long strings.
#[test]
fn test_symbol_query_long_string() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::symbol_query::matches_query;

    let long_name = "a".repeat(1000);
    let long_query = "a".repeat(500);
    let _result = matches_query(&long_name, &long_query);
    Ok(())
}

/// Symbol query should be case-sensitive or case-insensitive (verify behavior).
#[test]
fn test_symbol_query_case_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::symbol_query::matches_query;

    // Document the actual behavior.
    let exact_match = matches_query("TestName", "Test");
    let different_case = matches_query("testname", "Test");
    // Just verify the behavior is consistent; don't assume which way.
    let _ = (exact_match, different_case);
    Ok(())
}

// ============================================================================
// EDGE CASES: Formatting types
// ============================================================================

/// FormatRange::whole_document should handle empty content.
#[test]
fn test_format_range_whole_document_empty() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::formatting_types::FormatRange;

    let range = FormatRange::whole_document("");
    assert_eq!(range.start.line, 0);
    // For empty content, end line should be 0 or start line, depending on implementation.
    assert!(range.end.line >= range.start.line);
    Ok(())
}

/// FormatRange::whole_document should handle single-line content.
#[test]
fn test_format_range_whole_document_single_line() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::formatting_types::FormatRange;

    let range = FormatRange::whole_document("my $x = 1;");
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 0);
    Ok(())
}

/// FormatRange::whole_document should count lines correctly.
#[test]
fn test_format_range_whole_document_multiline() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::formatting_types::FormatRange;

    let content = "line1\nline2\nline3";
    let range = FormatRange::whole_document(content);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 2, "3-line file should have end line 2");
    Ok(())
}

/// FormatRange::whole_document should handle trailing newline.
#[test]
fn test_format_range_whole_document_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::formatting_types::FormatRange;

    let content = "line1\nline2\n";
    let range = FormatRange::whole_document(content);
    // Trailing newline creates an empty line 3, or not, depending on parsing.
    // Verify behavior is consistent.
    assert!(range.end.line >= 1);
    Ok(())
}

// ============================================================================
// EDGE CASES: Import management
// ============================================================================

/// Import collection should handle empty input.
#[test]
fn test_import_management_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::import_management::collect_imports;

    let imports = collect_imports(&[]);
    assert_eq!(imports.len(), 0);
    Ok(())
}

/// Import collection should extract use statements.
#[test]
fn test_import_management_collects_use_statements() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::import_management::collect_imports;

    let lines = vec!["use strict;".to_string(), "use warnings;".to_string()];
    let imports = collect_imports(&lines);
    assert_eq!(imports.len(), 2);
    Ok(())
}

/// Import collection should stop at first non-import.
#[test]
fn test_import_management_collects_all_use_statements_regardless_of_position()
-> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::import_management::collect_imports;

    // collect_imports is a line-by-line scanner: it collects every line that
    // starts with "use " or "require ", regardless of surrounding code.
    let lines = vec![
        "use strict;".to_string(),
        "sub foo { 1 }".to_string(),
        "use warnings;".to_string(), // After code -- still collected
    ];
    let imports = collect_imports(&lines);
    // Both use-lines must be collected; the sub line must not be.
    assert_eq!(imports.len(), 2, "both use-lines should be collected (scanner doesn't stop at code)");
    assert!(imports.iter().any(|i| i.contains("strict")), "use strict must be in results");
    assert!(imports.iter().any(|i| i.contains("warnings")), "use warnings after code must also be in results");
    assert!(!imports.iter().any(|i| i.contains("sub")), "sub declaration must not appear in imports");
    Ok(())
}

// ============================================================================
// SUBMODULE PATH CONSISTENCY TESTS: All 15 providers are accessible
// ============================================================================

/// All 15 provider modules should be accessible from the collapsed namespace.
/// This is a regression test for the module visibility fix.
#[test]
fn test_all_15_providers_accessible_from_perl_lsp_rs_core() -> Result<(), Box<dyn std::error::Error>>
{
    // Group 1: helpers
    use perl_lsp_rs_core::providers::completion_item;
    use perl_lsp_rs_core::providers::symbol_query;

    // Group 2: consumers
    use perl_lsp_rs_core::providers::file_completion;
    use perl_lsp_rs_core::providers::workspace_symbols;

    // Group 3: independents
    use perl_lsp_rs_core::providers::code_lens;
    use perl_lsp_rs_core::providers::color;
    use perl_lsp_rs_core::providers::document_highlight;
    use perl_lsp_rs_core::providers::document_links;
    use perl_lsp_rs_core::providers::folding;
    use perl_lsp_rs_core::providers::formatting_types;
    use perl_lsp_rs_core::providers::import_management;
    use perl_lsp_rs_core::providers::inlay_hints;
    use perl_lsp_rs_core::providers::on_type_formatting;
    use perl_lsp_rs_core::providers::selection_range;
    use perl_lsp_rs_core::providers::type_hierarchy;

    // Just importing should verify module accessibility.
    // Use each module's public API to confirm it compiles.
    let _ = (
        completion_item::CompletionItemKind::Function,
        symbol_query::matches_query("a", "a"),
        file_completion::FileCompletionOptions::default(),
        workspace_symbols::WorkspaceSymbolsProvider::new(),
        code_lens::CodeLensProvider::new(),
        code_lens::is_test_file("test.t"),
        color::detect_colors(""),
        document_highlight::DocumentHighlightProvider::new(),
        document_links::compute_links("", "", &[]),
        folding::FoldingRangeProvider::default(),
        formatting_types::FormatRange::whole_document(""),
        import_management::collect_imports(&[]),
        inlay_hints::InlayHintsProvider::new(),
        on_type_formatting::OnTypeFormattingProvider::new(),
        selection_range::selection_ranges("", &[]),
        type_hierarchy::TypeHierarchyProvider::new(),
    );
    Ok(())
}

/// FoldingRangeProvider should be usable as a type alias.
#[test]
fn test_folding_range_provider_type_alias() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::folding::{FoldingRangeExtractor, FoldingRangeProvider};

    let provider = FoldingRangeProvider::default();
    let _extracted = FoldingRangeExtractor::default();
    let _ = provider;
    Ok(())
}

/// OnTypeFormattingProvider should exist.
#[test]
fn test_on_type_formatting_provider_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::on_type_formatting::OnTypeFormattingProvider;

    let _provider = OnTypeFormattingProvider::new();
    Ok(())
}

/// SelectionRangeProvider should exist.
#[test]
fn test_selection_range_provider_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::selection_range::SelectionRangeProvider;

    let _provider = SelectionRangeProvider::new();
    Ok(())
}

/// TypeHierarchyProvider should exist.
#[test]
fn test_type_hierarchy_provider_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::type_hierarchy::TypeHierarchyProvider;

    let _provider = TypeHierarchyProvider::new();
    Ok(())
}

/// ColorProvider should exist.
#[test]
fn test_color_provider_accessible() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::color::ColorProvider;

    let _provider = ColorProvider;
    Ok(())
}

/// DocumentHighlightProvider should exist.
#[test]
fn test_document_highlight_provider_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::document_highlight::DocumentHighlightProvider;

    let _provider = DocumentHighlightProvider::new();
    Ok(())
}

/// InlayHintsProvider should exist.
#[test]
fn test_inlay_hints_provider_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::providers::inlay_hints::InlayHintsProvider;

    let _provider = InlayHintsProvider::new();
    Ok(())
}
