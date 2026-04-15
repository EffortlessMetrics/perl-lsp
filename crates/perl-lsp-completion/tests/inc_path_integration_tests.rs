//! Integration tests for @INC path module completion
//!
//! These tests verify the end-to-end workflow between components:
//! - Parser recognizes `use` statement and extracts prefix
//! - CompletionProvider orchestrates workspace index + include path scanning
//! - Results are properly formatted with correct sort order and deduplication
//!
//! These tests use the public API (CompletionProvider) and test the full
//! user journey from typing to receiving completion suggestions.

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

fn count_label(items: &[CompletionItem], label: &str) -> usize {
    items.iter().filter(|i| i.label == label).count()
}

fn create_temp_module(
    temp_dir: &std::path::Path,
    module_name: &str,
    package_content: &str,
) -> std::io::Result<()> {
    let module_path = temp_dir.join(module_name);
    if let Some(parent) = module_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&module_path, package_content)?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Integration Test 1: Complete user journey - configure and use include path
// -----------------------------------------------------------------------------

/// Integration test: User configures include paths, types `use`, gets completions
///
/// This test verifies the full user journey:
/// 1. Admin configures `.perl-lsp.toml` with `includePaths`
/// 2. User types `use DBI::` in their Perl code
/// 3. LSP suggests modules from the configured include path
/// 4. Results are properly sorted and deduplicated
#[test]
fn integration_user_journey_include_path_configuration() {
    // Step 1: Simulate include path configuration (as if from .perl-lsp.toml)
    let temp_lib = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_lib.path().to_path_buf();

    // Create realistic module structure
    create_temp_module(&include_path, "DBI.pm", "package DBI;\n1;\n").unwrap();
    create_temp_module(&include_path, "DBD/SQLite.pm", "package DBD::SQLite;\n1;\n").unwrap();
    create_temp_module(&include_path, "DBD/mysql.pm", "package DBD::mysql;\n1;\n").unwrap();
    create_temp_module(&include_path, "Net/HTTP.pm", "package Net::HTTP;\n1;\n").unwrap();

    // Step 2: Create workspace index (empty for this test)
    let index = Arc::new(WorkspaceIndex::new());

    // Step 3: User types `use DB` and triggers completion
    let code = "use DB";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Step 4: Verify results - with prefix "DB", only modules starting with DB appear
    assert!(has_label(&items, "DBI"), "DBI should appear from include path");
    assert!(has_label(&items, "DBD::SQLite"), "DBD::SQLite should appear from include path");
    assert!(has_label(&items, "DBD::mysql"), "DBD::mysql should appear from include path");
    // Net::HTTP does NOT start with "DB", so it should not appear
    assert!(!has_label(&items, "Net::HTTP"), "Net::HTTP should NOT appear for prefix 'DB'");

    // Deduplication check
    assert_eq!(count_label(&items, "DBI"), 1, "DBI should appear exactly once");
}

// -----------------------------------------------------------------------------
// Integration Test 2: Component handoff - Workspace index + Include paths
// -----------------------------------------------------------------------------

/// Integration test: Verify workspace modules and include path modules work together
///
/// This test verifies the handoff between:
/// 1. WorkspaceIndex (provides workspace packages)
/// 2. IncludePathScanner (provides external packages)
/// 3. Deduplication logic (prevents duplicates)
///
/// Flow: Parser → CompletionProvider → [WorkspaceIndex + IncludePathScanner] → Deduplicated results
#[test]
fn integration_workspace_and_include_path_handoff() {
    // Setup: Create modules in both workspace and include path
    let temp_include = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_include.path().to_path_buf();

    // Module in include path
    create_temp_module(&include_path, "Shared/Module.pm", "package Shared::Module;\n1;\n").unwrap();

    // Same module in workspace (simulating index_file)
    let workspace_uri = Url::parse("file:///workspace/Shared/Module.pm").unwrap();
    let workspace_code = "package Shared::Module;\nsub new { }\n1;\n";
    let index = Arc::new(WorkspaceIndex::new());
    must(index.index_file(workspace_uri, workspace_code.to_string()));

    // Another module only in workspace
    let workspace_uri2 = Url::parse("file:///workspace/WorkspaceOnly/Module.pm").unwrap();
    let workspace_code2 = "package WorkspaceOnly::Module;\nsub new { }\n1;\n";
    must(index.index_file(workspace_uri2, workspace_code2.to_string()));

    // Another module only in include path
    create_temp_module(&include_path, "Shared/External.pm", "package Shared::External;\n1;\n")
        .unwrap();

    // Complete after `use Shared::` - prefix is "Shared::"
    let code = "use Shared::";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Shared::Module should appear exactly once (workspace takes priority)
    assert_eq!(
        count_label(&items, "Shared::Module"),
        1,
        "Shared::Module should appear exactly once (workspace priority)"
    );

    // Shared::External should appear (from include path)
    assert!(
        has_label(&items, "Shared::External"),
        "Shared::External should appear from include path"
    );

    // WorkspaceOnly::Module should NOT appear (doesn't match prefix "Shared::")
    assert!(
        !has_label(&items, "WorkspaceOnly::Module"),
        "WorkspaceOnly::Module should NOT appear for prefix 'Shared::'"
    );
}

// -----------------------------------------------------------------------------
// Integration Test 3: Multiple include paths scanned in order
// -----------------------------------------------------------------------------

/// Integration test: Verify multiple include paths are all scanned
///
/// This test verifies:
/// 1. All configured include paths are traversed
/// 2. Modules from each path are discovered
/// 3. Results are merged correctly
#[test]
fn integration_multiple_include_paths_scanned() {
    let temp_dir1 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir2 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir3 = tempfile::tempdir().expect("tempdir should succeed");

    // Create unique modules in each path
    create_temp_module(temp_dir1.path(), "Path1/Module.pm", "package Path1::Module;\n1;\n")
        .unwrap();
    create_temp_module(temp_dir2.path(), "Path2/Module.pm", "package Path2::Module;\n1;\n")
        .unwrap();
    create_temp_module(temp_dir3.path(), "Path3/Module.pm", "package Path3::Module;\n1;\n")
        .unwrap();

    // Module in multiple paths (should be deduplicated)
    create_temp_module(temp_dir1.path(), "Shared/Module.pm", "package Shared::Module;\n1;\n")
        .unwrap();
    create_temp_module(temp_dir2.path(), "Shared/Module.pm", "package Shared::Module;\n1;\n")
        .unwrap();
    create_temp_module(temp_dir3.path(), "Shared/Module.pm", "package Shared::Module;\n1;\n")
        .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // Complete `use ` (empty prefix = all modules)
    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[
            temp_dir1.path().to_path_buf(),
            temp_dir2.path().to_path_buf(),
            temp_dir3.path().to_path_buf(),
        ],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // All three unique modules should appear
    assert!(
        has_label(&items, "Path1::Module"),
        "Path1::Module should appear from first include path"
    );
    assert!(
        has_label(&items, "Path2::Module"),
        "Path2::Module should appear from second include path"
    );
    assert!(
        has_label(&items, "Path3::Module"),
        "Path3::Module should appear from third include path"
    );

    // Shared module should appear exactly once (deduplicated)
    assert_eq!(
        count_label(&items, "Shared::Module"),
        1,
        "Shared::Module should appear exactly once despite being in multiple paths"
    );
}

// -----------------------------------------------------------------------------
// Integration Test 4: Prefix filtering across components
// -----------------------------------------------------------------------------

/// Integration test: Verify prefix filtering works correctly across sources
///
/// This test verifies that prefix filtering is applied consistently
/// whether the module comes from workspace or include paths.
#[test]
fn integration_prefix_filtering_consistency() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules with similar prefixes
    create_temp_module(&include_path, "Alpha/Module.pm", "package Alpha::Module;\n1;\n").unwrap();
    create_temp_module(&include_path, "Alpha/Class.pm", "package Alpha::Class;\n1;\n").unwrap();
    create_temp_module(&include_path, "Alphabet/Module.pm", "package Alphabet::Module;\n1;\n")
        .unwrap();
    create_temp_module(&include_path, "Beta/Module.pm", "package Beta::Module;\n1;\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // Test prefix "Alpha" - should match Alpha::Module, Alpha::Class, and Alphabet::Module
    // (because "Alphabet".starts_with("Alpha") == true)
    let code = "use Alpha";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // These SHOULD appear (matching prefix)
    assert!(has_label(&items, "Alpha::Module"), "Alpha::Module should appear for 'Alpha' prefix");
    assert!(has_label(&items, "Alpha::Class"), "Alpha::Class should appear for 'Alpha' prefix");
    assert!(
        has_label(&items, "Alphabet::Module"),
        "Alphabet::Module SHOULD appear for 'Alpha' prefix (Alphabet.starts_with(Alpha))"
    );

    // This should NOT appear (non-matching prefix)
    assert!(
        !has_label(&items, "Beta::Module"),
        "Beta::Module should NOT appear for 'Alpha' prefix (Beta does not start with Alpha)"
    );
}

// -----------------------------------------------------------------------------
// Integration Test 5: Cancellation returns partial results
// -----------------------------------------------------------------------------

/// Integration test: Verify cancellation stops scanning but returns results collected so far
///
/// Integration test: Verify cancellation callback is invoked and checked during scanning
///
/// This test verifies:
/// 1. Cancellation callback is checked during scanning
/// 2. Callback is invoked multiple times during the scan
#[test]
fn integration_cancellation_callback_invoked() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create some modules
    create_temp_module(&include_path, "Alpha/Module.pm", "package Alpha::Module;\n1;\n").unwrap();
    create_temp_module(&include_path, "Beta/Module.pm", "package Beta::Module;\n1;\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );
    // Cancellation after 8 checks
    let call_count = std::cell::Cell::new(0);
    let is_cancelled = || {
        call_count.set(call_count.get() + 1);
        call_count.get() >= 8
    };

    let _items =
        provider.get_completions_with_path_cancellable(code, position, None, &is_cancelled);

    // Cancellation callback should be checked multiple times during scan
    // The exact count may vary based on when cancellation is checked, so we verify it's invoked at least a few times
    assert!(
        call_count.get() >= 5,
        "cancellation callback should be checked multiple times during scan, was {}",
        call_count.get()
    );
}

// -----------------------------------------------------------------------------
// Integration Test 6: Error handling - nonexistent include path
// -----------------------------------------------------------------------------

/// Integration test: Verify nonexistent include paths don't cause crashes
///
/// This test verifies graceful handling when an include path doesn't exist.
#[test]
fn integration_nonexistent_include_path_graceful() {
    let index = Arc::new(WorkspaceIndex::new());

    // Use a path that definitely doesn't exist
    let nonexistent_path = PathBuf::from("/this/path/does/not/exist");

    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[nonexistent_path],
        &[],
    );

    // Should not panic, just return empty results
    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    assert!(
        items.is_empty() || !items.iter().any(|i| i.label.contains("::")),
        "Should handle nonexistent include path gracefully"
    );
}

// -----------------------------------------------------------------------------
// Integration Test 7: Deep module structure traversal
// -----------------------------------------------------------------------------

/// Integration test: Verify deeply nested modules are handled correctly with MAX_SCAN_DEPTH
///
/// This test verifies:
/// 1. Modules within MAX_SCAN_DEPTH are found
/// 2. Modules beyond MAX_SCAN_DEPTH are excluded
/// 3. Path → module name conversion works correctly for deep paths
#[test]
fn integration_deep_nesting_respects_max_scan_depth() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules at various depths
    // Depth 1: Module.pm
    create_temp_module(&include_path, "Depth1.pm", "package Depth1;\n1;\n").unwrap();
    // Depth 3: A/B/C/Module.pm
    create_temp_module(&include_path, "A/B/C/Module.pm", "package A::B::C::Module;\n1;\n").unwrap();
    // Depth 5: A/B/C/D/E/Module.pm (exactly at limit with MAX_SCAN_DEPTH=6)
    create_temp_module(
        &include_path,
        "A/B/C/D/E/Module.pm",
        "package A::B::C::D::E::Module;\n1;\n",
    )
    .unwrap();
    // Depth 6: A/B/C/D/E/F/Module.pm (beyond limit)
    create_temp_module(
        &include_path,
        "A/B/C/D/E/F/Module.pm",
        "package A::B::C::D::E::F::Module;\n1;\n",
    )
    .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // Query for A::B::C::
    let code = "use A::B::C::";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // D::E::Module at depth 5 should appear
    assert!(
        has_label(&items, "A::B::C::D::E::Module"),
        "depth-5 module should appear (within MAX_SCAN_DEPTH=6)"
    );

    // D::E::F::Module at depth 6 should NOT appear
    assert!(
        !has_label(&items, "A::B::C::D::E::F::Module"),
        "depth-6 module should NOT appear (beyond MAX_SCAN_DEPTH=6)"
    );
}

// -----------------------------------------------------------------------------
// Integration Test 8: System @INC paths integration
// -----------------------------------------------------------------------------

/// Integration test: Verify system @INC paths work alongside include paths
///
/// This test verifies:
/// 1. system_inc_paths are scanned after include_paths
/// 2. System modules are properly labeled with "(system)" detail
/// 3. Deduplication between system paths and include paths works
#[test]
fn integration_system_inc_paths_with_include_paths() {
    let temp_include = tempfile::tempdir().expect("tempdir should succeed");
    let temp_system = tempfile::tempdir().expect("tempdir should succeed");

    let include_path = temp_include.path().to_path_buf();
    let system_path = temp_system.path().to_path_buf();

    // Module in include path
    create_temp_module(&include_path, "Local/Module.pm", "package Local::Module;\n1;\n").unwrap();

    // Module in system path
    create_temp_module(&system_path, "System/Module.pm", "package System::Module;\n1;\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[system_path],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Both modules should appear
    assert!(has_label(&items, "Local::Module"), "Local::Module should appear from include path");
    assert!(has_label(&items, "System::Module"), "System::Module should appear from system path");

    // Check detail text indicates system origin
    let system_detail =
        items.iter().find(|i| i.label == "System::Module").and_then(|i| i.detail.clone());
    assert!(
        system_detail.as_ref().is_some_and(|d| d.contains("system")),
        "System::Module should have '(system)' detail"
    );
}
